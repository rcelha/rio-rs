use async_trait::async_trait;

use crate::object_placement::{ObjectPlacement, ObjectPlacementProvider};
use crate::ObjectId;

#[derive(Default)]
pub struct LocalObjectPlacementProvider {}

#[async_trait]
impl ObjectPlacementProvider for LocalObjectPlacementProvider {
    async fn update(&self, _object_placement: ObjectPlacement) {}
    async fn lookup(&self, _object_id: &ObjectId) -> Option<String> {
        None
    }
    async fn clean_silo(&self, _address: String) {}
    async fn remove(&self, _object_id: &ObjectId) {}
}
