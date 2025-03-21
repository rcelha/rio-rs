//! In-memory implementation of the trait [ObjectPlacement]

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use async_trait::async_trait;

use crate::object_placement::{ObjectPlacement, ObjectPlacementItem};
use crate::ObjectId;

type PlacementMap = Arc<RwLock<HashMap<String, String>>>;

/// In-memory implementation of the trait [ObjectPlacement]
#[derive(Default, Clone)]
pub struct LocalObjectPlacement {
    placement: PlacementMap,
}

#[async_trait]
impl ObjectPlacement for LocalObjectPlacement {
    async fn update(&self, object_placement: ObjectPlacementItem) {
        let object_id = format!(
            "{}.{}",
            object_placement.object_id.0, object_placement.object_id.1
        );
        let mut placement_guard = self
            .placement
            .write()
            .expect("Poisoned lock: ObjectPlacement map");
        if let Some(address) = object_placement.server_address {
            *placement_guard.entry(object_id).or_default() = address;
        } else {
            placement_guard.remove(&object_id);
        }
    }

    async fn lookup(&self, object_id: &ObjectId) -> Option<String> {
        let object_id = format!("{}.{}", object_id.0, object_id.1);
        let placement_guard = self
            .placement
            .read()
            .expect("Poisoned lock: ObjectPlacement map");
        placement_guard.get(&object_id).cloned()
    }

    async fn clean_server(&self, address: String) {
        let mut placement_guard = self
            .placement
            .write()
            .expect("Poisoned lock: ObjectPlacement map");
        placement_guard.retain(|_, v| *v != address);
    }

    async fn remove(&self, object_id: &ObjectId) {
        let object_id = format!("{}.{}", object_id.0, object_id.1);
        let mut placement_guard = self
            .placement
            .write()
            .expect("Poisoned lock: ObjectPlacement map");
        placement_guard.remove(&object_id);
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[tokio::test]
    async fn local_object_placement_provider_is_clonable() {
        let provider = LocalObjectPlacement::default();
        let cloned_provider = provider.clone();

        provider
            .update(ObjectPlacementItem::new(
                ObjectId("test".to_string(), "1".to_string()),
                Some("0.0.0.0:80".to_string()),
            ))
            .await;

        assert!(provider
            .lookup(&ObjectId("test".to_string(), "1".to_string()))
            .await
            .is_some());
        assert!(cloned_provider
            .lookup(&ObjectId("test".to_string(), "1".to_string()))
            .await
            .is_some());

        cloned_provider.clean_server("0.0.0.0:80".to_string()).await;

        assert!(provider
            .lookup(&ObjectId("test".to_string(), "1".to_string()))
            .await
            .is_none());
        assert!(cloned_provider
            .lookup(&ObjectId("test".to_string(), "1".to_string()))
            .await
            .is_none());
    }
}
