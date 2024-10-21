//! Talk to a rio-rs server
//!
//! Provides a client to interact with a cluster for both request/response and pub/sub
//!
//! There is a pooled client.
//! The client also does proper placement lookups and controls its own
//! caching strategy

mod builder;
mod pool;
pub mod tower_services;

use async_stream::stream;
pub use builder::ClientBuilder;
pub use pool::ClientConnectionManager;

use dashmap::mapref::one::RefMut;
use dashmap::DashMap;
use futures::SinkExt;
use futures::{Stream, StreamExt};
use lru::LruCache;
use rand::{prelude::SliceRandom, thread_rng};
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::collections::HashSet;
use std::marker::PhantomData;
use std::sync::{Arc, RwLock};
use tokio::net::TcpStream;
use tokio::time::timeout;
use tokio_util::codec::{Framed, LengthDelimitedCodec};
use tower::Service as TowerService;

use crate::cluster::storage::MembersStorage;
use crate::protocol::pubsub::{SubscriptionRequest, SubscriptionResponse};
use crate::protocol::{ClientError, RequestEnvelope, ResponseError};
use crate::registry::IdentifiableType;

pub const DEFAULT_TIMEOUT_MILLIS: u64 = 500;

/// Client struct to interact with a cluster for requests and subscriptions
#[derive(Clone)]
pub struct Client<S>
where
    S: MembersStorage,
{
    timeout_millis: u64,

    /// Membership view used for Server's service discovery
    members_storage: S,

    /// List of servers that are accepting requests
    active_servers: Option<HashSet<String>>,

    /// Timestamp of the last time self.active_servers was refresh
    ts_active_servers_refresh: u64,

    /// Framed TCP Stream mapped by ip+port address
    streams: Arc<DashMap<String, Framed<TcpStream, LengthDelimitedCodec>>>,

    /// TODO
    placement: Arc<RwLock<LruCache<(String, String), String>>>,
}

/// Stream of subscription messages. This is used for pub/sub.
pub struct SubscriptionStream<T>
where
    T: DeserializeOwned,
{
    // TODO make this over an impl G instead of Framed
    pub tcp_stream: Framed<TcpStream, LengthDelimitedCodec>,
    _phantom: PhantomData<T>,
}

impl<T> SubscriptionStream<T>
where
    T: DeserializeOwned,
{
    pub fn new(tcp_stream: Framed<TcpStream, LengthDelimitedCodec>) -> Self {
        SubscriptionStream {
            tcp_stream,
            _phantom: PhantomData {},
        }
    }
}

/// <div class="warning">
/// Remove Unpin
/// </div>
impl<T> Stream for SubscriptionStream<T>
where
    T: DeserializeOwned + std::marker::Unpin + std::fmt::Debug,
{
    type Item = Result<T, ResponseError>;
    fn poll_next(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        let self_mut = self.get_mut();
        self_mut.tcp_stream.poll_next_unpin(cx).map(|maybe_bytes| {
            let bytes_result = maybe_bytes?;
            let bytes_message = match bytes_result {
                Ok(bytes_message) => bytes_message,
                Err(err) => {
                    return Some(Err(ResponseError::DeseralizationError(err.to_string())));
                }
            };

            let sub_response: SubscriptionResponse = match bincode::deserialize(&bytes_message) {
                Ok(sub_response) => sub_response,
                Err(err) => return Some(Err(ResponseError::DeseralizationError(err.to_string()))),
            };

            let final_message = match sub_response.body {
                Ok(v) => {
                    let response: Result<T, _> = bincode::deserialize(&v)
                        .map_err(|e| ResponseError::DeseralizationError(e.to_string()));
                    response
                }
                Err(err) => Err(err),
            };
            Some(final_message)
        })
    }
}

type ClientResult<T> = Result<T, ClientError>;

impl<S> Client<S>
where
    S: 'static + MembersStorage,
{
    /// TODO
    pub fn new(members_storage: S) -> Self {
        Client {
            members_storage,
            timeout_millis: DEFAULT_TIMEOUT_MILLIS,
            active_servers: None,
            ts_active_servers_refresh: 0,
            streams: Arc::default(),
            placement: Arc::new(RwLock::new(LruCache::new(1000))),
        }
    }

    /// Fetch a list of active servers if it hasn't done yet, or if the current list is too old
    /// TODO
    async fn fetch_active_servers(&mut self) -> ClientResult<()> {
        if self.active_servers.is_some() && self.ts_active_servers_refresh > 0 {
            return Ok(());
        }

        let active_servers: HashSet<String> = self
            .members_storage
            .active_members()
            .await
            .map_err(|_| ClientError::RendevouzUnavailable)?
            .iter()
            .map(|member| member.address())
            .collect();

        self.active_servers = Some(active_servers);
        self.ts_active_servers_refresh = 1;
        Ok(())
    }

    async fn ensure_stream_exists(&mut self, address: &str) -> ClientResult<()> {
        self.fetch_active_servers().await?;

        // We start this method fetching the active servers, so if there are no active servers we
        // fail
        //
        // If we do have items but the asked address is not there, the active_servers might be
        // outdated and it will reset the refresh time and fetch it again
        match &self.active_servers {
            None => return Err(ClientError::NoServersAvailable),
            Some(active_servers) => {
                if active_servers.is_empty() {
                    return Err(ClientError::NoServersAvailable);
                }

                if !active_servers.contains(address) {
                    self.ts_active_servers_refresh = 0;
                    self.fetch_active_servers().await?;
                }
            }
        };

        // After fetch and re-fetch, if the asked address is not on the list, it means the caller
        // is outdated
        match &self.active_servers {
            None => return Err(ClientError::NoServersAvailable),
            Some(active_servers) => {
                if !active_servers.contains(address) {
                    return Err(ClientError::ServerNotAvailable(address.to_string()));
                }
            }
        };

        // If there are no stream for the address, create a new one
        // This is on a nested block so it controlls the guards in `self.stream`
        if self.streams.get(address).is_none() {
            let stream = TcpStream::connect(&address)
                .await
                .map_err(|x| ClientError::Unknown(format!("(Address {}) - {:?}", address, x)))?;
            let stream = Framed::new(stream, LengthDelimitedCodec::new());
            self.streams.insert(address.to_string(), stream);
        };
        Ok(())
    }

    /// Get an existing connection to server `address` or create a new one
    ///
    /// If the address is not one of the known online servers, it will fetch
    /// the list of active servers again
    ///
    /// TODO can I change this to read only?
    async fn server_stream(
        &mut self,
        address: &String,
    ) -> ClientResult<RefMut<'_, String, Framed<TcpStream, LengthDelimitedCodec>>> {
        self.ensure_stream_exists(address).await?;
        self.streams
            .get_mut(address)
            .ok_or(ClientError::Connectivity)
    }

    /// same as server_stream, but it pops from the stream cache
    ///
    /// <div class="warning">
    /// TODO - Remove copy and pasta
    /// </div>
    ///
    async fn pop_server_stream(
        &mut self,
        address: &String,
    ) -> ClientResult<Framed<TcpStream, LengthDelimitedCodec>> {
        self.ensure_stream_exists(address).await?;
        self.streams
            .remove(address)
            .map(|(_, v)| v)
            .ok_or(ClientError::Connectivity)
    }

    /// Returns the address for a given service object
    async fn get_service_object_address(
        &mut self,
        service_object_type: impl ToString,
        service_object_id: impl ToString,
    ) -> ClientResult<String> {
        self.fetch_active_servers().await?;
        let object_id = (
            service_object_id.to_string(),
            service_object_type.to_string(),
        );
        let address = {
            let mut placement_guard = self
                .placement
                .write()
                .map_err(|_| ClientError::PlacementLock)?;

            let cached_address = placement_guard.get(&object_id);
            match cached_address {
                Some(address) => address.clone(),
                None => {
                    let mut rng = thread_rng();
                    let servers: Vec<String> = self
                        .active_servers
                        .as_ref()
                        .ok_or(ClientError::NoServersAvailable)?
                        .iter()
                        .cloned()
                        .collect();
                    let random_server = servers
                        .choose(&mut rng)
                        .ok_or(ClientError::NoServersAvailable)?;
                    random_server.clone()
                }
            }
        };
        Ok(address)
    }

    /// Returns a stream to the server that a given ServiceObject might be allocated into
    /// TODO can I change this to read only?
    async fn service_object_stream(
        &mut self,
        service_object_type: impl ToString,
        service_object_id: impl ToString,
    ) -> ClientResult<RefMut<'_, String, Framed<TcpStream, LengthDelimitedCodec>>> {
        self.fetch_active_servers().await?;
        let address = self
            .get_service_object_address(service_object_type, service_object_id)
            .await?;
        self.server_stream(&address).await
    }

    /// Send a request to the cluster transparently (the caller doesn't need to know where the
    /// object is placed)
    ///
    ///
    /// TODO -  When the cached or selected server are not available, it needs to refresh all the
    ///         cache and try a different server, this process needs to repeat until it finds a new
    ///         available server
    pub async fn send<T>(
        &mut self,
        handler_type: impl AsRef<str>,
        handler_id: impl AsRef<str>,
        payload: &(impl Serialize + IdentifiableType + Send + Sync),
    ) -> Result<T, ClientError>
    where
        T: DeserializeOwned,
    {
        // TODO move fetch_active_servers into poll_ready self.ready().await?;
        self.fetch_active_servers().await?;

        let handler_type = handler_type.as_ref().to_string();
        let handler_id = handler_id.as_ref().to_string();
        let ser_payload = bincode::serialize(&payload)
            .map_err(|e| ClientError::SeralizationError(e.to_string()))?;
        let message_type = payload.instance_type_id().to_string();

        let request = RequestEnvelope::new(
            handler_type.clone(),
            handler_id.clone(),
            message_type.clone(),
            ser_payload.clone(),
        );
        let tower_svc = tower_services::Request::new(self.clone());
        let mut tower_svc = tower_services::RequestRedirect::new(tower_svc);
        let response = tower_svc.call(request).await;
        response.and_then(|x| {
            let body: T = bincode::deserialize(&x)
                .map_err(|e| ClientError::DeseralizationError(e.to_string()))?;
            Ok(body)
        })
    }

    async fn _subscribe<'a, T>(
        &'a mut self,
        handler_type: &str,
        handler_id: &str,
        address: &str,
    ) -> SubscriptionStream<T>
    where
        Self: 'a,
        T: DeserializeOwned + std::marker::Unpin + 'a + std::fmt::Debug,
    {
        let mut svc_stream = self.pop_server_stream(&address.to_string()).await.unwrap();
        let req = SubscriptionRequest {
            handler_type: handler_type.to_string(),
            handler_id: handler_id.to_string(),
        };
        let ser_request = bincode::serialize(&req).unwrap();
        svc_stream.send(ser_request.into()).await.unwrap();
        SubscriptionStream::<T>::new(svc_stream)
    }

    /// Subscribe to events from a service object
    ///
    /// <div class="warning">
    /// TODO
    ///
    /// - [x] Returns async iter
    /// - [x] Handle redirects
    /// - [ ] Move this logic into a tower service
    /// - [ ] Support moving service object (after you connect to a node and the handler you are
    ///       listening to moves to some other node)
    /// - [x] Use dedicated connection
    ///
    /// </div>
    pub async fn subscribe<'a, T>(
        &'a mut self,
        handler_type: impl AsRef<str>,
        handler_id: impl AsRef<str>,
    ) -> impl Stream<Item = Result<T, ClientError>> + 'a
    where
        Self: 'a,
        T: DeserializeOwned + std::marker::Unpin + 'a + std::fmt::Debug,
    {
        let handler_type = handler_type.as_ref().to_string();
        let handler_id = handler_id.as_ref().to_string();
        let address = self
            .get_service_object_address(&handler_type, &handler_id)
            .await;

        stream! {
            // TODO test this
            let mut address = match address {
                Ok(v) => v,
                Err(e) => {
                    yield Err(e);
                    return;
                }
            };

            loop {
                let mut subscription_stream = self._subscribe(&handler_type, &handler_id, &address).await;
                while let Some(v) = subscription_stream.next().await {
                    if let Err(ResponseError::Redirect(to)) = v {
                        address = to;
                        break;
                    }
                    let i = match v {
                        Ok(v) => Ok(v),
                        Err(err) => Err(ClientError::ResponseError(err)),

                    };
                    yield i;
                }
            }
        }
    }

    /// Connects to a the first server of the MembersStorage
    ///
    /// This is used mostly by the PeerToPeerClusterProvider to check whether
    /// a set of servers is reacheable and alive
    pub async fn ping(&mut self) -> Result<(), ClientError> {
        let servers = self
            .members_storage
            .members()
            .await
            .map_err(|_| ClientError::Connectivity)?;
        let server = servers.first().ok_or(ClientError::NoServersAvailable)?;

        async fn conn(address: &str) -> Result<(), ClientError> {
            TcpStream::connect(&address)
                .await
                .map(|_stream| Ok(()))
                .map_err(|_e| ClientError::Connectivity)?
        }

        match timeout(
            std::time::Duration::from_millis(self.timeout_millis),
            conn(&server.address()),
        )
        .await
        {
            Ok(x) => x,
            Err(_elapsed) => Err(ClientError::Connectivity),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{
        cluster::storage::{
            local::LocalStorage, Member, MembersStorage, MembershipResult, MembershipUnitResult,
        },
        errors::MembershipError,
    };
    use async_trait::async_trait;
    use chrono::{DateTime, Utc};

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

    // TODO re-enable these tests when re-implement poll_ready
    // #[tokio::test]
    // async fn test_poll_ready_no_active_server() {
    //     let mut client = client();
    //     assert!(client.active_servers.is_none());
    //     client.ready().await.expect("poll ready");
    //     assert_eq!(client.active_servers.expect("active servers").len(), 0);
    // }

    // #[tokio::test]
    // async fn test_poll_ready_with_active_servers() {
    //     let mut client = client();
    //     assert!(client.active_servers.is_none());
    //     let mut server = Member::new("0.0.0.0".to_string(), "1234".to_string());
    //     server.set_active(true);
    //     client
    //         .members_storage
    //         .push(server)
    //         .await
    //         .expect("add member");
    //     client.ready().await.expect("poll ready");
    //     assert_eq!(client.active_servers.expect("active servers").len(), 1);
    // }

    // #[tokio::test]
    // async fn test_poll_ready_error() {
    //     let mut client = Client {
    //         timeout_millis: 1000,
    //         members_storage: FailMemberStorage {},
    //         active_servers: None,
    //         ts_active_servers_refresh: 0,
    //         streams: Arc::default(),
    //         placement: Arc::new(RwLock::new(LruCache::new(10))),
    //     };
    //     let poll_ready_result = client.ready().await;
    //     assert!(poll_ready_result.is_err());
    //     poll_ready_result
    //         .map_err(|err| assert_eq!(err, ClientError::RendevouzUnavailable))
    //         .ok();
    // }

    async fn client_with_members() -> Client<LocalStorage> {
        let client = client();
        let mut server = Member::new("0.0.0.0".to_string(), "1234".to_string());
        server.set_active(true);
        client
            .members_storage
            .push(server)
            .await
            .expect("add member");
        client
    }

    #[tokio::test]
    async fn test_server_stream_no_servers_available_error() {
        let mut client = client();
        let stream_err = client
            .server_stream(&"0.0.0.0:6000".to_string())
            .await
            .unwrap_err();
        assert_eq!(stream_err, ClientError::NoServersAvailable);
    }

    #[tokio::test]
    async fn test_server_stream_server_not_available_error() {
        let mut client = client_with_members().await;
        let stream_err = client
            .server_stream(&"0.0.0.0:6000".to_string())
            .await
            .unwrap_err();
        assert_eq!(
            stream_err,
            ClientError::ServerNotAvailable("0.0.0.0:6000".to_string())
        );
    }

    #[tokio::test]
    async fn test_server_stream_cant_connect_to_server() {
        let mut client = client_with_members().await;
        let stream = client.server_stream(&"0.0.0.0:1234".to_string()).await;
        match stream {
            Err(ClientError::Unknown(_)) => (),
            _ => panic!("stream is not a ClientError::Unknown(_)"),
        };
    }

    #[tokio::test]
    async fn test_service_clone() {
        let client = client_with_members().await;
        let _ = client;
    }

    // TODO integration tests
    // use serde::Deserialize;
    // use rio_macros::TypeName;
    // #[tokio::test]
    // async fn test_client_send() {
    //     #[derive(TypeName, Debug, Serialize, Deserialize, PartialEq)]
    //     #[rio_path = "crate"]
    //     struct Message {}
    //     let mut client = client_with_members().await;
    //     let response: ClientResult<Message> = client
    //         .send("RemoteService".to_string(), "1".to_string(), &Message {})
    //         .await;
    //     response.unwrap();
    // }
}
