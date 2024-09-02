//! This is a pooled client. The client also does proper placement lookups and controls its own
//! caching strategy

use async_trait::async_trait;
use bb8::{ManageConnection, Pool};

use crate::cluster::storage::MembersStorage;
use crate::protocol::ClientError;

use super::Client;
use super::ClientBuilder;
use super::DEFAULT_TIMEOUT_MILLIS;

/// Struct to help implementing pooling with bb8
///
/// <div class="warning">
///
/// # TODO
/// - Move the cache out of the Client struct so we can share the cache across all connections in the pool
///
/// </div>
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

#[cfg(test)]
mod test {
    use bb8::Pool;

    use crate::cluster::storage::local::LocalStorage;
    use crate::cluster::storage::Member;

    use super::*;

    #[tokio::test]
    async fn basic_usage() {
        let local_members_storage = LocalStorage::default();
        local_members_storage
            .push(Member::new("0.0.0.0".to_string(), "9999".to_string()))
            .await
            .unwrap();

        let manager = ClientConnectionManager::new(local_members_storage);
        let client = Pool::builder().build(manager).await.unwrap();

        let mut conn_1 = client.get().await.unwrap();
        let conn_2 = client.get().await.unwrap();

        conn_1.fetch_active_servers().await.unwrap();

        assert_eq!(conn_1.members_storage.members().await.unwrap().len(), 1);
        assert_eq!(conn_2.members_storage.members().await.unwrap().len(), 1);
    }
}
