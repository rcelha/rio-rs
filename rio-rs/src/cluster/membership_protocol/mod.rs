//! Controls which cluster members' are healthy

use async_trait::async_trait;

use crate::{cluster::storage::MembersStorage, errors::ClusterProviderServeError};

pub mod local;
pub mod peer_to_peer;

/// The sole purpose of a [ClusterProvider] is to inform which
/// servers are part of the cluster and which of these are healthy
/// or not.
///
/// To list which servers are part of the cluster, it uses a [MembersStorage](super::storage::MembersStorage).
/// The cluster provider uses the MembersStorage's API to update the state of the providers.
#[async_trait]
pub trait ClusterProvider<T>
where
    Self: Clone,
    T: MembersStorage,
{
    /// Every ClusterProvider needs to have an [MembersStorage] associated to it
    ///
    /// <div class="warning">
    /// I am not sure this function is needed
    /// </div>
    fn members_storage(&self) -> &T;

    /// The ClusterProvider runs in a continuous loop, invoked by the [Server](crate::server::Server).
    ///
    /// Each CLusterProvider will implement different logic for its membership algorithm, but it
    /// needs to be able to run it along the duration of the [Server](crate::server::Server).
    async fn serve(&self, address: &str) -> Result<(), ClusterProviderServeError>;
}
