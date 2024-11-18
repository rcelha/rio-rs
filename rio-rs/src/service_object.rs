//! Module for implementing a Rio service

use std::sync::Arc;

use async_trait::async_trait;
use serde::{de::DeserializeOwned, Deserialize, Serialize};

use crate::app_data::AppData;
use crate::errors::{HandlerError, ServiceObjectLifeCycleError};
use crate::protocol::{ClientError, RequestEnvelope, RequestError};
use crate::registry::{Handler, IdentifiableType, Message};
use crate::server::{AdminCommands, AdminSender, InternalClientSender, SendCommand};

/// Internal representation of an object id.
///
/// It is stuct name + the object id (as in [WithId]).
/// It is used lookups across tthis project
#[derive(Debug)]
pub struct ObjectId(pub String, pub String);

impl ObjectId {
    pub fn new(struct_name: impl Into<String>, object_id: impl Into<String>) -> ObjectId {
        ObjectId(struct_name.into(), object_id.into())
    }
}

/// Common interface to get a string Id for an object
///
/// This is particularly useful for the registry, as every object
/// in the registry needs to have an Id for retrieval
// TODO move it out of the service_object module
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
#[async_trait]
pub trait ServiceObject: Default + WithId + IdentifiableType {
    /// Send a message to Rio cluster using a client tht is stored in AppData
    async fn send<T, V>(
        app_data: &AppData,
        handler_type_id: impl ToString + Send + Sync,
        handler_id: impl ToString + Send + Sync,
        payload: &V,
    ) -> Result<T, RequestError>
    where
        T: DeserializeOwned + Send + Sync,
        V: Serialize + IdentifiableType + Send + Sync,
    {
        let client = app_data.get::<InternalClientSender>();
        let payload = bincode::serialize(&payload).expect("TODO");
        let request = RequestEnvelope::new(
            handler_type_id.to_string(),
            handler_id.to_string(),
            V::user_defined_type_id().to_string(),
            payload,
        );
        let (request_message, channel) = SendCommand::build(request);
        client
            .send(request_message)
            .map_err(|e| ClientError::IoError(e.to_string()))?;

        let resp = channel
            .await
            .map_err(|e| ClientError::IoError(e.to_string()))??;

        let parsed_body = bincode::deserialize::<T>(&resp)
            .map_err(|e| ClientError::DeseralizationError(e.to_string()))?;
        Ok(parsed_body)
    }

    async fn before_load(&mut self, _: Arc<AppData>) -> Result<(), ServiceObjectLifeCycleError> {
        Ok(())
    }

    async fn after_load(&mut self, _: Arc<AppData>) -> Result<(), ServiceObjectLifeCycleError> {
        Ok(())
    }

    async fn before_shutdown(
        &mut self,
        _: Arc<AppData>,
    ) -> Result<(), ServiceObjectLifeCycleError> {
        Ok(())
    }

    async fn shutdown(
        &mut self,
        app_data: Arc<AppData>,
    ) -> Result<(), ServiceObjectLifeCycleError> {
        self.before_shutdown(app_data.clone()).await?;
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

/// [Message] that is sent to the object when it reaches specific
/// parts of its life cycle
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
    T: ServiceObject + ServiceObjectStateLoad + Send + Sync,
{
    type Returns = ();
    async fn handle(
        &mut self,
        message: LifecycleMessage,
        context: Arc<AppData>,
    ) -> Result<Self::Returns, crate::errors::HandlerError> {
        match message {
            LifecycleMessage::Load => {
                self.before_load(context.clone())
                    .await
                    .map_err(HandlerError::LyfecycleError)?;
                self.load(context.clone().as_ref())
                    .await
                    .map_err(HandlerError::LyfecycleError)?;
                self.after_load(context.clone())
                    .await
                    .map_err(HandlerError::LyfecycleError)?;

                Ok(())
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[tokio::test]
    async fn asd() {
        #[derive(Default, Serialize, Deserialize)]
        struct DummyMessage {}

        impl IdentifiableType for DummyMessage {
            fn user_defined_type_id() -> &'static str {
                "DummyMessage"
            }
        }

        #[derive(Default)]
        struct DummyService {}

        impl ServiceObject for DummyService {}

        impl WithId for DummyService {
            fn id(&self) -> &str {
                ""
            }
            fn set_id(&mut self, _: String) {}
        }

        impl IdentifiableType for DummyService {
            fn user_defined_type_id() -> &'static str {
                "DummyService"
            }
        }

        impl DummyService {
            #[allow(unused)]
            async fn test(&self, app_data: &AppData, handler_type_id: String, handler_id: String) {
                let payload = DummyMessage::default();
                let _: Result<DummyMessage, _> =
                    Self::send(app_data, handler_type_id, handler_id, &payload).await;
            }
        }
    }
}
