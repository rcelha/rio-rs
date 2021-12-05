use std::{
    any::{type_name, Any},
    collections::HashMap,
};

use serde::{de::DeserializeOwned, Deserialize, Serialize};

mod errors;
pub use errors::*;

pub struct Registry {
    // (ObjectTypeName, ObjectId) -> Box<Obj>
    mapping: HashMap<(String, String), Box<dyn Any>>,
    // (ObjectTypeName, MessageTypeName) -> Result<SerializedResult, Error>
    callable_mapping: HashMap<
        (String, String),
        Box<dyn FnMut(&mut Box<dyn Any>, &[u8]) -> Result<Vec<u8>, HandlerError>>,
    >,
}

impl Registry {
    pub fn new() -> Registry {
        Registry {
            mapping: HashMap::new(),
            callable_mapping: HashMap::new(),
        }
    }

    pub fn add<T: 'static>(&mut self, k: String, v: T)
    where
        T: IdentifiableType,
    {
        let type_id = T::user_defined_type_id().to_string();
        self.mapping.insert((type_id, k), Box::new(v));
    }

    pub fn add_handler<T: 'static, M: 'static>(&mut self)
    where
        T: Handler<M> + IdentifiableType,
        M: IdentifiableType + Message,
    {
        let type_id = T::user_defined_type_id().to_string();
        let message_type_id = M::user_defined_type_id().to_string();

        let callable = move |any_obj: &mut Box<dyn Any>,
                             encoded_message: &[u8]|
              -> Result<Vec<u8>, HandlerError> {
            let obj: &mut T = any_obj
                .downcast_mut()
                .ok_or_else(|| HandlerError::Unknown)?;
            let message: M =
                bincode::deserialize(&encoded_message).or_else(|_| Err(HandlerError::Unknown))?;
            let ret = obj.handle(message)?;
            bincode::serialize(&ret).or_else(|_| Err(HandlerError::ResponseSerializationError))
        };
        self.callable_mapping
            .insert((type_id, message_type_id), Box::new(callable));
    }

    pub fn send(
        &mut self,
        type_id: &str,
        object_id: &str,
        message_type_id: &str,
        message: &[u8],
    ) -> Result<Vec<u8>, HandlerError> {
        let object_key = (type_id.to_string(), object_id.to_string());
        let object = self
            .mapping
            .get_mut(&object_key)
            .ok_or_else(|| HandlerError::ObjectNotFound)?;

        let callable_key = (type_id.to_string(), message_type_id.to_string());
        let callable = self
            .callable_mapping
            .get_mut(&callable_key)
            .ok_or_else(|| HandlerError::HandlerNotFound)?;

        println!("found callable {}/{}", type_id, message_type_id);
        callable(object, message)
    }
}

// TODO create a derive for this
pub trait IdentifiableType {
    fn user_defined_type_id() -> &'static str {
        type_name::<Self>()
    }
}

pub trait Handler<M>
where
    M: Message,
{
    fn handle(&mut self, message: M) -> Result<M::Returns, HandlerError>;
}

pub trait Message: Serialize + DeserializeOwned {
    type Returns: Serialize;
}

#[cfg(test)]
mod test {
    use super::*;

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
    impl Message for HiMessage {
        type Returns = String;
    }

    #[derive(Serialize, Deserialize)]
    struct GoodbyeMessage {}
    impl IdentifiableType for GoodbyeMessage {
        fn user_defined_type_id() -> &'static str {
            "GoodbyeMessage"
        }
    }
    impl Message for GoodbyeMessage {
        type Returns = String;
    }

    impl Handler<HiMessage> for Human {
        fn handle(&mut self, _message: HiMessage) -> Result<String, HandlerError> {
            Ok("hi".to_string())
        }
    }
    impl Handler<GoodbyeMessage> for Human {
        fn handle(&mut self, _message: GoodbyeMessage) -> Result<String, HandlerError> {
            Ok("bye".to_string())
        }
    }

    #[test]
    fn sanity_check() {
        let mut registry = Registry::new();
        let obj = Human {};
        registry.add("john".to_string(), obj);
        registry.add_handler::<Human, HiMessage>();
        registry.add_handler::<Human, GoodbyeMessage>();
        registry
            .send(
                "Human",
                "john",
                "HiMessage",
                &bincode::serialize(&HiMessage {}).unwrap(),
            )
            .unwrap();

        registry
            .send(
                "Human",
                "john",
                "GoodbyeMessage",
                &bincode::serialize(&GoodbyeMessage {}).unwrap(),
            )
            .unwrap();
    }

    #[test]
    fn test_return() {
        let mut registry = Registry::new();
        let obj = Human {};
        registry.add("john".to_string(), obj);
        registry.add_handler::<Human, HiMessage>();
        let ret = registry.send(
            "Human",
            "john",
            "HiMessage",
            &bincode::serialize(&HiMessage {}).unwrap(),
        );
        let result: String = bincode::deserialize(&ret.unwrap()).unwrap();
        assert_eq!(result, "hi")
    }

    #[test]
    fn test_not_registered_message() {
        let mut registry = Registry::new();
        let obj = Human {};
        registry.add("john".to_string(), obj);
        let ret = registry.send(
            "Human",
            "john",
            "HiMessage",
            &bincode::serialize(&HiMessage {}).unwrap(),
        );
        assert!(ret.is_err());
    }
}
