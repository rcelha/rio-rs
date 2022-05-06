use std::sync::Arc;

use async_trait::async_trait;
use bb8::{Pool, RunError};
use serde::{de::DeserializeOwned, Deserialize, Serialize};

use crate::app_data::AppData;
use crate::client::ClientConnectionManager;
use crate::cluster::storage::MembersStorage;
use crate::errors::{ClientError, ServiceObjectLifeCycleError, HandlerError};
use crate::registry::{Handler, IdentifiableType, Message};
use crate::server::{AdminCommands, AdminSender};
use crate::state::ObjectStateManager;

pub struct ObjectId(pub String, pub String);

impl ObjectId {
    pub fn new(struct_name: impl Into<String>, object_id: impl Into<String>) -> ObjectId {
        ObjectId(struct_name.into(), object_id.into())
    }
}

pub trait FromId {
    fn from_id(id: String) -> Self;
    fn id(&self) -> &str;
}

#[async_trait]
pub trait ServiceObject: FromId + IdentifiableType + ObjectStateManager + ServiceObjectStateLoad {
    async fn send<S, T, V>(
        app_data: &AppData,
        handler_type_id: String,
        handler_id: String,
        payload: &V,
    ) -> Result<T, ClientError>
    where
        S: MembersStorage + 'static,
        T: DeserializeOwned,
        V: Serialize + IdentifiableType + Send + Sync,
    {
        let pool: &Pool<ClientConnectionManager> = app_data.get();
        match pool.get().await {
            Ok(mut client) => client.send(handler_type_id, handler_id, payload).await,
            Err(RunError::User(error)) => {
                println!("Aqui?");
                Err(error)
            }
            Err(e) => Err(ClientError::Unknown(e.to_string())),
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
        self.before_shutdown(&app_data).await?;
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
