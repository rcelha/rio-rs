//! Maps object's location in the cluster

use std::fmt::Debug;

use async_trait::async_trait;

use crate::errors::ObjectPlacementError;
use crate::ObjectId;

#[cfg(feature = "local")]
pub mod local;
#[cfg(feature = "postgres")]
pub mod postgres;
#[cfg(feature = "redis")]
pub mod redis;
#[cfg(feature = "sqlite")]
pub mod sqlite;

/// Struct providing placement information
pub struct ObjectPlacementItem {
    pub object_id: ObjectId,
    pub server_address: Option<String>,
    // TODO: ttl
    // TODO: last_seen
}

impl ObjectPlacementItem {
    pub fn new(object_id: ObjectId, server_address: Option<String>) -> ObjectPlacementItem {
        ObjectPlacementItem {
            object_id,
            server_address,
        }
    }
}

/// This trait decribes how to manipulate objects' allocation
/// This is pretty much a CRUD for the mapping
#[async_trait]
pub trait ObjectPlacement: Send + Sync + Clone + Debug {
    /// Setup step, one can define it for their [ObjectPlacement] so it does some
    /// prep work before the server is running
    async fn prepare(&self) -> Result<(), ObjectPlacementError> {
        Ok(())
    }
    /// Insert or update the object placement
    async fn update(
        &self,
        object_placement: ObjectPlacementItem,
    ) -> Result<(), ObjectPlacementError>;
    /// Find the server address for a given object
    async fn lookup(&self, object_id: &ObjectId) -> Result<Option<String>, ObjectPlacementError>;
    /// Unassign all objects for a given server
    async fn clean_server(&self, address: String) -> Result<(), ObjectPlacementError>;
    /// Unassign a single object by its ID
    async fn remove(&self, object_id: &ObjectId) -> Result<(), ObjectPlacementError>;
}
