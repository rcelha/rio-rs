//! Rio server

use std::convert::{TryFrom, TryInto};
use std::marker::PhantomData;
use std::net::SocketAddr;
use std::sync::Arc;

use bb8::Pool;
use derive_builder::Builder;
use tokio::sync::mpsc;
use tokio::{net::TcpListener, sync::RwLock};
use tower::ServiceExt;

use crate::app_data::AppData;
use crate::client::ClientConnectionManager;
use crate::cluster::membership_protocol::ClusterProvider;
use crate::cluster::storage::MembersStorage;
use crate::errors::{ServerBuilderError, ServerError};
use crate::object_placement::ObjectPlacementProvider;
use crate::protocol::pubsub::SubscriptionRequest;
use crate::protocol::RequestEnvelope;
use crate::registry::Registry;
use crate::service::Service;
use crate::ObjectId;

/// Internal commands, e.g., shutdown a service object
#[derive(Debug)]
pub enum AdminCommands {
    // Shutdown(hander_type, handler_id)
    Shutdown(String, String),
}

/// Channel for [AdminCommands]
pub type AdminReceiver = mpsc::UnboundedReceiver<AdminCommands>;

/// Channel for [AdminCommands]
pub type AdminSender = mpsc::UnboundedSender<AdminCommands>;

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
    S: MembersStorage + 'static,
    C: ClusterProvider<S>,
    P: ObjectPlacementProvider,
{
    /// Address given by the user
    #[builder(setter(into, strip_option), default = r#""0.0.0.0:0".to_string()"#)]
    address: String,

    /// TCP listener for the main server
    #[builder(setter(skip))]
    listener: Option<TcpListener>,

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
/// # use rio_rs::object_placement::local::LocalObjectPlacementProvider;
/// # use rio_rs::registry::Registry;
/// # use rio_rs::cluster::membership_protocol::local::LocalClusterProvider;
/// # use rio_rs::cluster::storage::local::LocalStorage;
/// # async fn run_server() {
/// #
/// let mut server = ServerBuilder::default()
///     .registry(Registry::default())
///     .cluster_provider(LocalClusterProvider {members_storage: LocalStorage::default()})
///     .object_placement_provider(LocalObjectPlacementProvider::default())
///     .client_pool_size(10)
///     .build().unwrap();
/// server.run().await;
/// #
/// # }
/// ```
pub struct ServerBuilder<S, C, P>
where
    S: MembersStorage,
    C: ClusterProvider<S>,
    P: ObjectPlacementProvider,
{
    address: String,
    registry: Option<Registry>,
    cluster_provider: Option<C>,
    object_placement_provider: Option<P>,
    client_pool_size: u32,

    _marker: PhantomData<S>,
}

impl<S, C, P> Default for ServerBuilder<S, C, P>
where
    S: MembersStorage,
    C: ClusterProvider<S>,
    P: ObjectPlacementProvider,
{
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
    S: MembersStorage,
    C: ClusterProvider<S> + 'static,
    P: ObjectPlacementProvider + 'static,
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
            .ok_or(ServerBuilderError::NoMembersStorage)?;
        let object_placement_provider = self
            .object_placement_provider
            .ok_or(ServerBuilderError::NoObjectPlacementProvider)?;
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
    S: MembersStorage + 'static,
    C: ClusterProvider<S>,
    P: ObjectPlacementProvider + 'static,
{
    pub fn new(
        address: String,
        registry: Registry,
        cluster_provider: C,
        object_placement_provider: P,
    ) -> Server<S, C, P> {
        Server {
            address,
            registry: Arc::new(RwLock::new(registry)),
            cluster_provider,
            object_placement_provider: Arc::new(RwLock::new(object_placement_provider)),
            app_data: Arc::new(AppData::new()),
            client_pool_size: 3,
            listener: None,
            _marker: PhantomData {},
        }
    }

    pub fn app_data<Data>(&mut self, data: Data)
    where
        Data: Send + Sync + 'static,
    {
        self.app_data.set(data);
    }

    /// Setup the server for running it
    pub async fn bind(&mut self) -> ServerResult<()> {
        let listener = TcpListener::bind(&self.address)
            .await
            .map_err(|err| ServerError::Bind(err.to_string()))?;
        self.listener.replace(listener);
        Ok(())
    }

    pub fn local_addr(&self) -> Option<tokio::io::Result<SocketAddr>> {
        self.listener.as_ref().map(|x| x.local_addr())
    }

    pub fn try_local_addr(&self) -> ServerResult<SocketAddr> {
        let bind_error = ServerError::Bind("Socket not bind".to_string());
        let maybe_addr = self.local_addr();
        let addr_result = maybe_addr.ok_or_else(|| bind_error)?;
        let addr = addr_result.map_err(|x| {
            let err = x.to_string();
            ServerError::Bind(err)
        })?;
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
    pub async fn run(&mut self) -> ServerResult<()> {
        let pool_manager =
            ClientConnectionManager::new(self.cluster_provider.members_storage().clone());
        let client_pool = Pool::builder()
            .max_size(self.client_pool_size)
            .build(pool_manager)
            .await
            .map_err(ServerError::ClientBuilder)?;
        self.app_data(client_pool);

        let (admin_sender, admin_receiver) = mpsc::unbounded_channel::<AdminCommands>();
        self.app_data(admin_sender);

        let local_addr = self.try_local_addr()?.to_string();

        tokio::select! {
            accept_result = self.accept() => {
                accept_result?;
            }
            cluster_provider_serve_result = self.cluster_provider.serve(&local_addr)  => {
                cluster_provider_serve_result.map_err(ServerError::ClusterProviderServe)?;
            }
            _ = self.consume_admin_commands(admin_receiver) => {
                println!("admin command serve finished first");
            }
        };
        Ok(())
    }

    async fn accept(&self) -> ServerResult<()> {
        let listener = self.listener.as_ref().ok_or(ServerError::Bind(
            "Socket not bind before accept connection".to_string(),
        ))?;
        println!("Listening on: {:?}", listener.local_addr());

        loop {
            let (stream, _) = listener.accept().await.map_err(|_| ServerError::Run)?;
            let mut service: Service<S, P> = self.try_into()?;

            ServiceExt::<RequestEnvelope>::ready(&mut service)
                .await
                .map_err(|_| ServerError::Run)?;
            ServiceExt::<SubscriptionRequest>::ready(&mut service)
                .await
                .map_err(|_| ServerError::Run)?;

            tokio::spawn(async move { service.run(stream).await });
        }
    }

    async fn consume_admin_commands(&self, mut admin_receiver: AdminReceiver) {
        while let Some(message) = admin_receiver.recv().await {
            match message {
                AdminCommands::Shutdown(object_kind, object_id) => {
                    println!("deleting {}.{}", object_kind, object_id);
                    let registry = self.registry.write().await;
                    registry
                        .remove(object_kind.clone(), object_id.clone())
                        .await;
                    self.object_placement_provider
                        .write()
                        .await
                        .remove(&ObjectId(object_kind, object_id))
                        .await;
                    println!("done deleting");
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
    S: MembersStorage + 'static,
    C: ClusterProvider<S>,
    P: ObjectPlacementProvider + 'static,
{
    type Error = ServerError;
    fn try_from(server: &Server<S, C, P>) -> Result<Self, Self::Error> {
        let address = server.try_local_addr()?.to_string();
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
    use crate::object_placement::local::LocalObjectPlacementProvider;
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
            .object_placement_provider(Arc::new(RwLock::new(
                LocalObjectPlacementProvider::default(),
            )))
            .build()
            .expect("Builder Failed");
    }
}
