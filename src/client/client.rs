//! Provides a client to interact with a cluster in a request/response manner
//!
//! There is a pooled client. The client also does proper placement lookups and controls its own
//! caching strategy

use std::collections::HashSet;
use std::sync::{Arc, RwLock};
use tokio::time::timeout;

use async_recursion::async_recursion;
use dashmap::mapref::one::RefMut;
use dashmap::DashMap;
use futures::future::BoxFuture;

use futures::SinkExt;
use lru::LruCache;
use rand::{prelude::SliceRandom, thread_rng};

use serde::de::DeserializeOwned;
use serde::Serialize;
use tokio::net::TcpStream;

use tokio_stream::StreamExt;
use tokio_util::codec::{Framed, LengthDelimitedCodec};
use tower::Service as TowerService;

use crate::cluster::storage::MembersStorage;
use crate::protocol::{ClientError, RequestEnvelope, ResponseEnvelope, ResponseError};
use crate::registry::IdentifiableType;

// TODO enable timeout?
use super::DEFAULT_TIMEOUT_MILLIS;

#[derive(Clone)]
/// Client struct to interact with a cluster in a request/response manner
pub struct Client<S>
where
    S: MembersStorage,
{
    pub(crate) timeout_millis: u64,

    /// Membership view used for Server's service discovery
    pub(crate) members_storage: S,

    /// List of servers that are accepting requests
    pub(crate) active_servers: Option<HashSet<String>>,

    /// Timestamp of the last time self.active_servers was refresh
    pub(crate) ts_active_servers_refresh: u64,

    /// Framed TCP Stream mapped by ip+port address
    pub(crate) streams: Arc<DashMap<String, Framed<TcpStream, LengthDelimitedCodec>>>,

    /// TODO
    pub(crate) placement: Arc<RwLock<LruCache<(String, String), String>>>,
}

// TODO do I need S to be 'static?
impl<S> TowerService<RequestEnvelope> for Client<S>
where
    S: 'static + MembersStorage,
{
    type Response = Vec<u8>;
    type Error = ClientError;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(
        &mut self,
        _: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        std::task::Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: RequestEnvelope) -> Self::Future {
        let mut this = self.clone();
        Box::pin(async move {
            let mut stream = this
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
                Some(Err(e)) => Err(ClientError::Unknown(e.to_string())),
                None => Err(ClientError::Unknown("Unknown error".to_string())),
            }
        })
    }
}

type ClientResult<T> = Result<T, ClientError>;

impl<S> Client<S>
where
    S: 'static + MembersStorage,
{
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

        self.streams
            .get_mut(address)
            .ok_or(ClientError::Connectivity)
    }

    /// Returns a stream to the server that a given ServiceObject might be allocated into
    /// TODO can I change this to read only?
    async fn service_object_stream(
        &mut self,
        service_object_type: impl ToString,
        service_object_id: impl ToString,
    ) -> ClientResult<RefMut<'_, String, Framed<TcpStream, LengthDelimitedCodec>>> {
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

        self.server_stream(&address).await
    }

    /// Send a request to the cluster transparently (the caller doesn't need to know where the
    /// object is placed)
    #[async_recursion]
    pub async fn send<T, V>(
        &mut self,
        handler_type: String,
        handler_id: String,
        payload: &V,
    ) -> Result<T, ClientError>
    where
        T: DeserializeOwned,
        V: Serialize + IdentifiableType + Send + Sync,
    {
        // TODO move fetch_active_servers into poll_ready self.ready().await?;
        self.fetch_active_servers().await?;
        let ser_payload = bincode::serialize(&payload)
            .map_err(|e| ClientError::SeralizationError(e.to_string()))?;
        let message_type = V::user_defined_type_id().to_string();

        let req = RequestEnvelope {
            handler_type: handler_type.clone(),
            handler_id: handler_id.clone(),
            payload: ser_payload,
            message_type,
        };
        let response = self.call(req).await;
        match response {
            Ok(v) => {
                let body: T = bincode::deserialize(&v)
                    .map_err(|e| ClientError::DeseralizationError(e.to_string()))?;
                Ok(body)
            }
            Err(ClientError::ResponseError(ResponseError::Redirect(to))) => {
                self.placement
                    .write()
                    .map_err(|_| ClientError::PlacementLock)?
                    .put((handler_type.clone(), handler_id.clone()), to);
                self.send::<T, V>(handler_type, handler_id, payload).await
            }
            // Retry so it picks up a new Server on the cluster
            Err(ClientError::ResponseError(ResponseError::DeallocateServiceObject)) => {
                self.placement
                    .write()
                    .map_err(|_| ClientError::PlacementLock)?
                    .pop(&(handler_type.clone(), handler_id.clone()));
                self.send::<T, V>(handler_type, handler_id, payload).await
            }
            Err(err) => Err(err),
        }
    }

    // TODO: remove this?
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
            LocalStorage, Member, MembersStorage, MembershipResult, MembershipUnitResult,
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
