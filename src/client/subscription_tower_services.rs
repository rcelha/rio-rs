use std::pin::Pin;

use std::task::{ready, Poll};

use futures::{Future, FutureExt, SinkExt};
use futures::{Stream, StreamExt};

use tokio::net::TcpStream;
use tokio_util::codec::{Framed, LengthDelimitedCodec};
use tower::Service;

use crate::cluster::storage::MembersStorage;
use crate::protocol::pubsub::{SubscriptionRequest, SubscriptionResponse};
use crate::protocol::ClientError;

use super::Client;

/// TowerService to retrieve stream of published messages from Rio server
#[derive(Clone)]
pub struct SubscriptionService<S>
where
    S: MembersStorage,
{
    client: Client<S>,
}

impl<S> SubscriptionService<S>
where
    S: MembersStorage,
{
    pub fn new(client: Client<S>) -> Self {
        SubscriptionService { client }
    }
}

impl<S> Service<SubscriptionRequest> for SubscriptionService<S>
where
    S: MembersStorage + 'static,
{
    type Response = SubscriptionStream;
    type Error = ClientError;
    type Future = SubscriptionStreamFuture;

    fn poll_ready(&mut self, _cx: &mut std::task::Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: SubscriptionRequest) -> Self::Future {
        let client = self.client.clone();
        let stream_future = SubscriptionStreamFuture::new(client, req);
        stream_future
    }
}

/// Future that returns a SubscriptionStream
pub struct SubscriptionStreamFuture {
    inner_future: Pin<Box<dyn Future<Output = Result<SubscriptionStream, ClientError>>>>,
}

impl SubscriptionStreamFuture {
    pub fn new<S>(mut client: Client<S>, request: SubscriptionRequest) -> Self
    where
        S: MembersStorage + 'static,
    {
        let handler_type = request.handler_type.clone();
        let handler_id = request.handler_id.clone();

        let inner_future = Box::pin(async move {
            let address_result = client
                .get_service_object_address(&handler_type, &handler_id)
                .await;
            let address = address_result?;

            let tcp_stream_result = client.pop_server_stream(&address).await;
            let mut tcp_stream = tcp_stream_result?;
            let ser_request = bincode::serialize(&request)
                .map_err(|err| ClientError::DeseralizationError(err.to_string()));
            let ser_request = ser_request?;
            let send_result = tcp_stream.send(ser_request.into()).await;
            send_result?;
            let sub_stream = SubscriptionStream::new(tcp_stream);
            Ok::<_, ClientError>(sub_stream)
        });

        SubscriptionStreamFuture { inner_future }
    }
}

impl Future for SubscriptionStreamFuture {
    type Output = Result<SubscriptionStream, ClientError>;

    fn poll(self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Self::Output> {
        let stream = ready!(self.get_mut().inner_future.poll_unpin(cx));
        Poll::Ready(stream)
    }
}

/// Wrapper to transform a Tcp stream into a stream of published messages
pub struct SubscriptionStream {
    tcp_stream: Framed<TcpStream, LengthDelimitedCodec>,
}

impl SubscriptionStream {
    pub fn new(tcp_stream: Framed<TcpStream, LengthDelimitedCodec>) -> Self {
        SubscriptionStream { tcp_stream }
    }
}

impl Stream for SubscriptionStream {
    type Item = Result<Vec<u8>, ClientError>;
    fn poll_next(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        let this = self.get_mut();
        let next = ready!(this.tcp_stream.poll_next_unpin(cx));
        if let Some(next) = next {
            let message_bytes = next?;
            let de_message: SubscriptionResponse = bincode::deserialize(&message_bytes)
                .map_err(|err| ClientError::DeseralizationError(err.to_string()))?;
            let body = de_message
                .body
                .map_err(|err| ClientError::ResponseError(err));
            Poll::Ready(Some(body))
        } else {
            Poll::Ready(None)
        }
    }
}
