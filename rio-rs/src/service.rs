use futures::future::BoxFuture;
use futures::sink::SinkExt;
use futures::{Stream, StreamExt};
use std::sync::Arc;

use tokio::net::TcpStream;
use tokio::sync::RwLock;
use tokio_util::codec::{Framed, LengthDelimitedCodec};
use tower::Service as TowerService;

use crate::app_data::{AppData, AppDataExt};
use crate::cluster::storage::MembersStorage;
use crate::message_router::MessageRouter;
use crate::object_placement::{ObjectPlacement, ObjectPlacementProvider};
use crate::protocol::pubsub::{SubscriptionRequest, SubscriptionResponse};
use crate::protocol::{RequestEnvelope, ResponseEnvelope, ResponseError};
use crate::registry::Registry;
use crate::{LifecycleMessage, ObjectId};

pub struct Service<S: MembersStorage, P: ObjectPlacementProvider> {
    pub(crate) address: String,
    pub(crate) registry: Arc<RwLock<Registry>>,
    pub(crate) members_storage: S,
    pub(crate) object_placement_provider: Arc<RwLock<P>>,
    pub(crate) app_data: Arc<AppData>,
}

impl<S: MembersStorage + 'static, P: ObjectPlacementProvider + 'static> Clone for Service<S, P> {
    fn clone(&self) -> Self {
        Self {
            address: self.address.clone(),
            registry: self.registry.clone(),
            members_storage: self.members_storage.clone(),
            object_placement_provider: self.object_placement_provider.clone(),
            app_data: self.app_data.clone(),
        }
    }
}

impl<S: MembersStorage + 'static, P: ObjectPlacementProvider + 'static>
    TowerService<RequestEnvelope> for Service<S, P>
{
    type Response = ResponseEnvelope;
    type Error = ResponseError;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(
        &mut self,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        std::task::Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: RequestEnvelope) -> Self::Future {
        let this = self.clone();
        let result = async move {
            let server_address = this
                .get_or_create_placement(req.handler_type.clone(), req.handler_id.clone())
                .await;

            let address_mismatch = this.get_address_mismatch_error(server_address).await;
            address_mismatch.map_or(Ok(()), Err)?;
            this.start_service_object(&req.handler_type, &req.handler_id)
                .await
                .expect("TODO");

            let response = this
                .registry
                .read()
                .await
                .send(
                    &req.handler_type,
                    &req.handler_id,
                    &req.message_type,
                    &req.payload,
                    this.app_data.clone(),
                )
                .await;

            match response {
                Ok(body) => Ok(ResponseEnvelope::new(body)),
                Err(err) => Err(ResponseError::Unknown(format!(
                    "[TODO] HandlerError: {}",
                    err
                ))),
            }
        };
        Box::pin(result)
    }
}

// Pub/sub service impl
#[derive(Debug)]
pub struct SubscriptionResponseIter {
    receiver_stream: tokio_stream::wrappers::BroadcastStream<SubscriptionResponse>,
}

impl SubscriptionResponseIter {
    pub fn new(channel: tokio::sync::broadcast::Receiver<SubscriptionResponse>) -> Self {
        let receiver_stream = tokio_stream::wrappers::BroadcastStream::new(channel);
        Self { receiver_stream }
    }
}

impl Stream for SubscriptionResponseIter {
    type Item = SubscriptionResponse;
    fn poll_next(
        self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        let this = self.get_mut();
        this.receiver_stream.poll_next_unpin(_cx).map(|i| {
            if let Some(result) = i {
                // TODO error handling
                // TODO deal with redirect
                // TODO deal with objects being removed from the current host!
                result.ok()
            } else {
                None
            }
        })
    }
}

/// Service implementation to handle [SubscriptionRequest] messages
impl<S, P> TowerService<SubscriptionRequest> for Service<S, P>
where
    S: MembersStorage + 'static,
    P: ObjectPlacementProvider + 'static,
{
    type Response = SubscriptionResponseIter;
    type Error = ResponseError;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(
        &mut self,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        std::task::Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: SubscriptionRequest) -> Self::Future {
        let this = self.clone();
        let result = async move {
            let server_address = this
                .get_or_create_placement(req.handler_type.clone(), req.handler_id.clone())
                .await;

            let address_mismatch = this.get_address_mismatch_error(server_address).await;
            address_mismatch.map_or(Ok(()), Err)?;
            // TODO deal with redirect
            this.start_service_object(&req.handler_type, &req.handler_id)
                .await
                .expect("TODO");
            let receiver = this
                .app_data
                .get_or_default::<MessageRouter>()
                .create_subscription(req.handler_type.clone(), req.handler_id.clone());
            Ok(SubscriptionResponseIter::new(receiver))
        };
        Box::pin(result)
    }
}

impl<S: MembersStorage + 'static, P: ObjectPlacementProvider + 'static> Service<S, P> {
    /// Returns the ip:port for where this object is placed
    ///
    /// If the object is not instantiated anywhere, it will allocate locally
    ///
    async fn get_or_create_placement(&self, handler_type: String, handler_id: String) -> String {
        let object_id = ObjectId(handler_type, handler_id);
        let placement_guard = self.object_placement_provider.read().await;
        let maybe_server_address = placement_guard.lookup(&object_id).await.take();
        drop(placement_guard);
        if let Some(server_address) = maybe_server_address {
            server_address
        } else {
            let new_placement = ObjectPlacement::new(object_id, Some(self.address.clone()));
            {
                self.object_placement_provider
                    .write()
                    .await
                    .update(new_placement)
                    .await;
            };
            self.address.clone()
        }
    }

    /// TODO
    async fn get_address_mismatch_error(&self, server_address: String) -> Option<ResponseError> {
        if server_address == self.address {
            return None;
        }

        // TODO error handling
        let mut split_address = server_address.split(':');
        let ip = split_address.next().expect("TODO: Address has no IP in it");
        let port = split_address
            .next()
            .expect("TODO: Address has no PORT in it");

        // TODO cache is_active response?
        let err = if self
            .members_storage
            .is_active(ip, port)
            .await
            .expect("TODO")
        {
            ResponseError::Redirect(server_address)
        } else {
            self.object_placement_provider
                .read()
                .await
                .clean_server(server_address)
                .await;
            ResponseError::DeallocateServiceObject
        };
        Some(err)
    }

    /// TODO
    ///
    /// TODO: Error
    async fn start_service_object(&self, handler_type: &str, handler_id: &str) -> Result<(), ()> {
        if !self
            .registry
            .read()
            .await
            .has(handler_type, handler_id)
            .await
        {
            let new_object = self
                .registry
                .read()
                .await
                .new_from_type(handler_type, handler_id.to_string())
                .expect("TODO: The type is not configured in the local registry (have you called `registry.add_type`?)");

            self.registry
                .read()
                .await
                .insert_boxed_object(handler_type.to_string(), handler_id.to_string(), new_object)
                .await;

            let _ = self
                .registry
                .read()
                .await
                .send(
                    handler_type,
                    handler_id,
                    "LifecycleMessage",
                    &bincode::serialize(&LifecycleMessage::Load).unwrap(),
                    self.app_data.clone(),
                )
                .await;
        }
        Ok(())
    }

    // TODO tune LenghtDelimitedCodec
    // TODO move this into a transport struct
    //
    /// Main service loop
    ///
    /// Consumes a stream of frames, each containing a command sent from clients.
    ///
    /// The commands might be either a request/response request or a subscription request
    pub async fn run(&mut self, stream: TcpStream) {
        let codec = LengthDelimitedCodec::new();
        let mut frames = Framed::new(stream, codec);

        while let Some(Ok(frame)) = StreamExt::next(&mut frames).await {
            let request: Result<RequestEnvelope, _> = bincode::deserialize(&frame);
            let subscription: Result<SubscriptionRequest, _> = bincode::deserialize(&frame);

            let either_request = match (request, subscription) {
                (Ok(message), _) => AllRequest::ReqResp(message),
                (_, Ok(message)) => AllRequest::PubSub(message),
                _ => {
                    panic!("TODO")
                }
            };
            match either_request {
                AllRequest::ReqResp(message) => {
                    let response = match self.call(message).await {
                        Ok(x) => x,
                        Err(err) => ResponseEnvelope::err(err),
                    };
                    let ser_response = bincode::serialize(&response).expect("TODO");
                    frames.send(ser_response.into()).await.unwrap();
                }
                AllRequest::PubSub(message) => {
                    let stream = self.call(message).await;

                    if let Err(err) = stream {
                        let response = SubscriptionResponse { body: Err(err) };
                        let ser_value = bincode::serialize(&response).expect("TODO");
                        frames.send(ser_value.into()).await.unwrap();
                        return;
                    }

                    let mut stream =
                        stream.expect("TODO improve error handling on the block above");

                    // TODO handle termination
                    while let Some(value) = StreamExt::next(&mut stream).await {
                        let ser_value = bincode::serialize(&value).expect("TODO");
                        frames.send(ser_value.into()).await.expect("TODO");
                    }
                }
            }
        }
    }
}

#[derive(Debug)]
enum AllRequest {
    ReqResp(RequestEnvelope),
    PubSub(SubscriptionRequest),
}

#[cfg(test)]
mod test {
    use std::time::Duration;

    use async_trait::async_trait;
    use rio_macros::{Message, TypeName, WithId};
    use serde::{Deserialize, Serialize};
    use tokio::time::timeout;
    use tower::ServiceExt;

    use super::*;
    use crate::cluster::storage::LocalStorage;
    use crate::object_placement::local::LocalObjectPlacementProvider;
    use crate::prelude::HandlerError;
    use crate::registry::Handler;

    #[derive(Default, WithId, TypeName)]
    #[rio_path = "crate"]
    struct MockService {
        id: String,
    }

    #[derive(Default, Debug, Message, TypeName, Serialize, Deserialize)]
    #[rio_path = "crate"]
    struct MockMessage {
        text: String,
    }

    #[derive(Default, Debug, Message, TypeName, Serialize, Deserialize)]
    #[rio_path = "crate"]
    struct MockResponse {
        text: String,
    }

    #[async_trait]
    impl Handler<MockMessage> for MockService {
        type Returns = MockResponse;
        async fn handle(
            &mut self,
            message: MockMessage,
            _: Arc<AppData>,
        ) -> Result<Self::Returns, HandlerError> {
            let resp = MockResponse {
                text: format!("{} received {}", self.id, message.text),
            };
            Ok(resp)
        }
    }

    fn svc() -> Service<LocalStorage, LocalObjectPlacementProvider> {
        let mut registry = Registry::new();
        registry.add_type::<MockService>();
        registry.add_handler::<MockService, MockMessage>();

        Service {
            address: "0.0.0.0:5000".to_string(),
            registry: Arc::new(RwLock::new(registry)),
            members_storage: LocalStorage::default(),
            object_placement_provider: Arc::new(RwLock::new(
                LocalObjectPlacementProvider::default(),
            )),
            app_data: Arc::new(AppData::new()),
        }
    }

    #[tokio::test]
    async fn test_poll_ready() {
        let mut svc = svc();
        ServiceExt::<RequestEnvelope>::ready(&mut svc)
            .await
            .expect("service ready");
    }

    #[tokio::test]
    async fn test_service_call() {
        let mut svc = svc();
        ServiceExt::<RequestEnvelope>::ready(&mut svc)
            .await
            .unwrap();

        let req = RequestEnvelope::new(
            "MockService".into(),
            "*".into(),
            "MockMessage".into(),
            bincode::serialize(&MockMessage { text: "hi".into() }).unwrap(),
        );
        let resp = svc.call(req).await.unwrap();
        let resp: MockResponse = bincode::deserialize(&resp.body.unwrap()).unwrap();
        assert_eq!(resp.text, "* received hi".to_string());
    }

    #[tokio::test]
    async fn test_service_subscription() {
        let mut svc = svc();
        ServiceExt::<SubscriptionRequest>::ready(&mut svc)
            .await
            .unwrap();

        let req = SubscriptionRequest {
            handler_type: "MockService".into(),
            handler_id: "*".into(),
        };
        let call_future = svc.call(req);
        let call_future = timeout(Duration::from_secs(3), call_future);
        let _stream = call_future.await.unwrap();
        // TODO assert_eq!(..., stream.next().await);
    }
}
