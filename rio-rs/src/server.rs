//! Rio server

use std::convert::TryFrom;
use std::marker::PhantomData;
use std::net::SocketAddr;
use std::sync::Arc;

use derive_builder::Builder;
use log::{error, info, warn};
use netwatch::ip::LocalAddresses;
use tokio::sync::mpsc;
use tokio::task::JoinSet;
use tokio::{net::TcpListener, sync::RwLock};
use tower::{Service as TowerService, ServiceExt};

use crate::app_data::AppData;
use crate::cluster::membership_protocol::ClusterProvider;
use crate::cluster::storage::MembershipStorage;
use crate::errors::{ServerBuilderError, ServerError};
use crate::object_placement::ObjectPlacement;
use crate::protocol::pubsub::SubscriptionRequest;
use crate::protocol::ResponseError;
use crate::protocol::{RequestEnvelope, ResponseEnvelope};
use crate::registry::Registry;
use crate::service::Service;
use crate::ObjectId;

/// Internal commands, e.g., shutdown a service object
#[derive(Debug)]
pub enum AdminCommands {
    ServerExit,
    // Shutdown(hander_type, handler_id)
    Shutdown(String, String),
}

/// Channel for [AdminCommands]
pub type AdminReceiver = mpsc::UnboundedReceiver<AdminCommands>;

/// Channel for [AdminCommands]
pub type AdminSender = mpsc::UnboundedSender<AdminCommands>;

/// Result for the internal client interface on the server
pub type SendCommandResult = Result<Vec<u8>, ResponseError>;

/// Request struct for the internal client interface on the server
/// TODO pub?
#[derive(Debug)]
pub struct SendCommand {
    pub request: RequestEnvelope,
    pub response_channel: tokio::sync::oneshot::Sender<SendCommandResult>,
}

impl SendCommand {
    pub fn build(
        request: RequestEnvelope,
    ) -> (
        SendCommand,
        tokio::sync::oneshot::Receiver<SendCommandResult>,
    ) {
        let (sender, receiver) = tokio::sync::oneshot::channel();
        let command = SendCommand {
            request,
            response_channel: sender,
        };
        (command, receiver)
    }
}

/// Channels for internal client
pub type InternalClientReceiver = mpsc::UnboundedReceiver<SendCommand>;

/// Channels for internal client
pub type InternalClientSender = mpsc::UnboundedSender<SendCommand>;

/// Application Server. It handles object registration ([Registry]),
/// clustering (through [ClusterProvider]s), server state (via [AppData]),
/// and more.
///
/// It handles various types of request: [AdminCommands], [RequestEnvelope], and
/// [SubscriptionRequest].
///
/// More of it can be seen in [Server::run].
#[derive(Builder)]
#[builder(name = "NewServerBuilder")]
pub struct Server<S, C, P>
where
    S: MembershipStorage,
    C: ClusterProvider<S>,
    P: ObjectPlacement,
{
    /// Address given by the user
    #[builder(setter(into, strip_option), default = r#""0.0.0.0:0".to_string()"#)]
    address: String,

    /// Address given by the user
    #[builder(
        setter(into, strip_option),
        default = r#"Some("0.0.0.0:0".to_string())"#
    )]
    #[cfg(feature = "http")]
    http_members_storage_address: Option<String>,

    registry: Arc<RwLock<Registry>>,
    cluster_provider: C,
    object_placement_provider: Arc<RwLock<P>>,
    app_data: Arc<AppData>,

    #[builder(default = "10")]
    client_pool_size: u32,

    #[builder(default = "PhantomData {}", setter(skip))]
    _marker: PhantomData<S>,
}

/// Builder pattern for [Server]
///
/// # Example
/// ```rust
/// # use rio_rs::server::ServerBuilder;
/// # use rio_rs::object_placement::local::LocalObjectPlacement;
/// # use rio_rs::registry::Registry;
/// # use rio_rs::cluster::membership_protocol::local::LocalClusterProvider;
/// # use rio_rs::cluster::storage::local::LocalStorage;
/// # async fn run_server() {
/// #
/// let mut server = ServerBuilder::default()
///     .registry(Registry::default())
///     .cluster_provider(LocalClusterProvider {members_storage: LocalStorage::default()})
///     .object_placement_provider(LocalObjectPlacement::default())
///     .client_pool_size(10)
///     .build().unwrap();
/// let listener = server.bind().await.unwrap();
/// server.run(listener).await;
/// #
/// # }
/// ```
pub struct ServerBuilder<S, C, P> {
    address: String,
    registry: Option<Registry>,
    cluster_provider: Option<C>,
    object_placement_provider: Option<P>,
    client_pool_size: u32,

    _marker: PhantomData<S>,
}

impl<S, C, P> Default for ServerBuilder<S, C, P> {
    fn default() -> Self {
        ServerBuilder {
            address: "0.0.0.0:5000".to_string(),
            registry: None,
            cluster_provider: None,
            object_placement_provider: None,
            client_pool_size: 3,
            _marker: PhantomData {},
        }
    }
}

impl<S, C, P> ServerBuilder<S, C, P>
where
    S: MembershipStorage + 'static,
    C: ClusterProvider<S> + 'static + Send + Sync,
    P: ObjectPlacement + 'static,
{
    pub fn new() -> Self {
        Self::default()
    }

    pub fn address(mut self, value: String) -> Self {
        self.address = value;
        self
    }

    pub fn client_pool_size(mut self, value: u32) -> Self {
        self.client_pool_size = value;
        self
    }

    pub fn registry(mut self, value: Registry) -> Self {
        self.registry = Some(value);
        self
    }

    pub fn cluster_provider(mut self, value: C) -> Self {
        self.cluster_provider = Some(value);
        self
    }

    pub fn object_placement_provider(mut self, value: P) -> Self {
        self.object_placement_provider = Some(value);
        self
    }

    pub fn build(self) -> Result<Server<S, C, P>, ServerBuilderError> {
        let address = self.address;
        let registry = self.registry.unwrap_or_default();
        let cluster_provider = self
            .cluster_provider
            .ok_or(ServerBuilderError::NoMembershipStorage)?;
        let object_placement_provider = self
            .object_placement_provider
            .ok_or(ServerBuilderError::NoObjectPlacement)?;
        let client_pool_size = self.client_pool_size;

        let mut server = Server::new(
            address,
            registry,
            cluster_provider,
            object_placement_provider,
        );
        server.client_pool_size = client_pool_size;

        Ok(server)
    }
}

type ServerResult<T> = Result<T, ServerError>;

impl<S, C, P> Server<S, C, P>
where
    S: MembershipStorage + 'static,
    C: ClusterProvider<S> + Send + Sync + 'static,
    P: ObjectPlacement + 'static,
{
    pub fn new(
        address: String,
        registry: Registry,
        cluster_provider: C,
        object_placement_provider: P,
    ) -> Server<S, C, P> {
        Server {
            address,
            #[cfg(feature = "http")]
            http_members_storage_address: None,
            registry: Arc::new(RwLock::new(registry)),
            cluster_provider,
            object_placement_provider: Arc::new(RwLock::new(object_placement_provider)),
            app_data: Arc::new(AppData::new()),
            client_pool_size: 3,
            _marker: PhantomData {},
        }
    }

    pub async fn prepare(&self) {
        self.cluster_provider.members_storage().prepare().await;
        let object_placement_provider_guard = self.object_placement_provider.read().await;
        object_placement_provider_guard.prepare().await;
    }

    pub fn app_data<Data>(&mut self, data: Data)
    where
        Data: Send + Sync + 'static,
    {
        self.app_data.set(data);
    }

    /// Setup the server for running it
    pub async fn bind(&mut self) -> ServerResult<TcpListener> {
        let listener = TcpListener::bind(&self.address)
            .await
            .map_err(|err| ServerError::Bind(err.to_string()))?;
        Ok(listener)
    }

    /// Tries to get a local address
    ///
    /// It will get the first ip address for the machine where it is running,
    /// and fallback to the address given by tokio's listener
    ///
    /// <div class="warning">
    /// **TODO**
    ///
    /// It potentially won't work on machine with multiple interfaces. So we need to
    /// add support to address mapping per node before merging this.
    ///
    /// For now, if that is your case, you need to specify the IP for binding
    /// </div>
    pub fn try_local_addr(listener: &TcpListener) -> ServerResult<SocketAddr> {
        let addr_result = listener.local_addr();
        let mut addr = addr_result.map_err(|x| {
            let err = x.to_string();
            ServerError::Bind(err)
        })?;

        // Try to update the local address using netwatch's LocalAddress
        let nw_local_addr = LocalAddresses::new();
        if let Some(first_local_address) = nw_local_addr.regular.first() {
            addr.set_ip(*first_local_address);
        }
        Ok(addr)
    }

    /// Run the server forever
    ///
    /// This is the main loop for a Rio server. It will handle a few types of future concurrently:
    /// - New TCP connections from clients
    /// - [AdminCommands] messages from running objects
    /// - [ClusterProvider] server loop
    ///
    /// If any of these fails, the server stops running with a [ServerError]
    pub async fn run(&mut self, listener: TcpListener) -> ServerResult<()> {
        let (admin_sender, admin_receiver) = mpsc::unbounded_channel::<AdminCommands>();
        self.app_data(admin_sender);

        let (internal_client_sender, internal_client_receiver) =
            mpsc::unbounded_channel::<SendCommand>();
        self.app_data(internal_client_sender);

        let local_addr = Self::try_local_addr(&listener)?.to_string();

        let mut service = Service::<S, P>::try_from(&*self)?;
        service.address = local_addr.clone();
        let mut accept_task = tokio::spawn(Self::accept(listener, service));

        let cluster_provider = self.cluster_provider.clone();
        let inner_local_addr = local_addr.clone();
        let mut cluster_provider_task =
            tokio::spawn(async move { cluster_provider.serve(&inner_local_addr).await });

        let mut service = Service::<S, P>::try_from(&*self)?;
        service.address = local_addr.clone();
        let mut internal_client_task = tokio::spawn(async move {
            Self::consume_internal_client_commands(internal_client_receiver, service).await
        });

        let admin_commands_fut = self.consume_admin_commands(admin_receiver);

        #[cfg(feature = "http")]
        let mut cluster_storage_http_server_task =
            if let Some(addr) = self.http_members_storage_address.clone() {
                let inner_members_storage = self.cluster_provider.members_storage().clone();
                tokio::spawn(async move {
                    crate::cluster::storage::http::serve(addr, inner_members_storage)
                        .await
                        .ok();
                })
            } else {
                tokio::spawn(async move {
                    warn!("HTTP Members Storage not enabled");
                    loop {
                        tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
                    }
                })
            };

        #[cfg(not(feature = "http"))]
        let mut cluster_storage_http_server_task = tokio::spawn(async move {
            warn!("HTTP Members Storage not enabled");
            loop {
                tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
            }
        });

        tokio::select! {
            accept_result = &mut accept_task => {
                accept_result
                    .map_err(|err| {
                        error!(
                            "accept: JoinHandle error: {}",
                            err.to_string()
                        );
                        ServerError::Run
                    })??;
            }
            cluster_provider_serve_result = &mut cluster_provider_task => {
                cluster_provider_serve_result
                    .map_err(|err| {
                        error!(
                            "cluster provider server: JoinHandle error: {}",
                            err.to_string()
                        );
                        ServerError::Run
                    })?
                .map_err(ServerError::ClusterProviderServe)?;
                warn!("Cluster provider has finished");
            }
            internal_client_result = &mut internal_client_task => {
                internal_client_result
                    .map_err(|err| {
                        error!(
                            "internal client: JoinHandle error: {}",
                            err.to_string()
                        );
                        ServerError::Run
                    })??;
                warn!("Internal client consumer finished first");
            }
            _ = admin_commands_fut => {
                warn!("Admin command serve finished first");
            }
            _ = &mut cluster_storage_http_server_task => {
                warn!("Http Server for Cluster Storage finished earlier");
            }

        }

        info!("Stoping server");
        accept_task.abort();
        cluster_provider_task.abort();
        internal_client_task.abort();
        cluster_storage_http_server_task.abort();
        info!("Server stopped");

        Ok(())
    }

    async fn accept(listener: TcpListener, service: Service<S, P>) -> ServerResult<()> {
        let local_addr = listener.local_addr().map_err(|_| {
            ServerError::Bind("Cannot get the local address for the listener".to_string())
        })?;
        info!("Listening on `{}`", local_addr.to_string());
        let mut joinset = JoinSet::new();

        loop {
            let (stream, _) = listener.accept().await.map_err(|_| ServerError::Run)?;
            let mut service: Service<S, P> = service.clone();

            ServiceExt::<RequestEnvelope>::ready(&mut service)
                .await
                .map_err(|_| ServerError::Run)?;
            ServiceExt::<SubscriptionRequest>::ready(&mut service)
                .await
                .map_err(|_| ServerError::Run)?;

            joinset.spawn(async move { service.run(stream).await });
        }
    }

    /// Consumes messages coming from a mpsc channel and make the bridge to
    /// the server service (need better naming here) to call another object service
    async fn consume_internal_client_commands(
        mut receiver: InternalClientReceiver,
        service: Service<S, P>,
    ) -> ServerResult<()> {
        let mut joinset = JoinSet::new();
        while let Some(message) = receiver.recv().await {
            let mut inner_service = service.clone();
            joinset.spawn(async move {
                let resp = inner_service
                    .call(message.request)
                    .await
                    .unwrap_or_else(ResponseEnvelope::err);
                message
                    .response_channel
                    .send(resp.body)
                    .inspect_err(|_| {
                        error!("The caller dropped");
                    })
                    .ok();
            });
        }
        let _ = joinset.join_all().await;
        Ok(())
    }

    /// Receives the messages from the AdminReceiver channel
    ///
    /// These are operations that need to be protected from external access, and only
    /// ServiceObjects have access to this channel - the channel is in the AppData
    async fn consume_admin_commands(&self, mut admin_receiver: AdminReceiver) {
        while let Some(message) = admin_receiver.recv().await {
            match message {
                // TODO I think this only works for the current server,
                //      meaning, if the service object is running on another
                //      node, it won't shutdown
                AdminCommands::Shutdown(object_kind, object_id) => {
                    let registry = self.registry.write().await;
                    registry
                        .remove(object_kind.clone(), object_id.clone())
                        .await;
                    self.object_placement_provider
                        .write()
                        .await
                        .remove(&ObjectId(object_kind, object_id))
                        .await;
                }
                AdminCommands::ServerExit => {
                    // Exists `while` to terminate the server
                    println!("I got a message to terminate this thing here. So Ill try");
                    return;
                }
            }
        }
    }
}

/// Transforms a [Server] into a [Service]
///
/// It can't be infalible, because it needs to be bind
/// so it can generate a Service
impl<S, C, P> TryFrom<&Server<S, C, P>> for Service<S, P>
where
    S: MembershipStorage + 'static,
    C: ClusterProvider<S> + 'static + Send + Sync,
    P: ObjectPlacement + 'static,
{
    type Error = ServerError;
    fn try_from(server: &Server<S, C, P>) -> Result<Self, Self::Error> {
        let address = "".to_string();
        let registry = server.registry.clone();
        let object_placement_provider = server.object_placement_provider.clone();
        let app_data = server.app_data.clone();
        let members_storage = server.cluster_provider.members_storage().clone();

        Ok(Service {
            address,
            registry,
            members_storage,
            object_placement_provider,
            app_data,
        })
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::cluster::membership_protocol::local::LocalClusterProvider;
    use crate::cluster::storage::local::LocalStorage;
    use crate::object_placement::local::LocalObjectPlacement;
    use crate::registry::Registry;

    #[tokio::test]
    async fn client_builder_sanity_check() {
        let _server = NewServerBuilder::default()
            .address("0.0.0.0:80")
            .registry(Arc::new(RwLock::new(Registry::default())))
            .app_data(Arc::new(Default::default()))
            .cluster_provider(LocalClusterProvider {
                members_storage: LocalStorage::default(),
            })
            .object_placement_provider(Arc::new(RwLock::new(LocalObjectPlacement::default())))
            .build()
            .expect("Builder Failed");
    }
}
