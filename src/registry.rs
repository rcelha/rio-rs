use super::HandlerError;
use async_trait::async_trait;
use serde::{de::DeserializeOwned, Serialize};
use std::{
    any::{type_name, Any},
    collections::HashMap,
    future::Future,
    pin::Pin,
    sync::Arc,
};
use tokio::sync::RwLock;

type LockHashMap<K, V> = Arc<RwLock<HashMap<K, V>>>;
type BoxFuture<T> = Pin<Box<dyn Future<Output = T> + Send>>;
type AsyncRet = BoxFuture<Result<Vec<u8>, HandlerError>>;
type BoxedCallback = Box<dyn FnMut(&str, &str, &[u8]) -> AsyncRet + Send>;
type BoxedDefault = Box<dyn Fn() -> Box<dyn Any + Send + Sync> + Send + Sync>;

#[derive(Default)]
pub struct Registry {
    // (ObjectTypeName, ObjectId) -> Box<Obj>
    mapping: LockHashMap<(String, String), Box<dyn Any + Send + Sync>>,
    // (ObjectTypeName, MessageTypeName) -> Result<SerializedResult, Error>
    callable_mapping: HashMap<(String, String), BoxedCallback>,
    // (ObjectTypeName) -> Fn() -> Box<Object>
    builder_mapping: HashMap<String, BoxedDefault>,
}

// TODO remove this?
unsafe impl Send for Registry {}
unsafe impl Sync for Registry {}

impl Registry {
    pub fn new() -> Registry {
        Registry::default()
    }

    pub async fn add<T: 'static>(&mut self, k: String, v: T)
    where
        T: IdentifiableType + Send + Sync,
    {
        let type_id = T::user_defined_type_id().to_string();
        self.mapping.write().await.insert((type_id, k), Box::new(v));
    }

    pub fn add_builder<T: 'static>(&mut self)
    where
        T: IdentifiableType + Default + Send + Sync,
    {
        let k = T::user_defined_type_id().to_string();
        self.builder_mapping
            .insert(k, Box::new(move || Box::new(T::default())));
    }

    pub fn add_handler<T: 'static, M: 'static>(&mut self)
    where
        T: 'static + Handler<M> + IdentifiableType + Default + Send + Sync,
        M: 'static + IdentifiableType + Message + Send,
    {
        let mapping = self.mapping.clone();
        let type_id = T::user_defined_type_id().to_string();
        let message_type_id = M::user_defined_type_id().to_string();

        let callable = move |type_id: &str, object_id: &str, encoded_message: &[u8]| -> AsyncRet {
            let message: M = bincode::deserialize(encoded_message)
                .map_err(|_| HandlerError::Unknown)
                .unwrap();
            let object_key = (type_id.to_string(), object_id.to_string());
            let mapping_inner = mapping.clone();
            Box::pin(async move {
                let mut writabble_mapping = mapping_inner.write().await;
                let boxed_object = writabble_mapping.get_mut(&object_key).unwrap();
                let object: &mut T = boxed_object
                    .downcast_mut()
                    .ok_or(HandlerError::Unknown)
                    .unwrap(); // TODO ?;

                let ret = object.handle(message).await.unwrap(); //TODO ?;
                bincode::serialize(&ret).or(Err(HandlerError::ResponseSerializationError))
            })
        };
        let boxed_callable: BoxedCallback = Box::new(callable);
        self.callable_mapping
            .insert((type_id, message_type_id), boxed_callable);

        // TODO only do it once
        self.add_builder::<T>();
    }

    pub async fn send(
        &mut self,
        type_id: &str,
        object_id: &str,
        message_type_id: &str,
        message: &[u8],
    ) -> Result<Vec<u8>, HandlerError> {
        let callable_key = (type_id.to_string(), message_type_id.to_string());
        let callable = self
            .callable_mapping
            .get_mut(&callable_key)
            .ok_or(HandlerError::HandlerNotFound)?;

        println!("found callable {}/{}", type_id, message_type_id);
        callable(type_id, object_id, message).await
    }

    pub async fn has(&self, type_id: &str, object_id: &str) -> bool {
        let object_key = (type_id.to_string(), object_id.to_string());
        self.mapping.read().await.get(&object_key).is_some()
    }

    pub async fn insert_object(&mut self, type_id: String, object_id: String) {
        let default = self.builder_mapping.get(&type_id).unwrap();
        self.mapping
            .write()
            .await
            .insert((type_id, object_id), default());
    }
}

// TODO create a derive for this
// TODO deal with duplicates
pub trait IdentifiableType {
    fn user_defined_type_id() -> &'static str {
        type_name::<Self>()
    }
}

#[async_trait]
pub trait Handler<M>
where
    M: Message,
{
    type Returns: Serialize + Sync + Send;
    async fn handle(&mut self, message: M) -> Result<Self::Returns, HandlerError>;
}

pub trait Message: Serialize + DeserializeOwned {}

#[cfg(test)]
mod test {
    use super::*;
    use serde::Deserialize;

    #[derive(Default)]
    struct Human {}
    impl IdentifiableType for Human {
        fn user_defined_type_id() -> &'static str {
            "Human"
        }
    }

    #[derive(Serialize, Deserialize)]
    struct HiMessage {}
    impl IdentifiableType for HiMessage {
        fn user_defined_type_id() -> &'static str {
            "HiMessage"
        }
    }
    impl Message for HiMessage {}

    #[derive(Serialize, Deserialize)]
    struct GoodbyeMessage {}
    impl IdentifiableType for GoodbyeMessage {
        fn user_defined_type_id() -> &'static str {
            "GoodbyeMessage"
        }
    }
    impl Message for GoodbyeMessage {}

    #[async_trait]
    impl Handler<HiMessage> for Human {
        type Returns = String;
        async fn handle(&mut self, _message: HiMessage) -> Result<String, HandlerError> {
            Ok("hi".to_string())
        }
    }

    #[async_trait]
    impl Handler<GoodbyeMessage> for Human {
        type Returns = String;
        async fn handle(&mut self, _message: GoodbyeMessage) -> Result<String, HandlerError> {
            Ok("bye".to_string())
        }
    }

    #[tokio::test]
    async fn sanity_check() {
        let mut registry = Registry::new();
        let obj = Human {};
        registry.add("john".to_string(), obj).await;
        registry.add_handler::<Human, HiMessage>();
        registry.add_handler::<Human, GoodbyeMessage>();
        registry
            .send(
                "Human",
                "john",
                "HiMessage",
                &bincode::serialize(&HiMessage {}).unwrap(),
            )
            .await
            .unwrap();

        registry
            .send(
                "Human",
                "john",
                "GoodbyeMessage",
                &bincode::serialize(&GoodbyeMessage {}).unwrap(),
            )
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn test_return() {
        let mut registry = Registry::new();
        let obj = Human {};
        registry.add("john".to_string(), obj).await;
        registry.add_handler::<Human, HiMessage>();
        let ret = registry
            .send(
                "Human",
                "john",
                "HiMessage",
                &bincode::serialize(&HiMessage {}).unwrap(),
            )
            .await;
        let result: String = bincode::deserialize(&ret.unwrap()).unwrap();
        assert_eq!(result, "hi")
    }

    #[tokio::test]
    async fn test_not_registered_message() {
        let mut registry = Registry::new();
        let obj = Human {};
        registry.add("john".to_string(), obj).await;
        let ret = registry
            .send(
                "Human",
                "john",
                "HiMessage",
                &bincode::serialize(&HiMessage {}).unwrap(),
            )
            .await;
        assert!(ret.is_err());
    }

    #[tokio::test]
    async fn test_send_sync() {
        let join_handler = tokio::spawn(async move {
            let mut registry = Registry::new();
            registry.add_handler::<Human, HiMessage>();
            let obj = Human {};
            registry.add("john".to_string(), obj).await;
            registry
                .send(
                    "Human",
                    "john",
                    "HiMessage",
                    &bincode::serialize(&HiMessage {}).unwrap(),
                )
                .await
                .unwrap();
            tokio::time::sleep(std::time::Duration::from_micros(1)).await;
        });
        join_handler.await.unwrap();
    }

    #[tokio::test]
    async fn test_send_sync_lock() {
        let mut registry = Registry::new();
        registry.add_handler::<Human, HiMessage>();
        let obj = Human {};
        registry.add("john".to_string(), obj).await;
        let registry = Arc::new(RwLock::new(registry));
        let inner_registry = Arc::clone(&registry);

        let join_handler = tokio::spawn(async move {
            inner_registry
                .write()
                .await
                .send(
                    "Human",
                    "john",
                    "HiMessage",
                    &bincode::serialize(&HiMessage {}).unwrap(),
                )
                .await
                .unwrap();
            tokio::time::sleep(std::time::Duration::from_micros(1)).await;
        });
        join_handler.await.unwrap();
    }

    #[tokio::test]
    async fn test_has_object() {
        let mut registry = Registry::new();
        let obj = Human {};
        registry.add("john".to_string(), obj).await;
        assert!(registry.has("Human", "john").await);
        assert!(!registry.has("Human", "not john").await);
    }

    #[tokio::test]
    async fn test_dynamic_insert_object() {
        let mut registry = Registry::new();
        registry.add_builder::<Human>();
        assert!(!registry.has("Human", "john").await);
        registry
            .insert_object("Human".to_string(), "john".to_string())
            .await;
        assert!(registry.has("Human", "john").await);
    }

    #[tokio::test]
    async fn test_automatic_dynamic_insert_object() {
        let mut registry = Registry::new();
        registry.add_handler::<Human, HiMessage>();
        assert!(!registry.has("Human", "john").await);
        registry
            .insert_object("Human".to_string(), "john".to_string())
            .await;
        assert!(registry.has("Human", "john").await);
    }
}
