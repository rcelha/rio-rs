//!
//! There is a pooled client. The client also does proper placement lookups and controls its own
//! caching strategy

use async_trait::async_trait;
use bb8::{ManageConnection, Pool};

use crate::cluster::storage::MembersStorage;
use crate::protocol::ClientError;

use super::Client;
use super::ClientBuilder;
use super::DEFAULT_TIMEOUT_MILLIS;

/// TODO: Move cache out of the Client struct so we can share the cache across all connections in
/// the pool
pub struct ClientConnectionManager<S: MembersStorage> {
    pub(crate) members_storage: S,
    pub(crate) timeout_millis: u64,
}
impl<S: MembersStorage + 'static> ClientConnectionManager<S> {
    pub fn new(members_storage: S) -> Self {
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
impl<S: MembersStorage + 'static> ManageConnection for ClientConnectionManager<S> {
    type Connection = Client<S>;
    type Error = ClientError;
    async fn connect(&self) -> Result<Self::Connection, Self::Error> {
        ClientBuilder::new()
            .members_storage(self.members_storage.clone())
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
