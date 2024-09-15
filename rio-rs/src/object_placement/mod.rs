//! Maps object's location in the cluster

use async_trait::async_trait;

use crate::ObjectId;

pub mod local;
pub mod sql;

/// Struct providing placement information
pub struct ObjectPlacement {
    pub object_id: ObjectId,
    pub server_address: Option<String>,
    // TODO: ttl
    // TODO: last_seen
}

impl ObjectPlacement {
    pub fn new(object_id: ObjectId, server_address: Option<String>) -> ObjectPlacement {
        ObjectPlacement {
            object_id,
            server_address,
        }
    }
}

/// This trait decribes how to manipulate objects' allocation
/// This is pretty much a CRUD for the mapping
#[async_trait]
pub trait ObjectPlacementProvider: Send + Sync + Clone {
    /// Setup step, one can define it for their [ObjectPlacementProvider] so it does some
    /// prep work before the server is running
    async fn prepare(&self) {}
    /// Insert or update the object placement
    async fn update(&self, object_placement: ObjectPlacement);
    /// Find the server address for a given object
    async fn lookup(&self, object_id: &ObjectId) -> Option<String>;
    /// Unassign all objects for a given server
    async fn clean_server(&self, address: String);
    /// Unassign a single object by its ID
    async fn remove(&self, object_id: &ObjectId);
}
