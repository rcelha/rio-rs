use std::marker::PhantomData;
use std::task::Poll;

use futures::future::BoxFuture;
use futures::{pin_mut, FutureExt, SinkExt, StreamExt};
use log::{error, info, warn};
use tower::Service as TowerService;

use crate::cluster::storage::MembersStorage;
use crate::protocol::RequestError;
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
    type Error = RequestError;
    type Future = BoxFuture<'a, Result<Self::Response, Self::Error>>;

    /// Waits for members to be available
    fn poll_ready(&mut self, cx: &mut std::task::Context<'_>) -> Poll<Result<(), Self::Error>> {
        let fetch_active_servers = self.client.fetch_active_servers();
        pin_mut!(fetch_active_servers);
        fetch_active_servers
            .poll_unpin(cx)
            .map_ok(|_| ())
            .map_err(|e| e.into())
    }

    /// TODO
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
                    Ok(message.body?)
                }
                Some(Err(e)) => Err(RequestError::ClientError(ClientError::IoError(
                    e.to_string(),
                ))),
                // When there are no more items on the stream, it means the TCP stream was
                // disconnected
                None => Err(RequestError::ClientError(ClientError::Disconnect)),
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
    type Error = RequestError;
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
                    Err(RequestError::ResponseError(ResponseError::Redirect(to))) => {
                        // Add the new address to the placement so in the next iteration
                        // it will use the right server
                        info!("Redirect to {}", to);
                        inner_service
                            .client
                            .placement
                            .write()
                            .map_err(|_| ClientError::PlacementLock)?
                            .put((handler_type.clone(), handler_id.clone()), to);
                    }
                    // Retry so it picks up a new Server on the cluster
                    Err(
                        RequestError::ResponseError(ResponseError::DeallocateServiceObject)
                        | RequestError::ClientError(ClientError::Disconnect)
                        | RequestError::ClientError(ClientError::ServerNotAvailable(_)),
                    ) => {
                        // Removed the old placement, the next request
                        // will pickup a new placement to try from
                        warn!("Refresh the list of servers");
                        warn!("{:?}", response.err());
                        inner_service.client.ts_active_servers_refresh = 0; // forces re-fetching the
                                                                            // active servers
                        inner_service
                            .client
                            .placement
                            .write()
                            .map_err(|_| ClientError::PlacementLock)?
                            .pop(&(handler_type.clone(), handler_id.clone()));
                    }
                    Err(e) => {
                        error!("Uncaught error {:#?}", e);
                        return Err(e);
                    }
                    // Return as is
                    rest => return rest,
                }
            }
        })
    }
}

#[cfg(test)]
mod test {
    use async_trait::async_trait;
    use chrono::{DateTime, Utc};
    use lru::LruCache;
    use std::sync::{Arc, RwLock};
    use tower::ServiceExt;

    use super::*;
    use crate::{
        cluster::storage::{
            local::LocalStorage, Member, MembersStorage, MembershipResult, MembershipUnitResult,
        },
        errors::MembershipError,
    };

    #[derive(Clone, Default)]
    struct FailMemberStorage {}

    #[async_trait]
    impl MembersStorage for FailMemberStorage {
        async fn push(&self, _: Member) -> MembershipUnitResult {
            Ok(())
        }
        async fn remove(&self, _: &str, _: &str) -> MembershipUnitResult {
            Ok(())
        }
        async fn set_is_active(&self, _: &str, _: &str, _: bool) -> MembershipUnitResult {
            Ok(())
        }
        async fn members(&self) -> MembershipResult<Vec<Member>> {
            Err(MembershipError::Unknown("".to_string()))
        }
        async fn notify_failure(&self, _: &str, _: &str) -> MembershipUnitResult {
            Ok(())
        }
        async fn member_failures(&self, _: &str, _: &str) -> MembershipResult<Vec<DateTime<Utc>>> {
            Ok(vec![])
        }
    }

    fn client() -> Client<LocalStorage> {
        Client {
            timeout_millis: 1000,
            members_storage: LocalStorage::default(),
            active_servers: None,
            ts_active_servers_refresh: 0,
            streams: Arc::default(),
            placement: Arc::new(RwLock::new(LruCache::new(10))),
        }
    }

    #[tokio::test]
    async fn test_poll_ready_no_active_server() {
        let client = client();
        let mut request = Request::new(client);
        request.ready().await.expect("poll_ready");
        assert!(request.client.active_servers.unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_poll_ready_with_active_servers() {
        // starts with no active servers
        let client = client();
        assert!(client.active_servers.is_none());

        let mut server = Member::new("0.0.0.0".to_string(), "1234".to_string());
        server.set_active(true);
        client
            .members_storage
            .push(server)
            .await
            .expect("add member");

        // When poll_ready is called, it fetches the active servers
        let mut request = Request::new(client);
        request.ready().await.expect("poll_ready");
        assert_eq!(
            request.client.active_servers.expect("active servers").len(),
            1
        );
    }

    #[tokio::test]
    async fn test_poll_ready_error() {
        let client = Client {
            timeout_millis: 1000,
            members_storage: FailMemberStorage {},
            active_servers: None,
            ts_active_servers_refresh: 0,
            streams: Arc::default(),
            placement: Arc::new(RwLock::new(LruCache::new(10))),
        };
        let mut request = Request::new(client);
        let waker = futures::task::noop_waker();
        let mut context = std::task::Context::from_waker(&waker);
        let poll_ready = request.poll_ready(&mut context);
        assert_eq!(
            poll_ready,
            Poll::Ready(Err(RequestError::ClientError(
                ClientError::RendevouzUnavailable
            )))
        );
    }
}
