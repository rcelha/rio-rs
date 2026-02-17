//! Trait object registry
//!
//! Provides storage for objects and maps their callables to handle registered message types

use crate::{app_data::AppData, errors::HandlerError, WithId};
use dashmap::DashMap;
use log::warn;
use std::{
    any::{Any, TypeId},
    collections::HashMap,
    fmt::Debug,
    future::Future,
    pin::Pin,
    sync::Arc,
};
use tokio::sync::RwLock;
use tracing::Instrument;

mod handler;
mod identifiable_type;

pub use handler::{Handler, Message};
pub use identifiable_type::IdentifiableType;

type LockHashMap<K, V> = Arc<DashMap<K, RwLock<V>>>;
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
    /// Object allocation map
    /// `(ObjectTypeName, ObjectId)` -> `Box<Obj>`
    object_map: LockHashMap<(String, String), Box<dyn Any + Send + Sync>>,

    /// Maps the objects types and messages to their handler functions
    /// (ObjectTypeName, MessageTypeName) -> Result<SerializedResult, Error>
    handler_map_: papaya::HashMap<(String, String), BoxedCallback>,

    /// Maps the types to the object constructors
    type_map: HashMap<String, BoxedDefaultWithId>,

    /// Internal control for duplicate type ids
    supported_types: HashMap<String, TypeId>,
}

impl Debug for Registry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Registry")
            .field("object_map", &"DashMap<(String, String), Box<dyn Any + Send + Sync>>")
            .field(
                "handler_map_",
                &"papaya::HashMap<(String, String), Box<dyn Fn(&str, &str, &[u8], Arc<AppData>) -> AsyncRet + Send + Sync>>",
            )
            .finish()
    }
}

impl Registry {
    pub fn new() -> Registry {
        Registry::default()
    }

    /// Add a trait object of type `T` to the object map
    pub async fn add<T>(&mut self, k: String, v: T)
    where
        T: 'static + IdentifiableType + Send + Sync,
    {
        let type_id = T::user_defined_type_id().to_string();
        self.object_map
            .insert((type_id, k), RwLock::new(Box::new(v)));
    }

    /// Add new types to the contructor map
    ///
    /// It will emmit warnings regarding duplicate types, and it won't add the duplicates
    pub fn add_type<T>(&mut self)
    where
        T: IdentifiableType + 'static + Send + Sync + Default + WithId,
    {
        let type_id = T::user_defined_type_id().to_string();
        let type_of = TypeId::of::<T>();

        if self.type_map.contains_key(&type_id) {
            return;
        }

        if let Some(duplicate_control) = self.supported_types.get(&type_id) {
            if duplicate_control != &type_of {
                warn!(
                    "The type {:?} is already present, and it represents another object type",
                    type_id
                );
                warn!("You might have a duplicate on your code-base");
                return;
            }
        }

        let boxed_fn = Box::new(move |id: String| -> Box<dyn Any + Send + Sync> {
            let mut value = T::default();
            value.set_id(id);
            Box::new(value)
        });
        self.type_map.insert(type_id.clone(), boxed_fn);
        self.supported_types.insert(type_id, type_of);
    }

    /// Creates a new object with some id (using FromId)
    ///
    /// <div class="warning">TODO deal existing objects to avoid double allocation</div>
    pub fn new_from_type(&self, type_id: &str, id: String) -> Option<Box<dyn Any + Send + Sync>> {
        let builder = self.type_map.get(type_id)?;
        let ret = builder(id);
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
            Box::pin(
                async move {
                    let boxed_object_guard = inner_object_map
                        .get(&object_key)
                        .ok_or(HandlerError::ObjectNotFound)?;
                    let mut boxed_object = boxed_object_guard
                        .write()
                        .instrument(tracing::info_span!("handler_lock_acquire"))
                        .await;

                    let object: &mut T =
                        boxed_object.downcast_mut().ok_or(HandlerError::Unknown)?;

                    let handler_result = object
                        .handle(message, context)
                        .instrument(tracing::info_span!("handler_handle"))
                        .await;

                    // Serializes the error into a binary variant
                    // We do this to support 'custom' error types for each one of the Handler's
                    // implementation
                    let ret = handler_result.map_err(|err| {
                        let ser_err = bincode::serialize(&err).expect("TODO");
                        HandlerError::ApplicationError(ser_err)
                    })?;

                    // Serialize the whole result back to the caller
                    bincode::serialize(&ret).or(Err(HandlerError::ResponseSerializationError))
                }
                .instrument(tracing::info_span!("handler_get_and_handle")),
            )
        };
        let boxed_callable: BoxedCallback = Box::new(callable);
        let callable_key = (type_id, message_type_id);
        self.handler_map_.pin().insert(callable_key, boxed_callable);
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
            let handler_map_pin = self.handler_map_.guard();
            let message_handler = self
                .handler_map_
                .get(&callable_key, &handler_map_pin)
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
        self.object_map
            .insert((type_id, object_id), RwLock::new(object));
    }

    /// remove object from registry
    pub async fn remove(&self, type_id: String, object_id: String) {
        let key = (type_id, object_id);

        if self.object_map.remove(&key).is_none() {
            warn!(
                "Failed to remove object from mapping ({:?} not present)",
                key
            );
        }

        let handler_map = self.handler_map_.pin();
        if handler_map.remove(&key).is_none() {
            warn!(
                "Failed to remove handler from mapping ({:?} not present)",
                key
            );
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use async_trait::async_trait;
    use serde::{Deserialize, Serialize};
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
        pub id: String,
        pub registry: Arc<RwLock<Registry>>,
        pub proxy: bool,
    }

    impl WithId for Proxy {
        fn set_id(&mut self, id: String) {
            self.id = id;
        }

        fn id(&self) -> &str {
            &self.id
        }
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
        type Error = String;

        async fn handle(&mut self, _message: HiMessage, _: Arc<AppData>) -> Result<String, String> {
            Ok("hi".to_string())
        }
    }

    #[async_trait]
    impl Handler<HiMessage> for Proxy {
        type Returns = String;
        type Error = String;

        async fn handle(
            &mut self,
            message: HiMessage,
            context: Arc<AppData>,
        ) -> Result<String, String> {
            if self.proxy {
                let final_id = "final-1".to_string();

                self.registry
                    .read()
                    .await
                    .send(
                        "Proxy",
                        &final_id,
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
        type Error = String;

        async fn handle(
            &mut self,
            _message: GoodbyeMessage,
            _: Arc<AppData>,
        ) -> Result<String, String> {
            Ok("bye".to_string())
        }
    }

    #[async_trait]
    impl Handler<ErrorMessage> for Human {
        type Returns = String;
        type Error = String;
        async fn handle(
            &mut self,
            _message: ErrorMessage,
            _: Arc<AppData>,
        ) -> Result<String, String> {
            Err("err".to_string())
        }
    }

    #[tokio::test]
    async fn sanity_check() {
        fn is_sync<T: Sync>(_t: T) {}
        fn is_send<T: Send>(_t: T) {}

        is_sync(Human::default());
        is_sync(HiMessage {});
        is_sync(Registry::new());
        is_sync(Box::new(Registry::new()));

        is_send(Human::default());
        is_send(HiMessage {});
        is_send(Registry::new());
        is_send(Box::new(Registry::new()));

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
        assert!(matches!(ret, Err(HandlerError::ApplicationError(_))));
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
        let count = 1_000_000;
        let registry = Registry::new();
        let registry = Arc::new(RwLock::new(registry));

        registry.write().await.add_handler::<Proxy, HiMessage>();

        let mut join_set = tokio::task::JoinSet::new();
        let local_reg = registry.clone();
        join_set.spawn(async move {
            local_reg
                .write()
                .await
                .add(
                    "final-1".to_string(),
                    Proxy {
                        id: "final-1".to_string(),
                        registry: local_reg.clone(),
                        proxy: false,
                    },
                )
                .await;
        });

        for i in 1..count {
            let local_reg = registry.clone();
            join_set.spawn(async move {
                local_reg
                    .write()
                    .await
                    .add(
                        format!("proxy-{}", i),
                        Proxy {
                            id: format!("proxy-{}", i),
                            registry: local_reg.clone(),
                            proxy: true,
                        },
                    )
                    .await;
            });
        }
        let _ = join_set.join_all().await;

        let app_data = Arc::new(AppData::new());
        let mut join_set = tokio::task::JoinSet::new();
        for i in 1..count {
            let local_reg = registry.clone();
            let local_app_data = app_data.clone();
            join_set.spawn(async move {
                local_reg
                    .read()
                    .await
                    .send(
                        "Proxy",
                        &format!("proxy-{}", i),
                        "HiMessage",
                        b"",
                        local_app_data,
                    )
                    .await
                    .unwrap();
            });
        }
        join_set.join_all().await;
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
