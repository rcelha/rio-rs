use std::{
    any::{type_name, Any},
    collections::HashMap,
};

pub struct Registry {
    // (ObjectTypeName, ObjectId) -> Box<Obj>
    mapping: HashMap<(String, String), Box<dyn Any>>,
    // (ObjectTypeName, MessageTypeName) -> Box<Fn>
    callable_mapping:
        HashMap<(String, String), Box<dyn FnMut(&mut Box<dyn Any>, Box<dyn Any>) -> Box<dyn Any>>>,
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

        let callable = move |any_obj: &mut Box<dyn Any>, any_args: Box<dyn Any>| -> Box<dyn Any> {
            let obj: &mut T = any_obj.downcast_mut().expect("Type conversion failed");
            let message: M = *any_args.downcast().expect("Type conversion failed");
            let ret = obj.handle(message);
            Box::new(ret)
        };
        self.callable_mapping
            .insert((type_id, message_type_id), Box::new(callable));
    }

    pub fn send(
        &mut self,
        type_id: &str,
        object_id: &str,
        message_type_id: &str,
        message: Box<dyn Any>,
    ) -> Box<dyn Any> {
        let object_key = (type_id.to_string(), object_id.to_string());
        let object = self.mapping.get_mut(&object_key);
        if object.is_none() {
            println!("NO found object {}/{}", type_id, object_id);
            println!("Current {:?}", self.mapping);
            return Box::new(());
        }
        let object = object.unwrap();
        println!("found object {}/{}", type_id, object_id);

        let callable_key = (type_id.to_string(), message_type_id.to_string());
        let callable = self.callable_mapping.get_mut(&callable_key);
        if callable.is_none() {
            println!("NO found message handler {}/{}", type_id, message_type_id);
            println!("Current {:?}", self.callable_mapping.keys());
            return Box::new(());
        }

        let callable = callable.unwrap();
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
    fn handle(&mut self, message: M) -> Result<M::Returns, ()>;
}

pub trait Message {
    type Returns;
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

    struct HiMessage {}
    impl IdentifiableType for HiMessage {
        fn user_defined_type_id() -> &'static str {
            "HiMessage"
        }
    }
    impl Message for HiMessage {
        type Returns = String;
    }

    struct GoodbyeMessage {}
    impl IdentifiableType for GoodbyeMessage {}
    impl Message for GoodbyeMessage {
        type Returns = String;
    }

    impl Handler<HiMessage> for Human {
        fn handle(&mut self, _message: HiMessage) -> Result<String, ()> {
            println!("hi");
            Ok("hi".to_string())
        }
    }
    impl Handler<GoodbyeMessage> for Human {
        fn handle(&mut self, _message: GoodbyeMessage) -> Result<String, ()> {
            println!("bye");
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
        registry.send("Human", "john", "HiMessage", Box::new(HiMessage {}));
        registry.send(
            "Human",
            "john",
            "rio_rs::test::GoodbyeMessag",
            Box::new(GoodbyeMessage {}),
        );
    }

    #[test]
    fn test_return() {
        let mut registry = Registry::new();
        let obj = Human {};
        registry.add("john".to_string(), obj);
        registry.add_handler::<Human, HiMessage>();
        let ret = registry.send("Human", "john", "HiMessage", Box::new(HiMessage {}));
        let result: Result<String, ()> = *ret.downcast().unwrap();
        assert_eq!(result, Ok("hi".to_string()));
    }

    #[test]
    fn test_not_registered_message() {
        let mut registry = Registry::new();
        let obj = Human {};
        registry.add("john".to_string(), obj);
        registry.send("Human", "john", "HiMessage", Box::new(HiMessage {}));
    }
}
