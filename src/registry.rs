//! Trait object registry
//!
//! Provides storage for objects and maps their callables to handle registered message types

use crate::{app_data::AppData, errors::HandlerError};
use async_trait::async_trait;
use dashmap::DashMap;
use serde::{de::DeserializeOwned, Serialize};
use std::{
    any::{type_name, Any, TypeId},
    collections::HashMap,
    future::Future,
    pin::Pin,
    sync::Arc,
};

type LockHashMap<K, V> = Arc<DashMap<K, V>>;
type BoxFuture<T> = Pin<Box<dyn Future<Output = T> + Send>>;
type AsyncRet = BoxFuture<Result<Vec<u8>, HandlerError>>;
type BoxedCallback = Box<dyn Fn(&str, &str, &[u8], Arc<AppData>) -> AsyncRet + Send>;
type BoxedDefault = Box<dyn Fn() -> Box<dyn Any + Send + Sync> + Send + Sync>;
type BoxedStatic = Box<dyn Fn(Box<dyn Any>) -> Box<dyn Any + Send + Sync> + Send + Sync>;

#[derive(Default)]
pub struct Registry {
    // (ObjectTypeName, ObjectId) -> Box<Obj>
    object_map: LockHashMap<(String, String), Box<dyn Any + Send + Sync>>,
    // (ObjectTypeName, MessageTypeName) -> Result<SerializedResult, Error>
    handler_map: DashMap<(String, String), BoxedCallback>,
    // (ObjectTypeName) -> Fn() -> Box<Object>
    builder_map: HashMap<String, BoxedDefault>,

    // Static Map
    static_fn_map: HashMap<(String, TypeId), BoxedStatic>,
}

unsafe impl Send for Registry {}
unsafe impl Sync for Registry {}

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
    pub fn add_static_fn<T, Argument, Function>(&mut self, static_fn: Function)
    where
        T: IdentifiableType + 'static + Send + Sync,
        Argument: Any,
        Function: Fn(Argument) -> T + 'static + Send + Sync,
    {
        let type_id = T::user_defined_type_id().to_string();
        let argument_type_id = TypeId::of::<Argument>();
        let boxed_fn = Box::new(move |arg: Box<dyn Any>| -> Box<dyn Any + Send + Sync> {
            let cast_arg = arg
                .downcast::<Argument>()
                .expect("TODO: Unsupported function called");
            Box::new(static_fn(*cast_arg))
        });

        let _ = self
            .static_fn_map
            .insert((type_id, argument_type_id), boxed_fn);
    }

    /// TODO
    pub fn call_static_fn<T, Argument>(&mut self, argument: Argument) -> Option<T>
    where
        T: IdentifiableType + 'static + Send + Sync + Default,
        Argument: Any,
    {
        let type_id = T::user_defined_type_id().to_string();
        let argument_type_id = TypeId::of::<Argument>();
        let ret = self.static_fn_map.get(&(type_id, argument_type_id))?(Box::new(argument));
        let ret = ret.downcast::<T>().ok()?;
        Some(*ret)
    }

    /// TODO
    pub fn call_static_fn_box<Argument>(
        &self,
        type_id: String,
        argument: Argument,
    ) -> Option<Box<dyn Any + Send + Sync>>
    where
        Argument: Any,
    {
        let argument_type_id = TypeId::of::<Argument>();
        let ret = self.static_fn_map.get(&(type_id, argument_type_id))?(Box::new(argument));
        Some(ret)
    }

    /// Add a builder of type `T` to the builder map
    ///
    /// The builder is just a shortcut to insert a object generated with default to
    /// the object map
    pub fn add_builder<T: 'static>(&mut self)
    where
        T: IdentifiableType + Default + Send + Sync,
    {
        let k = T::user_defined_type_id().to_string();
        self.builder_map
            .insert(k, Box::new(move || Box::new(T::default())));
    }

    /// Adds a message (M) handler for a given type (T)
    pub fn add_handler<T: 'static, M: 'static>(&mut self)
    where
        T: 'static + Handler<M> + IdentifiableType + Default + Send + Sync,
        M: 'static + IdentifiableType + Message + Send,
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

        // TODO only do it once
        self.add_builder::<T>();
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
    pub async fn insert_object(&self, type_id: String, object_id: String) {
        let default = self.builder_map.get(&type_id).unwrap();
        self.object_map.insert((type_id, object_id), default());
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
    struct Human {}
    impl IdentifiableType for Human {
        fn user_defined_type_id() -> &'static str {
            "Human"
        }
    }

    impl From<String> for Human {
        fn from(_: String) -> Self {
            Human {}
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
        let _: Box<dyn Any + Send + Sync> = Box::new(Registry::new());
        let _: Box<dyn Any + Send> = Box::new(Registry::new());
        let _: Box<dyn Any + Sync> = Box::new(Registry::new());

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
        let obj = Human {};
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
        let obj = Human {};
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
        let obj = Human {};
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
            .insert_object("Human".to_string(), "john".to_string())
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
            let obj = Human {};
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

    #[tokio::test]
    async fn test_static_fn() {
        let mut registry = Registry::new();
        registry.add_static_fn::<Human, String, _>(From::<String>::from);
        let new_human: Option<Human> = registry.call_static_fn("Oi".to_string());
        assert!(new_human.is_some());

        let boxed_human = registry.call_static_fn_box("Human".to_string(), "Oi".to_string());
        assert!(boxed_human.is_some());

        assert_eq!(
            new_human.unwrap(),
            *boxed_human.unwrap().downcast::<Human>().unwrap()
        );
    }
}
