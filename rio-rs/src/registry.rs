//! Trait object registry
//!
//! Provides storage for objects and maps their callables to handle registered message types

use crate::{app_data::AppData, errors::HandlerError, WithId};
use async_trait::async_trait;
use dashmap::DashMap;
use serde::{de::DeserializeOwned, Serialize};
use std::{
    any::{type_name, Any},
    collections::HashMap,
    future::Future,
    pin::Pin,
    sync::Arc,
};

type LockHashMap<K, V> = Arc<DashMap<K, V>>;
// TODO -> type BoxFuture<T> = Pin<Box<dyn Future<Output = T> + Send + Sync>>;
type BoxFuture<T> = Pin<Box<dyn Future<Output = T> + Send>>;
type AsyncRet = BoxFuture<Result<Vec<u8>, HandlerError>>;
type BoxedCallback = Box<dyn Fn(&str, &str, &[u8], Arc<AppData>) -> AsyncRet + Send + Sync>;
type BoxedDefaultWithId = Box<dyn Fn(String) -> Box<dyn Any + Send + Sync> + Send + Sync>;

/// Store objects dynamically, registering handlers for different message types
///
/// The registry also offers the possibility of registering loose functions unique by its argument
/// and return type
#[derive(Default)]
pub struct Registry {
    // (ObjectTypeName, ObjectId) -> Box<Obj>
    object_map: LockHashMap<(String, String), Box<dyn Any + Send + Sync>>,

    // (ObjectTypeName, MessageTypeName) -> Result<SerializedResult, Error>
    handler_map: DashMap<(String, String), BoxedCallback>,

    /// TODO
    type_map: HashMap<String, BoxedDefaultWithId>,
}

impl Registry {
    pub fn new() -> Registry {
        Registry::default()
    }

    /// Add a trait object of type `T` to the object map
    pub async fn add<T: 'static>(&mut self, k: String, v: T)
    where
        T: IdentifiableType + Send + Sync,
    {
        let type_id = T::user_defined_type_id().to_string();
        self.object_map.insert((type_id, k), Box::new(v));
    }

    /// TODO
    pub fn add_type<T>(&mut self)
    where
        T: IdentifiableType + 'static + Send + Sync + Default + WithId,
    {
        let type_id = T::user_defined_type_id().to_string();
        let boxed_fn = Box::new(move |id: String| -> Box<dyn Any + Send + Sync> {
            let mut value = T::default();
            value.set_id(id);
            Box::new(value)
        });
        self.type_map.insert(type_id, boxed_fn);
    }

    /// TODO
    pub fn new_from_type(&self, type_id: &str, id: String) -> Option<Box<dyn Any + Send + Sync>> {
        let ret = self.type_map.get(type_id)?(id);
        Some(ret)
    }

    /// Adds a message (M) handler for a given type (T)
    pub fn add_handler<T, M>(&mut self)
    where
        T: 'static + Handler<M> + IdentifiableType + Send + Sync,
        M: 'static + IdentifiableType + Message + Send + Sync,
    {
        let object_map = self.object_map.clone();
        let type_id = T::user_defined_type_id().to_string();
        let message_type_id = M::user_defined_type_id().to_string();

        let callable = move |type_id: &str,
                             object_id: &str,
                             encoded_message: &[u8],
                             context: Arc<AppData>|
              -> AsyncRet {
            let message: M = match bincode::deserialize(encoded_message) {
                Ok(val) => val,
                Err(_) => return Box::pin(async { Err(HandlerError::MessageSerializationError) }),
            };

            let inner_object_map = object_map.clone();
            let object_key = (type_id.to_string(), object_id.to_string());
            Box::pin(async move {
                let mut boxed_object = inner_object_map
                    .get_mut(&object_key)
                    .ok_or(HandlerError::ObjectNotFound)?;
                let object: &mut T = boxed_object.downcast_mut().ok_or(HandlerError::Unknown)?;
                let ret = object.handle(message, context).await?;
                bincode::serialize(&ret).or(Err(HandlerError::ResponseSerializationError))
            })
        };
        let boxed_callable: BoxedCallback = Box::new(callable);
        let callable_key = (type_id, message_type_id);
        self.handler_map.insert(callable_key, boxed_callable);
    }

    pub async fn send(
        &self,
        type_id: &str,
        object_id: &str,
        message_type_id: &str,
        message: &[u8],
        context: Arc<AppData>,
    ) -> Result<Vec<u8>, HandlerError> {
        let callable_key = (type_id.to_string(), message_type_id.to_string());
        let future_result = {
            let message_handler = self
                .handler_map
                .get(&callable_key)
                .ok_or(HandlerError::HandlerNotFound)?;
            message_handler(type_id, object_id, message, context)
        };
        future_result.await
    }

    pub async fn has(&self, type_id: &str, object_id: &str) -> bool {
        let object_key = (type_id.to_string(), object_id.to_string());
        self.object_map.get(&object_key).is_some()
    }

    /// Build and insert new object to the object map
    pub async fn insert_boxed_object(
        &self,
        type_id: String,
        object_id: String,
        object: Box<dyn Any + 'static + Send + Sync>,
    ) {
        self.object_map.insert((type_id, object_id), object);
    }

    /// remove object from registry
    pub async fn remove(&self, type_id: String, object_id: String) {
        let key = (type_id, object_id);
        self.object_map.remove(&key).map(|(_, _)| ()).or_else(|| {
            println!("TODO: error deleting {:?}", key);
            Some(())
        });
    }
}

/// Define a name for a given Struct so it can be used at runtime.
///
/// By default this will use [std::any::type_name] (which might not be compatible across all your
/// infrastructure), so it is advised to implement your own 'user_defined_type_id'
///
/// <div class="warning">TODO deal with duplicates</div>
pub trait IdentifiableType {
    fn user_defined_type_id() -> &'static str {
        type_name::<Self>()
    }

    /// Same as IdentifiableType::user_defined_type_id, but it can be
    /// called directly from the struct instance. This is handy for when
    /// one uses impl Trait instead of generic
    fn instance_type_id(&self) -> &'static str {
        Self::user_defined_type_id()
    }
}

#[async_trait]
pub trait Handler<M>
where
    Self: Send + Sync,
    M: Message + Send + Sync,
{
    type Returns: Serialize + Sync + Send;
    async fn handle(
        &mut self,
        message: M,
        context: Arc<AppData>,
    ) -> Result<Self::Returns, HandlerError>;
}

pub trait Message: Serialize + DeserializeOwned {}

#[cfg(test)]
mod test {
    use super::*;
    use serde::Deserialize;
    use tokio::sync::RwLock;

    #[derive(Default, Debug, PartialEq)]
    struct Human {
        pub id: String,
    }

    impl IdentifiableType for Human {
        fn user_defined_type_id() -> &'static str {
            "Human"
        }
    }

    impl WithId for Human {
        fn set_id(&mut self, id: String) {
            self.id = id;
        }

        fn id(&self) -> &str {
            &self.id
        }
    }

    #[derive(Default)]
    struct Proxy {
        pub registry: Arc<RwLock<Registry>>,
        pub proxy: bool,
    }
    impl IdentifiableType for Proxy {
        fn user_defined_type_id() -> &'static str {
            "Proxy"
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

    #[derive(Serialize, Deserialize)]
    struct ErrorMessage {
        pub value: String,
    }

    impl IdentifiableType for ErrorMessage {
        fn user_defined_type_id() -> &'static str {
            "ErrorMessage"
        }
    }
    impl Message for ErrorMessage {}

    #[async_trait]
    impl Handler<HiMessage> for Human {
        type Returns = String;
        async fn handle(
            &mut self,
            _message: HiMessage,
            _: Arc<AppData>,
        ) -> Result<String, HandlerError> {
            Ok("hi".to_string())
        }
    }

    #[async_trait]
    impl Handler<HiMessage> for Proxy {
        type Returns = String;
        async fn handle(
            &mut self,
            message: HiMessage,
            context: Arc<AppData>,
        ) -> Result<String, HandlerError> {
            if self.proxy {
                self.registry
                    .read()
                    .await
                    .send(
                        "Proxy",
                        "final-1",
                        "HiMessage",
                        &bincode::serialize(&message).unwrap(),
                        context,
                    )
                    .await
                    .unwrap();
            }
            Ok("hi".to_string())
        }
    }

    #[async_trait]
    impl Handler<GoodbyeMessage> for Human {
        type Returns = String;
        async fn handle(
            &mut self,
            _message: GoodbyeMessage,
            _: Arc<AppData>,
        ) -> Result<String, HandlerError> {
            Ok("bye".to_string())
        }
    }

    #[async_trait]
    impl Handler<ErrorMessage> for Human {
        type Returns = String;
        async fn handle(
            &mut self,
            _message: ErrorMessage,
            _: Arc<AppData>,
        ) -> Result<String, HandlerError> {
            Err(HandlerError::Unknown)
        }
    }

    #[tokio::test]
    async fn sanity_check() {
        fn is_sync<T: Sync>(_t: T) {}
        is_sync(Human::default());
        is_sync(HiMessage {});
        is_sync(Registry::new());
        is_sync(Box::new(Registry::new()));

        let mut registry = Registry::new();
        let obj = Human::default();
        registry.add("john".to_string(), obj).await;
        registry.add_handler::<Human, HiMessage>();
        registry.add_handler::<Human, GoodbyeMessage>();
        registry
            .send(
                "Human",
                "john",
                "HiMessage",
                &bincode::serialize(&HiMessage {}).unwrap(),
                Arc::new(AppData::new()),
            )
            .await
            .unwrap();

        registry
            .send(
                "Human",
                "john",
                "GoodbyeMessage",
                &bincode::serialize(&GoodbyeMessage {}).unwrap(),
                Arc::new(AppData::new()),
            )
            .await
            .unwrap();

        registry
            .remove("Human".to_string(), "john".to_string())
            .await;

        registry
            .send(
                "Human",
                "john",
                "GoodbyeMessage",
                &bincode::serialize(&GoodbyeMessage {}).unwrap(),
                Arc::new(AppData::new()),
            )
            .await
            .unwrap_err();
    }

    #[tokio::test]
    async fn test_return() {
        let mut registry = Registry::new();
        let obj = Human::default();
        registry.add("john".to_string(), obj).await;
        registry.add_handler::<Human, HiMessage>();
        let ret = registry
            .send(
                "Human",
                "john",
                "HiMessage",
                &bincode::serialize(&HiMessage {}).unwrap(),
                Arc::new(AppData::new()),
            )
            .await;
        let result: String = bincode::deserialize(&ret.unwrap()).unwrap();
        assert_eq!(result, "hi")
    }

    #[tokio::test]
    async fn test_return_error() {
        let mut registry = Registry::new();
        let obj = Human::default();
        registry.add("john".to_string(), obj).await;
        registry.add_handler::<Human, ErrorMessage>();
        let ret = registry
            .send(
                "Human",
                "john",
                "ErrorMessage",
                &bincode::serialize(&ErrorMessage {
                    value: "Test".to_string(),
                })
                .unwrap(),
                Arc::new(AppData::new()),
            )
            .await;
        assert_eq!(ret, Err(HandlerError::Unknown));
    }

    #[tokio::test]
    async fn test_not_registered_message() {
        let mut registry = Registry::new();
        let obj = Human::default();
        registry.add("john".to_string(), obj).await;
        let ret = registry
            .send(
                "Human",
                "john",
                "HiMessage",
                &bincode::serialize(&HiMessage {}).unwrap(),
                Arc::new(AppData::new()),
            )
            .await;
        assert_eq!(ret, Err(HandlerError::HandlerNotFound));
    }

    #[tokio::test]
    async fn test_object_not_found() {
        let mut registry = Registry::new();
        registry.add_handler::<Human, HiMessage>();
        let ret = registry
            .send(
                "Human",
                "john",
                "HiMessage",
                &bincode::serialize(&HiMessage {}).unwrap(),
                Arc::new(AppData::new()),
            )
            .await;
        assert!(ret.is_err());
    }

    #[tokio::test]
    async fn test_trait_not_registered() {
        let mut registry = Registry::new();
        registry.add_handler::<Human, HiMessage>();
        let ret = registry
            .send(
                "NotHuman",
                "john",
                "HiMessage",
                &bincode::serialize(&HiMessage {}).unwrap(),
                Arc::new(AppData::new()),
            )
            .await;
        assert_eq!(ret, Err(HandlerError::HandlerNotFound));
    }

    #[tokio::test]
    async fn test_message_deserialization_error() {
        let mut registry = Registry::new();
        registry.add_handler::<Human, ErrorMessage>();
        registry
            .insert_boxed_object(
                "Human".to_string(),
                "john".to_string(),
                Box::new(Human::default()),
            )
            .await;
        let ret = registry
            .send(
                "Human",
                "john",
                "ErrorMessage",
                b"",
                Arc::new(AppData::new()),
            )
            .await;
        assert_eq!(ret, Err(HandlerError::MessageSerializationError));
    }

    #[tokio::test]
    async fn test_proxy_deadlock() {
        let registry = Registry::new();
        let registry = Arc::new(RwLock::new(registry));

        registry.write().await.add_handler::<Proxy, HiMessage>();

        registry
            .write()
            .await
            .add(
                "proxy-1".to_string(),
                Proxy {
                    registry: registry.clone(),
                    proxy: true,
                },
            )
            .await;

        registry
            .write()
            .await
            .add(
                "final-1".to_string(),
                Proxy {
                    registry: registry.clone(),
                    proxy: false,
                },
            )
            .await;

        registry
            .read()
            .await
            .send(
                "Proxy",
                "proxy-1",
                "HiMessage",
                b"",
                Arc::new(AppData::new()),
            )
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn test_send_sync() {
        let join_handler = tokio::spawn(async move {
            let mut registry = Registry::new();
            registry.add_handler::<Human, HiMessage>();
            let obj = Human::default();
            registry.add("john".to_string(), obj).await;
            registry
                .send(
                    "Human",
                    "john",
                    "HiMessage",
                    &bincode::serialize(&HiMessage {}).unwrap(),
                    Arc::new(AppData::new()),
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
        let obj = Human::default();
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
                    Arc::new(AppData::new()),
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
        let obj = Human::default();
        registry.add("john".to_string(), obj).await;
        assert!(registry.has("Human", "john").await);
        assert!(!registry.has("Human", "not john").await);
    }

    #[tokio::test]
    async fn test_insert_object() {
        let mut registry = Registry::new();
        registry.add_handler::<Human, HiMessage>();
        assert!(!registry.has("Human", "john").await);
        registry
            .insert_boxed_object(
                "Human".to_string(),
                "john".to_string(),
                Box::new(Human::default()),
            )
            .await;
        assert!(registry.has("Human", "john").await);
    }

    #[tokio::test]
    async fn test_new_from_type() {
        let mut registry = Registry::new();
        registry.add_type::<Human>();
        let boxed_human = registry.new_from_type("Human", "1".to_string()).unwrap();
        let human = boxed_human.downcast::<Human>().unwrap();
        assert_eq!(human.id(), "1");
    }
}
