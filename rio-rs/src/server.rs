use std::marker::PhantomData;
use std::sync::Arc;

use bb8::Pool;
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
pub struct Server<S, C, P>
where
    S: MembersStorage + 'static,
    C: ClusterProvider<S>,
    P: ObjectPlacementProvider,
{
    address: String,
    registry: Arc<RwLock<Registry>>,
    cluster_provider: C,
    object_placement_provider: Arc<RwLock<P>>,
    app_data: Arc<AppData>,

    client_pool_size: u32,

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
/// # use rio_rs::cluster::storage::LocalStorage;
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
            _marker: PhantomData {},
        }
    }

    pub fn app_data<Data>(&mut self, data: Data)
    where
        Data: Send + Sync + 'static,
    {
        self.app_data.set(data);
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

        tokio::select! {
            accept_result = self.accept() => {
                accept_result?;
            }
            cluster_provider_serve_result = self.cluster_provider.serve(&self.address)  => {
                cluster_provider_serve_result.map_err(ServerError::ClusterProviderServe)?;
            }
            _ = self.consume_admin_commands(admin_receiver) => {
                println!("admin command serve finished first");
            }
        };
        Ok(())
    }

    async fn accept(&self) -> ServerResult<()> {
        let listener = TcpListener::bind(&self.address)
            .await
            .map_err(|err| ServerError::Bind(err.to_string()))?;

        let bind = listener.local_addr().expect("TODO");
        println!("Listening on: {:}", bind);

        loop {
            let (stream, _) = listener.accept().await.map_err(|_| ServerError::Run)?;
            let mut service: Service<S, P> = self.into();

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

impl<S: MembersStorage, C: ClusterProvider<S>, P: ObjectPlacementProvider> From<&Server<S, C, P>>
    for Service<S, P>
{
    fn from(server: &Server<S, C, P>) -> Self {
        let address = server.address.clone();
        let registry = server.registry.clone();
        let object_placement_provider = server.object_placement_provider.clone();
        let app_data = server.app_data.clone();
        let members_storage = server.cluster_provider.members_storage().clone();

        Service {
            address,
            registry,
            members_storage,
            object_placement_provider,
            app_data,
        }
    }
}
