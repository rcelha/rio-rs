use async_trait::async_trait;

use crate::ObjectId;

pub mod local;
pub mod sql;

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

#[async_trait]
pub trait ObjectPlacementProvider: Send + Sync {
    async fn update(&self, object_placement: ObjectPlacement);
    async fn lookup(&self, object_id: &ObjectId) -> Option<String>;
    async fn clean_server(&self, address: String);
    async fn remove(&self, object_id: &ObjectId);
}
