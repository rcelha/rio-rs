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
    async fn clean_silo(&self, address: String);
}
