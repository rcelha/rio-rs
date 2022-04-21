use std::time::Duration;

use async_trait::async_trait;

use crate::{errors::ClusterProviderServeError, membership_provider::MembersStorage};

pub mod peer_to_peer;

#[async_trait]
pub trait ClusterProvider<T>
where
    T: MembersStorage,
{
    fn members_storage(&self) -> &T;
    async fn serve(&self, address: &str) -> Result<(), ClusterProviderServeError>;
}

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

#[cfg(test)]
mod test {}
