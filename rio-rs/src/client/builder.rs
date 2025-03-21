//! Provides a client to interact with a cluster in a request/response manner
//!
//! There is a pooled client. The client also does proper placement lookups and controls its own
//! caching strategy

use crate::cluster::storage::MembershipStorage;
use crate::errors::ClientBuilderError;

use super::pool::ClientConnectionManager;
use super::Client;
use super::DEFAULT_TIMEOUT_MILLIS;

/// Helper Struct to build clients from configuration
pub struct ClientBuilder<S: MembershipStorage> {
    members_storage: Option<S>,
    timeout_millis: u64,
}

impl<S: MembershipStorage> Default for ClientBuilder<S> {
    fn default() -> Self {
        ClientBuilder {
            members_storage: None,
            timeout_millis: 0,
        }
    }
}

impl<S: MembershipStorage + 'static> ClientBuilder<S> {
    pub fn new() -> Self {
        ClientBuilder {
            timeout_millis: DEFAULT_TIMEOUT_MILLIS,
            ..Default::default()
        }
    }

    pub fn members_storage(mut self, members_storage: S) -> Self {
        self.members_storage = Some(members_storage);
        self
    }

    pub fn timeout_millis(mut self, timeout_millis: u64) -> Self {
        self.timeout_millis = timeout_millis;
        self
    }

    pub fn build(self) -> Result<Client<S>, ClientBuilderError> {
        let members_storage = self
            .members_storage
            .clone()
            .ok_or(ClientBuilderError::NoMembershipStorage)?;

        let mut client = Client::new(members_storage);
        client.timeout_millis = self.timeout_millis;
        Ok(client)
    }

    pub fn build_connection_manager(
        &self,
    ) -> Result<ClientConnectionManager<S>, ClientBuilderError> {
        let members_storage = self
            .members_storage
            .clone()
            .ok_or(ClientBuilderError::NoMembershipStorage)?;
        let mut connection_manager = ClientConnectionManager::new(members_storage);
        connection_manager.timeout_millis = self.timeout_millis;
        Ok(connection_manager)
    }
}

#[cfg(test)]
mod test {
    use crate::cluster::storage::local::LocalStorage;

    use super::*;

    #[tokio::test]
    async fn test_default_builder() {
        let client_builder = ClientBuilder::<LocalStorage>::new();
        assert!(client_builder.members_storage.is_none());
    }

    #[tokio::test]
    async fn test_builder_without_storage() {
        let client = ClientBuilder::<LocalStorage>::new().build();
        assert!(client.is_err());
    }

    #[tokio::test]
    async fn test_builder_build() {
        let client = ClientBuilder::new()
            .members_storage(LocalStorage::default())
            .build();
        assert!(client.is_ok());
    }
}
