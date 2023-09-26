use std::marker::PhantomData;
use std::task::Poll;

use futures::future::BoxFuture;
use futures::FutureExt;
use futures::SinkExt;
use futures::StreamExt;
use tower::Service as TowerService;

use crate::cluster::storage::MembersStorage;
use crate::protocol::{ClientError, RequestEnvelope, ResponseEnvelope, ResponseError};

use super::Client;

/// Requests have only a single response from the server (for streaming back result, see **TODO**)
///
/// This contains the [Client] because it does
#[derive(Clone)]
pub struct Request<'a, S>
where
    S: MembersStorage,
{
    client: Client<S>,
    _phantom: PhantomData<&'a ()>,
}

impl<'a, S> Request<'a, S>
where
    S: MembersStorage,
{
    pub fn new(client: Client<S>) -> Self {
        Request {
            client,
            _phantom: PhantomData,
        }
    }
}

impl<'a, S> TowerService<RequestEnvelope> for Request<'a, S>
where
    S: MembersStorage + 'static, // TODO remove 'static
{
    type Response = Vec<u8>;
    type Error = ClientError;
    type Future = BoxFuture<'a, Result<Self::Response, Self::Error>>;

    /// Waits for members to be available
    ///
    /// <div class="warning">
    /// TODO
    ///
    /// Call `client.fetch_active_servers` in `poll_ready`
    /// </div>
    fn poll_ready(&mut self, cx: &mut std::task::Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.client
            .members_storage
            .members()
            .poll_unpin(cx)
            .map_ok(|_| ())
            .map_err(|_| ClientError::RendevouzUnavailable)
    }

    ///
    fn call(&mut self, req: RequestEnvelope) -> Self::Future {
        let mut client = self.client.clone();
        Box::pin(async move {
            let mut stream = client
                .service_object_stream(&req.handler_type, &req.handler_id)
                .await?;

            let ser_request = bincode::serialize(&req)
                .map_err(|e| ClientError::SeralizationError(e.to_string()))?;

            stream.send(ser_request.into()).await?;
            match stream.next().await {
                Some(Ok(frame)) => {
                    let message: ResponseEnvelope = bincode::deserialize(&frame)
                        .map_err(|e| ClientError::DeseralizationError(e.to_string()))?;

                    match message.body {
                        Ok(v) => Ok(v),
                        Err(err) => Err(ClientError::ResponseError(err)),
                    }
                }
                // TODO: Add more granularity to ClientError
                Some(Err(e)) => Err(ClientError::Unknown(e.to_string())),
                None => Err(ClientError::Unknown("Unknown error".to_string())),
            }
        })
    }
}

/// This type wraps a [Request], and it retries its call under some conditions:
///
/// - When the object is not on the cached/expected placement
/// - When the object is not yet allocated
pub struct RequestRedirect<'a, S>
where
    S: MembersStorage,
    Request<'a, S>: Clone,
{
    inner: Request<'a, S>,
}

impl<'a, S> RequestRedirect<'a, S>
where
    S: MembersStorage,
    Request<'a, S>: Clone,
{
    pub fn new(inner: Request<'a, S>) -> Self {
        RequestRedirect { inner }
    }
}

impl<'a, S> TowerService<RequestEnvelope> for RequestRedirect<'a, S>
where
    S: MembersStorage + 'static, // TODO remove 'static
    Request<'a, S>: Clone,
{
    type Response = Vec<u8>;
    type Error = ClientError;
    type Future = BoxFuture<'a, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, cx: &mut std::task::Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    /// <div class="warning">There are tons of extra allocations to avoid race conditions</div>
    ///
    /// <div class="warning">
    /// This method works over a clone of [Request], we need to make sure
    /// it keeps in sync with the originial version
    /// </div>
    fn call(&mut self, req: RequestEnvelope) -> Self::Future {
        // Clones a bunch of stuff so the future returned by this
        // function lives shorter than the actual service (as in `'b: 'a`)
        let handler_type = req.handler_type.clone();
        let handler_id = req.handler_id.clone();
        let request = req.clone();
        let mut inner_service = self.inner.clone();

        Box::pin(async move {
            loop {
                // Used a cloned request, so it can be used in a loop
                let response = inner_service.call(request.clone()).await;
                match response {
                    Err(ClientError::ResponseError(ResponseError::Redirect(to))) => {
                        // Add the new address to the placement so in the next iteration
                        // it will use the right server
                        inner_service
                            .client
                            .placement
                            .write()
                            .map_err(|_| ClientError::PlacementLock)?
                            .put((handler_type.clone(), handler_id.clone()), to);
                    }
                    // Retry so it picks up a new Server on the cluster
                    Err(ClientError::ResponseError(ResponseError::DeallocateServiceObject)) => {
                        // Removed the old placement, the next request
                        // will pickup a new placement to try from
                        inner_service
                            .client
                            .placement
                            .write()
                            .map_err(|_| ClientError::PlacementLock)?
                            .pop(&(handler_type.clone(), handler_id.clone()));
                    }
                    // Return as is
                    rest => return rest,
                }
            }
        })
    }
}
