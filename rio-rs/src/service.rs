//! Server services

use futures::future::BoxFuture;
use futures::sink::SinkExt;
use futures::{FutureExt, Stream, StreamExt};
use log::error;
use std::panic::AssertUnwindSafe;
use std::sync::Arc;

use tokio::net::TcpStream;
use tokio::sync::RwLock;
use tokio_util::codec::{Framed, LengthDelimitedCodec};
use tower::Service as TowerService;

use crate::app_data::{AppData, AppDataExt};
use crate::cluster::storage::MembershipStorage;
use crate::message_router::MessageRouter;
use crate::object_placement::{ObjectPlacement, ObjectPlacementItem};
use crate::protocol::pubsub::{SubscriptionRequest, SubscriptionResponse};
use crate::protocol::{RequestEnvelope, ResponseEnvelope, ResponseError};
use crate::registry::Registry;
use crate::{LifecycleMessage, ObjectId};

/// Service to respond to Requests from [crate::client::Client]
#[derive(Clone)]
pub struct Service<S: MembershipStorage, P: ObjectPlacement> {
    pub(crate) address: String,
    pub(crate) registry: Arc<RwLock<Registry>>,
    pub(crate) members_storage: S,
    pub(crate) object_placement_provider: Arc<RwLock<P>>,
    pub(crate) app_data: Arc<AppData>,
}

/// Service implementation to handle [RequestEnvelope] request
impl<S: MembershipStorage + 'static, P: ObjectPlacement + 'static> TowerService<RequestEnvelope>
    for Service<S, P>
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

    /// Call a service locally, or return an error that will
    /// indicate whether this service is allocated somewhere
    /// else
    fn call(&mut self, req: RequestEnvelope) -> Self::Future {
        let this = self.clone();
        let result = async move {
            // Test if this object is in fact allocated in this instance
            let server_address = this
                .get_or_create_placement(req.handler_type.clone(), req.handler_id.clone())
                .await;
            this.check_address_mismatch(server_address).await?;

            // Ensure the object is started in the registry
            this.start_service_object(&req.handler_type, &req.handler_id)
                .await
                .map_err(|err| {
                    // Transform some internal error types into better user facing errors
                    // while retaining other error types
                    match err {
                        ResponseError::Unknown(_) => ResponseError::Allocate,
                        e => e,
                    }
                })?;

            // Req + Response to registry
            let guard = this.registry.read().await;
            let fut = guard.send(
                &req.handler_type,
                &req.handler_id,
                &req.message_type,
                &req.payload,
                this.app_data.clone(),
            );
            // TODO review the use of `catch_unwind` and `AssertUnwindSafe`
            let fut = AssertUnwindSafe(fut);
            let response = fut.catch_unwind().await;

            // Handle result, 'translating' it to the protocol
            match response {
                Ok(Ok(body)) => Ok(ResponseEnvelope::new(body)),
                Ok(Err(err)) => Err(ResponseError::from(err)),
                Err(_) => {
                    // When there is a panic, we will 'remove' the service object
                    // from both the registry and the ObjectPlacement
                    this.registry
                        .read()
                        .await
                        .remove(req.handler_type.clone(), req.handler_id.clone())
                        .await;
                    this.object_placement_provider
                        .read()
                        .await
                        .remove(&ObjectId(req.handler_type.clone(), req.handler_id.clone()))
                        .await;
                    Err(ResponseError::Unknown("Panic".to_string()))
                }
            }
        };
        Box::pin(result)
    }
}

/// This is a iterator to be used on the server to stream
/// messages back to the client
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
                if result.is_err() {
                    error!("Error on stream recv {:?}", result);
                }
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
    S: MembershipStorage + 'static,
    P: ObjectPlacement + 'static,
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

            this.check_address_mismatch(server_address).await?;
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

impl<S: MembershipStorage + 'static, P: ObjectPlacement + 'static> Service<S, P> {
    /// Returns the ip:port for where this object is placed
    ///
    /// If the object is not instantiated anywhere, it will allocate locally
    async fn get_or_create_placement(&self, handler_type: String, handler_id: String) -> String {
        let object_id = ObjectId(handler_type, handler_id);
        let placement_guard = self.object_placement_provider.read().await;
        let mut maybe_server_address = placement_guard.lookup(&object_id).await.take();
        drop(placement_guard);

        // Ensures the placement is on an active server
        if let Some(server_address) = maybe_server_address.as_ref() {
            let mut addr_split = server_address.splitn(2, ":");
            let ip = addr_split.next().unwrap_or_default();
            let port = addr_split.next().unwrap_or_default();

            // This case should never happen, but writing it here to be handled gracefuly.
            // It means the placement was stored with bad data, so we remove the record and
            // take `maybe_server_address` so it picks up a new placement at the end of this
            // function
            if ip.is_empty() || port.is_empty() {
                error!(object_id:? = object_id,
                       address:%  = server_address,
                       ip:%  = ip,
                       port:%  = port;
                       "The object's placement is in a bad state. This is likely a bug on the object placement code");

                let placement_guard = self.object_placement_provider.read().await;
                placement_guard.remove(&object_id).await;
                maybe_server_address.take();
            }
            // In case the server in which this object is allocated is inactive/unavailable,
            // we clean up the server (disassociate all the objects from it), and take
            // `maybe_server_address` so it picks up a new placement at the end of this function
            else if !self
                .members_storage
                .is_active(ip, port)
                .await
                .unwrap_or(false)
            {
                let placement_guard = self.object_placement_provider.read().await;
                placement_guard
                    .clean_server(server_address.to_string())
                    .await;
                maybe_server_address.take();
            }
        }

        if let Some(server_address) = maybe_server_address {
            server_address
        } else {
            let new_placement = ObjectPlacementItem::new(object_id, Some(self.address.clone()));
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

    /// Checks if the given address is from the local server.
    /// There are various checks that needs to run.
    ///
    /// It returns an Error if it is not
    async fn check_address_mismatch(&self, server_address: String) -> Result<(), ResponseError> {
        if server_address == self.address {
            return Ok(());
        }

        let mut split_address = server_address.split(':');
        let ip = split_address.next().ok_or_else(|| {
            ResponseError::Unknown(format!(
                "Malformed address: Missing IP in '{}'",
                server_address
            ))
        })?;
        let port = split_address.next().ok_or_else(|| {
            ResponseError::Unknown(format!(
                "Malformed address: Missing PORT in '{}'",
                server_address
            ))
        })?;

        let is_active = self
            .members_storage
            .is_active(ip, port)
            .await
            .map_err(|e| ResponseError::Unknown(e.to_string()))?;

        // This object is active somewhere else
        if is_active {
            return Err(ResponseError::Redirect(server_address));
        }

        // This object is not allocated here, and it is not active either
        self.object_placement_provider
            .read()
            .await
            .clean_server(server_address)
            .await;
        Err(ResponseError::DeallocateServiceObject)
    }

    /// Startup a service object and insert it into registry
    ///
    /// If is already running, ignore it
    async fn start_service_object(
        &self,
        handler_type: &str,
        handler_id: &str,
    ) -> Result<(), ResponseError> {
        // Allocate holding the same read lock as the test, it ensures there is no ongoing write
        {
            let registry_guard = self.registry.read().await;
            if registry_guard.has(handler_type, handler_id).await {
                return Ok(());
            }

            let new_object = registry_guard
                .new_from_type(handler_type, handler_id.to_string())
                .ok_or(ResponseError::NotSupported(handler_type.to_string()))?;

            registry_guard
                .insert_boxed_object(handler_type.to_string(), handler_id.to_string(), new_object)
                .await;
        };

        let lifecycle_result = {
            let object_guard = self.registry.read().await;
            let lifecycle_msg = LifecycleMessage::Load;
            let lifecycle_ser_msg = bincode::serialize(&lifecycle_msg).expect("TODO");
            let lifecycle_fut = object_guard.send(
                handler_type,
                handler_id,
                "LifecycleMessage",
                &lifecycle_ser_msg,
                self.app_data.clone(),
            );

            // Catch panics on LifecycleMessage::Load
            let lifecycle_fut = AssertUnwindSafe(lifecycle_fut);
            lifecycle_fut.catch_unwind().await
        };

        // TODO remove duplicated logic (Self::send)
        if let Err(e) = lifecycle_result {
            self.registry
                .read()
                .await
                .remove(handler_type.to_string(), handler_id.to_string())
                .await;
            self.object_placement_provider
                .read()
                .await
                .remove(&ObjectId(handler_type.to_string(), handler_id.to_string()))
                .await;

            return Err(ResponseError::Unknown(format!("Task panicked: {:?}", e)));
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
                    unreachable!("Got both or neither requests")
                }
            };
            match either_request {
                AllRequest::ReqResp(message) => {
                    let response = match self.call(message).await {
                        Ok(x) => x,
                        Err(err) => ResponseEnvelope::err(err),
                    };
                    let ser_result = bincode::serialize(&response);
                    let ser_response = match ser_result {
                        Ok(value) => value,
                        Err(err) => {
                            let new_return = ResponseEnvelope::err(
                                ResponseError::SeralizationError(err.to_string()),
                            );
                            bincode::serialize(&new_return)
                                .expect("Serialization of response error should be infalible")
                        }
                    };
                    frames.send(ser_response.into()).await.unwrap();
                }
                AllRequest::PubSub(message) => {
                    let stream = self.call(message).await;

                    // If there is an upstream error to establish the subscription,
                    // wrapi it in a SubscriptionResponse and return earlier
                    let mut stream = match stream {
                        Ok(value) => value,
                        Err(err) => {
                            let sub_response = SubscriptionResponse::err(err);
                            let ser_response = bincode::serialize(&sub_response)
                                .expect("Error serialization should be infalible");
                            frames.send(ser_response.into()).await.ok();
                            return;
                        }
                    };

                    while let Some(value) = StreamExt::next(&mut stream).await {
                        let ser_result = bincode::serialize(&value);
                        let ser_response = match ser_result {
                            Ok(value) => value,
                            Err(err) => {
                                let new_return = SubscriptionResponse::err(
                                    ResponseError::SeralizationError(err.to_string()),
                                );
                                bincode::serialize(&new_return)
                                    .expect("Serialization of response error should be infalible")
                            }
                        };

                        let send_result = frames.send(ser_response.into()).await;

                        // Stop receiving messages if the sink we redirect messages to is
                        // closed
                        if let Err(err) = send_result {
                            error!("Channel is closed due {}", err);
                            break;
                        }
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
    use crate::cluster::storage::local::LocalStorage;
    use crate::object_placement::local::LocalObjectPlacement;

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
        type Error = ();
        async fn handle(
            &mut self,
            message: MockMessage,
            _: Arc<AppData>,
        ) -> Result<Self::Returns, Self::Error> {
            let resp = MockResponse {
                text: format!("{} received {}", self.id, message.text),
            };
            Ok(resp)
        }
    }

    fn svc() -> Service<LocalStorage, LocalObjectPlacement> {
        let mut registry = Registry::new();
        registry.add_type::<MockService>();
        registry.add_handler::<MockService, MockMessage>();

        Service {
            address: "0.0.0.0:5000".to_string(),
            registry: Arc::new(RwLock::new(registry)),
            members_storage: LocalStorage::default(),
            object_placement_provider: Arc::new(RwLock::new(LocalObjectPlacement::default())),
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
