use async_trait::async_trait;

use crate::GrainId;

pub mod sql;

pub struct GrainPlacement {
    pub grain_id: GrainId,
    pub silo_address: Option<String>,
    // TODO: ttl
    // TODO: last_seen
}

impl GrainPlacement {
    pub fn new(grain_id: GrainId, silo_address: Option<String>) -> GrainPlacement {
        GrainPlacement {
            grain_id,
            silo_address,
        }
    }
}

#[async_trait]
pub trait GrainPlacementProvider: Send + Sync {
    async fn update(&self, grain_placement: GrainPlacement);
    async fn lookup(&self, grain_id: &GrainId) -> Option<String>;
    async fn upsert(&self, grain_id: GrainId, address: String) -> String {
        let maybe_silo_address = self.lookup(&grain_id).await;
        if let Some(silo_address) = maybe_silo_address {
            silo_address
        } else {
            let new_placement = GrainPlacement::new(grain_id, Some(address));
            let new_address = new_placement.silo_address.clone().unwrap();
            self.update(new_placement).await;
            new_address
        }
    }
    async fn clean_silo(&self, address: String);
    async fn remove(&self, grain_id: &GrainId);
}

#[derive(Default)]
pub struct LocalGrainPlacementProvider {}

#[async_trait]
impl GrainPlacementProvider for LocalGrainPlacementProvider {
    async fn update(&self, _grain_placement: GrainPlacement) {}
    async fn lookup(&self, _grain_id: &GrainId) -> Option<String> {
        None
    }
    async fn clean_silo(&self, _address: String) {}
    async fn remove(&self, _grain_id: &GrainId) {}
}
