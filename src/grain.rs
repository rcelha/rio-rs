use std::sync::Arc;

use async_trait::async_trait;
use bb8::{Pool, RunError};
use serde::{de::DeserializeOwned, Deserialize, Serialize};

use crate::{
    app_data::AppData,
    client::ClientConnectionManager,
    errors::{ClientError, GrainLifeCycleError, HandlerError},
    membership_provider::MembersStorage,
    registry::{Handler, IdentifiableType, Message},
};

pub struct GrainId(pub String, pub String);

impl GrainId {
    pub fn new(struct_name: impl Into<String>, object_id: impl Into<String>) -> GrainId {
        GrainId(struct_name.into(), object_id.into())
    }
}

pub trait FromId {
    fn from_id(id: String) -> Self;
    fn id(&self) -> &str;
}

#[async_trait]
pub trait Grain: FromId + IdentifiableType {
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
            Err(RunError::User(error)) => Err(error),
            Err(e) => Err(ClientError::Unknown(e.to_string())),
        }
    }

    async fn before_load(&mut self, _: &AppData) -> Result<(), GrainLifeCycleError> {
        Ok(())
    }

    async fn after_load(&mut self, _: &AppData) -> Result<(), GrainLifeCycleError> {
        Ok(())
    }

    async fn load(&mut self, context: &AppData) -> Result<(), GrainLifeCycleError> {
        self.before_load(context).await?;
        let ret = Ok(());
        self.after_load(context).await?;
        ret
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
    T: Grain + Send + Sync,
{
    type Returns = ();
    async fn handle(
        &mut self,
        message: LifecycleMessage,
        context: Arc<AppData>,
    ) -> Result<Self::Returns, crate::errors::HandlerError> {
        match message {
            LifecycleMessage::Load => self
                .load(&context)
                .await
                .map_err(HandlerError::LyfecycleError),
        }
    }
}
