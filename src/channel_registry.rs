use std::{
    any::{Any, TypeId},
    collections::HashMap,
    error::Error,
    fmt::Debug,
    sync::Arc,
};

use tokio::sync::{mpsc, oneshot, RwLock};

use crate::{
    app_data::AppData,
    registry::{Handler, Message},
};

#[derive(Default)]
pub struct Registry {
    inboxes: HashMap<String, Box<dyn Any>>,
}

impl Registry {
    pub fn new() -> Registry {
        Registry::default()
    }

    fn object_id<T: 'static, M: 'static>(object_id: &str) -> String {
        format!(
            "{:?}/{:?}/{}",
            TypeId::of::<T>(),
            TypeId::of::<M>(),
            object_id
        )
    }

    pub fn register<T, M, R>(&mut self) -> Result<(), Box<dyn Error>> {
        Ok(())
    }

    pub fn add_object<T, M, R>(
        &mut self,
        object: Arc<RwLock<T>>,
        object_id: String,
    ) -> Result<(), Box<dyn Error>>
    where
        T: 'static + Send + Sync + Handler<M, Returns = R>,
        M: 'static + Debug + Send + Sync + Message,
        R: 'static + Debug + Send + Sync + Message,
    {
        let (inbox_tx, mut inbox_rx) =
            mpsc::unbounded_channel::<(M, Arc<AppData>, oneshot::Sender<R>)>();

        tokio::task::spawn(async move {
            while let Some(i) = inbox_rx.recv().await {
                let response = object.write().await.handle(i.0, i.1).await.unwrap();
                i.2.send(response).expect("TODO: fail to send response");
            }
        });

        let object_id = Self::object_id::<T, M>(&object_id);
        self.inboxes.insert(object_id, Box::new(inbox_tx));
        Ok(())
    }

    /// T: Object type in the registry
    /// M: Message the object receives
    /// R: Return from message handler
    pub async fn send<T, M, R>(
        &self,
        object_id: &str,
        message: M,
        app_data: Arc<AppData>,
    ) -> Result<R, Box<dyn Error>>
    where
        T: 'static + Send + Sync,
        M: 'static + Debug + Default + Send + Sync,
        R: 'static + Debug + Default + Send + Sync,
    {
        let object_id = Self::object_id::<T, M>(object_id);
        let inbox_tx = self.inboxes.get(&object_id).unwrap();
        let inbox_tx: &mpsc::UnboundedSender<(M, Arc<AppData>, oneshot::Sender<R>)> =
            inbox_tx.downcast_ref().unwrap();

        let (outbox_tx, outbox_rx) = oneshot::channel::<R>();
        inbox_tx.send((message, app_data, outbox_tx)).unwrap();
        Ok(outbox_rx.await.unwrap())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{app_data::AppData, prelude::HandlerError, registry::Message};
    use serde::{Deserialize, Serialize};
    use tokio::sync::RwLock;

    #[derive(Clone, Default, Debug, PartialEq, Serialize, Deserialize)]
    struct Ping {}
    impl Message for Ping {}

    #[derive(Default, Debug, PartialEq, Serialize, Deserialize)]
    struct Pong {}
    impl Message for Pong {}

    #[derive(Default, Debug, PartialEq, Serialize, Deserialize)]
    struct Bounce {}
    impl Message for Bounce {}

    struct Ball {
        pub kind: &'static str,
    }

    #[async_trait::async_trait]
    impl Handler<Ping> for Ball {
        type Returns = Pong;

        async fn handle(
            &mut self,
            _: Ping,
            _: Arc<AppData>,
        ) -> Result<Self::Returns, HandlerError> {
            println!("ping ... pong");
            println!("kind {}", self.kind);
            Ok(Pong {})
        }
    }

    #[async_trait::async_trait]
    impl Handler<Bounce> for Ball {
        type Returns = Bounce;

        async fn handle(
            &mut self,
            message: Bounce,
            _: Arc<AppData>,
        ) -> Result<Self::Returns, HandlerError> {
            println!("pong! pong! pong!");
            Ok(message)
        }
    }

    #[tokio::test]
    async fn sanity_check() {
        let mut r = Registry::new();
        r.register::<Ball, Ping, Pong>().unwrap();
        r.register::<Ball, Bounce, Bounce>().unwrap();

        let object = Arc::new(RwLock::new(Ball { kind: "round" }));
        let object2 = Arc::new(RwLock::new(Ball { kind: "square" }));

        r.add_object::<_, Ping, Pong>(object.clone(), "round".to_string())
            .unwrap();
        r.add_object::<_, Ping, Pong>(object2.clone(), "square".to_string())
            .unwrap();
        r.add_object::<_, Bounce, Bounce>(object.clone(), "round".to_string())
            .unwrap();

        let app_data = Arc::new(AppData::new());

        println!("-----");
        let response = r
            .send::<Ball, Ping, Pong>("round", Ping {}, app_data.clone())
            .await
            .unwrap();
        assert_eq!(response, Pong {});

        println!("-----");
        let response = r
            .send::<Ball, Ping, Pong>("square", Ping {}, app_data.clone())
            .await
            .unwrap();
        assert_eq!(response, Pong {});

        println!("-----");
        let response = r
            .send::<Ball, Bounce, Bounce>("round", Bounce {}, app_data.clone())
            .await
            .unwrap();
        assert_eq!(response, Bounce {});
    }
}
