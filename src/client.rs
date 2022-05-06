//! Provides a client to interact with a cluster in a request/response manner
//!
//! There is a pooled client. The client also does proper placement lookups and controls its own
//! caching strategy

use std::collections::HashMap;

use async_recursion::async_recursion;
use async_trait::async_trait;
use bb8::{ManageConnection, Pool};
use futures::SinkExt;
use lru::LruCache;
use rand::{prelude::SliceRandom, thread_rng};
use serde::{de::DeserializeOwned, Serialize};
use tokio::net::TcpStream;
use tokio::time::timeout;
use tokio_stream::StreamExt;
use tokio_util::codec::{Framed, LengthDelimitedCodec};

use crate::cluster::storage::MembersStorage;
use crate::errors::{ClientBuilderError, ClientError};
use crate::protocol::{RequestEnvelope, ResponseEnvelope, ResponseError};
use crate::registry::IdentifiableType;

const DEFAULT_TIMEOUT_MILLIS: u64 = 500;

/// Client struct to interact with a cluster in a request/response manner
pub struct Client {
    timeout_millis: u64,

    /// Membership view used for Silo service discovery
    members_storage: Box<dyn MembersStorage>,

    /// Framed TCP Stream mapped by ip+port address
    streams: HashMap<String, Framed<TcpStream, LengthDelimitedCodec>>,

    /// ServiceObject placement cache ((type, id) -> address)
    placement: LruCache<(String, String), String>,
}

/// Helper Struct to build clients from configuration
#[derive(Default)]
pub struct ClientBuilder {
    members_storage: Option<Box<dyn MembersStorage>>,
    timeout_millis: u64,
}

impl ClientBuilder {
    pub fn new() -> Self {
        ClientBuilder {
            timeout_millis: DEFAULT_TIMEOUT_MILLIS,
            ..Default::default()
        }
    }

    pub fn members_storage(&mut self, members_storage: impl MembersStorage + 'static) -> &mut Self {
        self.members_storage = Some(Box::new(members_storage));
        self
    }

    pub fn timeout_millis(&mut self, timeout_millis: u64) -> &mut Self {
        self.timeout_millis = timeout_millis;
        self
    }

    pub fn boxed_members_storage(&mut self, members_storage: Box<dyn MembersStorage>) -> &mut Self {
        self.members_storage = Some(members_storage);
        self
    }

    pub fn build(&self) -> Result<Client, ClientBuilderError> {
        let members_storage = self
            .members_storage
            .clone()
            .ok_or(ClientBuilderError::NoMembersStorage)?;

        let mut client = Client::new(members_storage);
        client.timeout_millis = self.timeout_millis;
        Ok(client)
    }

    pub fn build_connection_manager(&self) -> Result<ClientConnectionManager, ClientBuilderError> {
        let members_storage = self
            .members_storage
            .clone()
            .ok_or(ClientBuilderError::NoMembersStorage)?;
        let mut connection_manager = ClientConnectionManager::new(members_storage);
        connection_manager.timeout_millis = self.timeout_millis;
        Ok(connection_manager)
    }
}

impl Client {
    pub fn new(members_storage: Box<dyn MembersStorage>) -> Self {
        Client {
            timeout_millis: DEFAULT_TIMEOUT_MILLIS,
            streams: HashMap::new(),
            members_storage,
            placement: LruCache::new(1000), // TODO: configure capacity
        }
    }

    /// Returns the address (ip + port) for a given ServiceObject in the cluster
    ///
    /// In case this information is no available on the client, it will try
    /// a random server. The server has the ability to 'redirect' the client
    /// to the right server in case there is a mismatch
    async fn service_object_lookup(
        &mut self,
        handler_type_id: String,
        handler_id: String,
    ) -> Result<String, ClientError> {
        let object_id = (handler_type_id, handler_id);
        if let Some(address) = self.placement.get(&object_id) {
            return Ok(address.clone());
        }

        let silos = self
            .members_storage
            .active_members()
            .await
            .map_err(|err| ClientError::Unknown(err.to_string()))?;
        let mut rng = thread_rng();
        silos
            .choose(&mut rng)
            .map(|i| {
                let address = i.address();
                self.placement.put(object_id, address.clone());
                address
            })
            .ok_or(ClientError::NoSilosAvailable)
    }

    /// Get or create a TCP stream to a server in the cluster
    async fn stream(
        &mut self,
        address: &str,
    ) -> Result<&mut Framed<TcpStream, LengthDelimitedCodec>, ClientError> {
        if !self.streams.contains_key(address) {
            let stream = TcpStream::connect(&address)
                .await
                .map_err(|x| ClientError::Unknown(format!("(Address {}) - {:?}", address, x)))?;
            let stream = Framed::new(stream, LengthDelimitedCodec::new());
            self.streams.insert(address.to_string(), stream);
        }
        self.streams.get_mut(address).ok_or(ClientError::Unknown(
            "Failed to lookup after insertion".to_string(),
        ))
    }

    /// Creates a connection and close it. Useful to test client/server connectivity
    pub async fn ping(&mut self) -> Result<(), ClientError> {
        let silos = self
            .members_storage
            .members()
            .await
            .map_err(|_| ClientError::Connectivity)?;
        let silo = silos.first().ok_or(ClientError::NoSilosAvailable)?;

        async fn conn(address: &str) -> Result<(), ClientError> {
            TcpStream::connect(&address)
                .await
                .map(|_stream| Ok(()))
                .map_err(|_e| ClientError::Connectivity)?
        }

        match timeout(
            std::time::Duration::from_millis(self.timeout_millis),
            conn(&silo.address()),
        )
        .await
        {
            Ok(x) => x,
            Err(_elapsed) => Err(ClientError::Connectivity),
        }
    }

    /// TODO replace Option with Result
    async fn service_object_stream(
        &mut self,
        handler_type_id: String,
        handler_id: String,
    ) -> Result<&mut Framed<TcpStream, LengthDelimitedCodec>, ClientError> {
        let address = self
            .service_object_lookup(handler_type_id, handler_id)
            .await?;
        let stream = self.stream(&address).await?;
        Ok(stream)
    }

    /// Send a request to the cluster transparently (the caller doesn't need to know where the
    /// object is placed)
    #[async_recursion]
    pub async fn send<'a, T, V>(
        &mut self,
        handler_type_id: String,
        handler_id: String,
        payload: &V,
    ) -> Result<T, ClientError>
    where
        T: DeserializeOwned,
        V: Serialize + IdentifiableType + Send + Sync,
    {
        let stream = self
            .service_object_stream(handler_type_id.clone(), handler_id.clone())
            .await?;

        let request = RequestEnvelope::new(
            handler_type_id.clone(),
            handler_id.clone(),
            V::user_defined_type_id().to_string(),
            bincode::serialize(&payload).unwrap(),
        );
        let ser_request = bincode::serialize(&request).unwrap();
        stream.send(ser_request.into()).await.unwrap();

        match stream.next().await {
            Some(Ok(frame)) => {
                let message: ResponseEnvelope = bincode::deserialize(&frame).unwrap();
                match message.body {
                    Ok(v) => {
                        let body: T = bincode::deserialize(&v).unwrap();
                        Ok(body)
                    }
                    Err(ResponseError::Redirect(to)) => {
                        self.placement
                            .put((handler_type_id.clone(), handler_id.clone()), to);
                        self.send::<T, V>(handler_type_id, handler_id, payload)
                            .await
                    }
                    // Retry so it picks up a new Silo on the cluster
                    Err(ResponseError::DeallocateServiceObject) => {
                        self.placement
                            .pop(&(handler_type_id.clone(), handler_id.clone()));
                        self.send::<T, V>(handler_type_id, handler_id, payload)
                            .await
                    }

                    Err(err) => Err(ClientError::Unknown(format!("protocol error: {}", err))),
                }
            }
            Some(Err(e)) => Err(ClientError::Unknown(e.to_string())),
            _ => Err(ClientError::Unknown("Unknown error".to_string())),
        }
    }
}

/// TODO: Move cache out of the Client struct so we can share the cache across all connections in
/// the pool
pub struct ClientConnectionManager {
    members_storage: Box<dyn MembersStorage>,
    timeout_millis: u64,
}

impl ClientConnectionManager {
    pub fn new(members_storage: Box<dyn MembersStorage>) -> Self {
        ClientConnectionManager {
            members_storage,
            timeout_millis: DEFAULT_TIMEOUT_MILLIS,
        }
    }

    pub fn pool() -> bb8::Builder<Self> {
        Pool::builder()
    }
}

#[async_trait]
impl ManageConnection for ClientConnectionManager {
    type Connection = Client;
    type Error = ClientError;
    async fn connect(&self) -> Result<Self::Connection, Self::Error> {
        ClientBuilder::new()
            .boxed_members_storage(self.members_storage.clone())
            .timeout_millis(self.timeout_millis)
            .build()
            .map_err(|err| ClientError::Unknown(err.to_string()))
    }

    async fn is_valid(
        &self,
        _conn: &mut bb8::PooledConnection<'_, Self>,
    ) -> Result<(), Self::Error> {
        Ok(())
    }

    fn has_broken(&self, _conn: &mut Self::Connection) -> bool {
        false
    }
}
