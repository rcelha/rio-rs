use std::sync::Arc;

use async_trait::async_trait;
use bb8::{Pool, RunError};
use serde::{de::DeserializeOwned, Deserialize, Serialize};

use crate::app_data::AppData;
use crate::client::ClientConnectionManager;
use crate::cluster::storage::MembersStorage;
use crate::errors::{HandlerError, ServiceObjectLifeCycleError};
use crate::protocol::ClientError;
use crate::registry::{Handler, IdentifiableType, Message};
use crate::server::{AdminCommands, AdminSender};
use crate::state::ObjectStateManager;

/// TODO docs
pub struct ObjectId(pub String, pub String);

impl ObjectId {
    pub fn new(struct_name: impl Into<String>, object_id: impl Into<String>) -> ObjectId {
        ObjectId(struct_name.into(), object_id.into())
    }
}

/// TODO docs
pub trait WithId {
    fn set_id(&mut self, id: String);
    fn id(&self) -> &str;
}

/// ServiceObjects are the objects that will respond to various types of messages through
/// the Rio Server
///
/// The server stores each ServiceObject onto a registry and control their life cycle
///
/// There are a few requirements in oder to implement a ServiceObject:
///     - Default
///     - WithId
///     - IdentifiableType
///     - ObjectStateManager
///     - ServiceObjectStateLoad
/// TODO docs
#[async_trait]
pub trait ServiceObject:
    Default + WithId + IdentifiableType + ObjectStateManager + ServiceObjectStateLoad
{
    /// Send a message to Rio cluster using a client tht is stored in AppData
    async fn send<S, T, V, H, I>(
        app_data: &AppData,
        handler_type_id: &H,
        handler_id: &I,
        payload: &V,
    ) -> Result<T, ClientError>
    where
        S: MembersStorage + 'static,
        T: DeserializeOwned,
        V: Serialize + IdentifiableType + Send + Sync,
        H: AsRef<str> + Send + Sync,
        I: AsRef<str> + Send + Sync,
    {
        let pool: &Pool<ClientConnectionManager<S>> = app_data.get();
        match pool.get().await {
            Ok(mut client) => client.send(handler_type_id, handler_id, payload).await,
            Err(RunError::User(error)) => Err(error),
            // TODO: might want a time out error in ClientError
            Err(RunError::TimedOut) => Err(ClientError::Connectivity),
        }
    }

    async fn before_load(&mut self, _: &AppData) -> Result<(), ServiceObjectLifeCycleError> {
        Ok(())
    }

    async fn after_load(&mut self, _: &AppData) -> Result<(), ServiceObjectLifeCycleError> {
        Ok(())
    }

    async fn before_shutdown(&mut self, _: &AppData) -> Result<(), ServiceObjectLifeCycleError> {
        Ok(())
    }

    async fn shutdown(&mut self, app_data: &AppData) -> Result<(), ServiceObjectLifeCycleError> {
        self.before_shutdown(app_data).await?;
        let admin_sender = app_data.get::<AdminSender>().clone();
        admin_sender
            .send(AdminCommands::Shutdown(
                Self::user_defined_type_id().to_string(),
                self.id().to_string(),
            ))
            .expect("TODO");
        Ok(())
    }
}

/// Load all states for a into a ServiceObject
#[async_trait]
pub trait ServiceObjectStateLoad {
    async fn load(&mut self, _: &AppData) -> Result<(), ServiceObjectLifeCycleError> {
        Ok(())
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub enum LifecycleMessage {
    Load,
}

impl Message for LifecycleMessage {}

impl IdentifiableType for LifecycleMessage {
    fn user_defined_type_id() -> &'static str {
        "LifecycleMessage"
    }
}

#[async_trait]
impl<T> Handler<LifecycleMessage> for T
where
    T: ServiceObject + Send + Sync,
{
    type Returns = ();
    async fn handle(
        &mut self,
        message: LifecycleMessage,
        context: Arc<AppData>,
    ) -> Result<Self::Returns, crate::errors::HandlerError> {
        match message {
            LifecycleMessage::Load => {
                self.before_load(&context)
                    .await
                    .map_err(HandlerError::LyfecycleError)?;
                self.load(&context)
                    .await
                    .map_err(HandlerError::LyfecycleError)?;
                self.after_load(&context)
                    .await
                    .map_err(HandlerError::LyfecycleError)?;

                Ok(())
            }
        }
    }
}
