use async_trait::async_trait;

use crate::ObjectId;

pub mod sql;
pub mod local;

pub struct ObjectPlacement {
    pub object_id: ObjectId,
    pub silo_address: Option<String>,
    // TODO: ttl
    // TODO: last_seen
}

impl ObjectPlacement {
    pub fn new(object_id: ObjectId, silo_address: Option<String>) -> ObjectPlacement {
        ObjectPlacement {
            object_id,
            silo_address,
        }
    }
}

#[async_trait]
pub trait ObjectPlacementProvider: Send + Sync {
    async fn update(&self, object_placement: ObjectPlacement);
    async fn lookup(&self, object_id: &ObjectId) -> Option<String>;
    async fn upsert(&self, object_id: ObjectId, address: String) -> String {
        let maybe_silo_address = self.lookup(&object_id).await;
        if let Some(silo_address) = maybe_silo_address {
            silo_address
        } else {
            let new_placement = ObjectPlacement::new(object_id, Some(address));
            let new_address = new_placement.silo_address.clone().unwrap();
            self.update(new_placement).await;
            new_address
        }
    }
    async fn clean_silo(&self, address: String);
    async fn remove(&self, object_id: &ObjectId);
}
