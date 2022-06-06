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
    async fn upsert(&self, object_id: ObjectId, address: String) -> String {
        let maybe_server_address = self.lookup(&object_id).await;
        if let Some(server_address) = maybe_server_address {
            server_address
        } else {
            let new_placement = ObjectPlacement::new(object_id, Some(address));
            let new_address = new_placement.server_address.clone().unwrap();
            self.update(new_placement).await;
            new_address
        }
    }
    async fn clean_server(&self, address: String);
    async fn remove(&self, object_id: &ObjectId);
}
