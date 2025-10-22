use std::marker::PhantomData;
use std::task::Poll;
use std::time::Duration;

use futures::future::BoxFuture;
use futures::{pin_mut, FutureExt, SinkExt, StreamExt};
use log::{debug, error, info, warn};
use serde::de::DeserializeOwned;
use tower::Service as TowerService;

use crate::cluster::storage::MembershipStorage;
use crate::protocol::RequestError;
use crate::protocol::{ClientError, RequestEnvelope, ResponseEnvelope, ResponseError};

use super::Client;

/// Requests have only a single response from the server (for streaming back result, see [crate::client::Client::subscribe])
///
/// This contains the [Client] because it does
///
/// - `'a` is the lifetime for the tower service's box future
/// - `S` is the MembershipStorage for the internal client
/// - `E` is the generic for the RequestError
#[derive(Clone)]
pub struct Request<'a, S, E> {
    client: Client<S>,
    _lifetime_marker: PhantomData<&'a ()>,
    _error_marker: PhantomData<E>,
}

impl<'a, S, E> Request<'a, S, E>
where
    S: MembershipStorage,
{
    pub fn new(client: Client<S>) -> Self {
        Request {
            client,
            _lifetime_marker: PhantomData,
            _error_marker: PhantomData,
        }
    }
}

impl<'a, S, E: std::error::Error + DeserializeOwned> TowerService<RequestEnvelope>
    for Request<'a, S, E>
where
    S: MembershipStorage + 'static, // TODO remove 'static
{
    type Response = Vec<u8>;
    type Error = RequestError<E>;
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
pub struct RequestRedirect<'a, S, E>
where
    S: MembershipStorage,
    Request<'a, S, E>: Clone,
{
    inner: Request<'a, S, E>,
}

impl<'a, S, E> RequestRedirect<'a, S, E>
where
    S: MembershipStorage,
    Request<'a, S, E>: Clone,
{
    pub fn new(inner: Request<'a, S, E>) -> Self {
        RequestRedirect { inner }
    }
}

impl<'a, S, E> TowerService<RequestEnvelope> for RequestRedirect<'a, S, E>
where
    E: std::error::Error + DeserializeOwned + Send + Sync + 'a,
    S: MembershipStorage + 'static,
    Request<'a, S, E>: Clone,
{
    type Response = Vec<u8>;
    type Error = RequestError<E>;
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

        // TODO move this to config
        let retry_min_duration = Duration::from_nanos(1_000); // 0.01ms
        let retry_max_duration = Duration::from_secs(2);
        let max_retries = Some(20);
        // END: TODO move this to config

        let mut retry_count = 0;
        let mut retry_duration = retry_min_duration.clone();
        Box::pin(async move {
            loop {
                // Used a cloned request, so it can be used in a loop
                let response = inner_service.call(request.clone()).await;
                match response {
                    // This case happens when there is a mismatch between the client and the
                    // servers regarding where the service object is allocated
                    // Ps.: No need to sleep on redirect
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
                    // All these errors indicate that the server we've tried is no longer available
                    // When facing one of the errors below, we need to retry the
                    // request so it picks up a new Server on the cluster
                    Err(e)
                        if matches!(
                            e,
                            RequestError::ResponseError(ResponseError::DeallocateServiceObject,)
                                | RequestError::ClientError(ClientError::Disconnect)
                                | RequestError::ClientError(ClientError::ServerNotAvailable(_))
                                | RequestError::ClientError(ClientError::IoError(_))
                        ) =>
                    {
                        // early quiting if max_retries reached
                        if let Some(max_retries) = max_retries {
                            if retry_count > max_retries {
                                error!("Max retries ({}) reached: {:?}", max_retries, e);
                                return Err(e);
                            }
                        }
                        warn!("{:?}", e);

                        // update retry info
                        debug!("Retry in {:?}", retry_duration);
                        tokio::time::sleep(retry_duration).await;
                        retry_count += 1;
                        retry_duration *= 2;
                        retry_duration =
                            retry_duration.clamp(retry_min_duration, retry_max_duration);

                        // Removed the old placement, the next request
                        // will pickup a new placement to try from
                        warn!("Refresh the list of servers");
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
                        // Have a separate case to avoid spamming error logs with the application
                        // error in binary format
                        if let RequestError::ResponseError(ResponseError::ApplicationError(_)) = e {
                            error!("Uncaught error ResponseError::ApplicationError(...)");
                        } else {
                            error!("Uncaught error {:#?}", e);
                        }
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
    use serde::{Deserialize, Serialize};
    use std::sync::{Arc, RwLock};
    use thiserror::Error;
    use tower::ServiceExt;

    use super::*;
    use crate::{
        cluster::storage::{
            local::LocalStorage, Member, MembershipResult, MembershipStorage, MembershipUnitResult,
        },
        errors::MembershipError,
    };

    #[derive(Error, Debug, Serialize, Deserialize, PartialEq)]
    enum NoopError {
        #[error("No-op")]
        Noop,
    }

    #[derive(Clone, Default)]
    struct FailMembershipStorage {}

    #[async_trait]
    impl MembershipStorage for FailMembershipStorage {
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
            membership_storage: LocalStorage::default(),
            active_servers: Default::default(),
            ts_active_servers_refresh: 0,
            streams: Arc::default(),
            placement: Arc::new(RwLock::new(LruCache::new(10))),
        }
    }

    #[tokio::test]
    async fn test_poll_ready_no_active_server() {
        let client = client();
        let mut request: Request<_, NoopError> = Request::new(client);
        request.ready().await.expect("poll_ready");
        assert!(request.client.active_servers.is_empty());
    }

    #[tokio::test]
    async fn test_poll_ready_with_active_servers() {
        // starts with no active servers
        let client = client();
        assert!(client.active_servers.is_empty());

        let mut server = Member::new("0.0.0.0".to_string(), "1234".to_string());
        server.set_active(true);
        client
            .membership_storage
            .push(server)
            .await
            .expect("add member");

        // When poll_ready is called, it fetches the active servers
        let mut request: Request<_, NoopError> = Request::new(client);
        request.ready().await.expect("poll_ready");
        assert_eq!(request.client.active_servers.len(), 1);
    }

    #[tokio::test]
    async fn test_poll_ready_error() {
        let client = Client {
            timeout_millis: 1000,
            membership_storage: FailMembershipStorage {},
            active_servers: Default::default(),
            ts_active_servers_refresh: 0,
            streams: Arc::default(),
            placement: Arc::new(RwLock::new(LruCache::new(10))),
        };
        let mut request: Request<_, NoopError> = Request::new(client);
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
