use std::sync::Arc;

use bb8::Pool;
use tokio::sync::mpsc;
use tokio::{net::TcpListener, sync::RwLock};
use tower::ServiceExt;

use crate::app_data::AppData;
use crate::client::ClientConnectionManager;
use crate::cluster::membership_protocol::ClusterProvider;
use crate::cluster::storage::MembersStorage;
use crate::object_placement::ObjectPlacementProvider;
use crate::registry::Registry;
use crate::service::Service;
use crate::ObjectId;

#[derive(Debug)]
pub enum AdminCommands {
    Shutdown(String, String),
}

pub type AdminReceiver = mpsc::UnboundedReceiver<AdminCommands>;
pub type AdminSender = mpsc::UnboundedSender<AdminCommands>;

pub struct Server<T>
where
    T: MembersStorage + 'static,
{
    address: String,
    registry: Arc<RwLock<Registry>>,
    membership_provider: Box<dyn ClusterProvider<T>>,
    object_placement_provider: Arc<RwLock<dyn ObjectPlacementProvider>>,
    app_data: Arc<AppData>,
}

impl<T> Server<T>
where
    T: MembersStorage + 'static,
{
    pub fn new(
        address: String,
        registry: Registry,
        membership_provider: impl ClusterProvider<T> + 'static,
        object_placement_provider: impl ObjectPlacementProvider + 'static,
    ) -> Server<T> {
        Server {
            address,
            registry: Arc::new(RwLock::new(registry)),
            membership_provider: Box::new(membership_provider),
            object_placement_provider: Arc::new(RwLock::new(object_placement_provider)),
            app_data: Arc::new(AppData::new()),
        }
    }

    pub fn app_data<Data>(&mut self, data: Data)
    where
        Data: Send + Sync + 'static,
    {
        self.app_data.set(data);
    }

    /// Run the server forever
    pub async fn run(&mut self) {
        let boxed_storage = dyn_clone::clone_box(self.membership_provider.members_storage());
        let pool_manager = ClientConnectionManager::new(boxed_storage);
        let client_pool = Pool::builder()
            .max_size(10)
            .build(pool_manager)
            .await
            .expect("TODO: client builder error");
        self.app_data(client_pool);

        let (admin_sender, admin_receiver) = mpsc::unbounded_channel::<AdminCommands>();
        self.app_data(admin_sender);

        tokio::select! {
            _ = self.accept() => {
                println!("serve finished first");
            }
            _ = self.membership_provider.serve(&self.address)  => {
                println!("membership serve finished first");
            }
            _ = self.consume_admin_commands(admin_receiver) => {
                println!("admin command serve finished first");
            }
        };
    }

    async fn accept(&self) {
        let listener = TcpListener::bind(&self.address)
            .await
            .expect("TODO: Failed to bind address");
        println!("Listening on: {}", self.address);
        loop {
            let (stream, _) = listener.accept().await.unwrap();
            let mut service: Service = self.into();
            service.ready().await.expect("TODO: Accept error");
            tokio::spawn(async move { service.run(stream).await });
        }
    }

    async fn consume_admin_commands(&self, mut admin_receiver: AdminReceiver) {
        while let Some(message) = admin_receiver.recv().await {
            match message {
                AdminCommands::Shutdown(object_kind, object_id) => {
                    println!("deleting {}.{}", object_kind, object_id);
                    let registry = self.registry.write().await;
                    registry.remove(object_kind.clone(), object_id.clone()).await;
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

impl<T> Into<Service> for &Server<T>
where
    T: MembersStorage,
{
    fn into(self) -> Service {
        let address = self.address.clone();
        let registry = self.registry.clone();
        let object_placement_provider = self.object_placement_provider.clone();
        let app_data = self.app_data.clone();
        let members_storage = dyn_clone::clone_box(self.membership_provider.members_storage());

        Service {
            address,
            registry,
            members_storage,
            object_placement_provider,
            app_data,
        }
    }
}
