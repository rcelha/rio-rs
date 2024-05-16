//! Controls which cluster members are healthy

use async_trait::async_trait;

use crate::{cluster::storage::MembersStorage, errors::ClusterProviderServeError};

pub mod local;
pub mod peer_to_peer;

#[async_trait]
pub trait ClusterProvider<T>
where
    T: MembersStorage,
{
    fn members_storage(&self) -> &T;
    async fn serve(&self, address: &str) -> Result<(), ClusterProviderServeError>;
}
