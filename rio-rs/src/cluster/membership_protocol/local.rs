//! In-memory ClusterProvider for testing.

use async_trait::async_trait;
use std::time::Duration;

use super::ClusterProvider;
use crate::{cluster::storage::MembersStorage, errors::ClusterProviderServeError};

/// Local server compatible with the ClusterProvider API
///
/// This is only for tests, and it is doesn't offer real
/// cluster capabilities
#[derive(Clone)]
pub struct LocalClusterProvider<T> {
    pub members_storage: T,
}

#[async_trait]
impl<T> ClusterProvider<T> for LocalClusterProvider<T>
where
    T: MembersStorage,
{
    fn members_storage(&self) -> &T {
        &self.members_storage
    }

    async fn serve(&self, _address: &str) -> Result<(), ClusterProviderServeError> {
        loop {
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    }
}
