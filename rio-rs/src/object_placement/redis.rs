//! Redis implementation of the trait [ObjectPlacement]

use std::collections::HashSet;

use async_trait::async_trait;
use bb8_redis::{bb8::Pool, redis::AsyncCommands, RedisConnectionManager};

use super::{ObjectPlacement, ObjectPlacementItem};
use crate::errors::ObjectPlacementError;
use crate::ObjectId;

#[derive(Clone, Debug)]
pub struct RedisObjectPlacement {
    pool: Pool<RedisConnectionManager>,
    key_prefix: String,
}

impl RedisObjectPlacement {
    pub async fn from_connect_string(connection_string: &str, key_prefix: Option<String>) -> Self {
        let conn_manager = RedisConnectionManager::new(connection_string).expect("TODO");
        let pool = Pool::builder().build(conn_manager).await.expect("TODO");
        let key_prefix = key_prefix.unwrap_or_default();
        Self { pool, key_prefix }
    }
}

#[async_trait]
impl ObjectPlacement for RedisObjectPlacement {
    async fn update(
        &self,
        object_placement: ObjectPlacementItem,
    ) -> Result<(), ObjectPlacementError> {
        let object_id = format!(
            "{}:{}",
            object_placement.object_id.0, object_placement.object_id.1
        );
        let k1 = format!("{}{}", self.key_prefix, object_id);
        let mut client = self
            .pool
            .get()
            .await
            .map_err(|e| ObjectPlacementError::Upstream(e.to_string()))?;

        if let Some(server_address) = object_placement.server_address {
            let k2 = format!("{}{}", self.key_prefix, server_address);
            let mut pipe = redis::pipe();
            pipe.set(&k1, &server_address).sadd(&k2, &object_id);
            let _: () = pipe
                .exec_async(&mut *client)
                .await
                .map_err(|e| ObjectPlacementError::Upstream(e.to_string()))?;
        } else {
            // If there is no server associated with the allocation
            // it means we can remove the placement associated with the object
            let _: () = client
                .del(&k1)
                .await
                .map_err(|e| ObjectPlacementError::Upstream(e.to_string()))?;
        }
        Ok(())
    }

    async fn lookup(&self, object_id: &ObjectId) -> Result<Option<String>, ObjectPlacementError> {
        let k = format!("{}{}:{}", self.key_prefix, object_id.0, object_id.1);
        let mut client = self
            .pool
            .get()
            .await
            .map_err(|e| ObjectPlacementError::Upstream(e.to_string()))?;
        let placement: Option<String> = client
            .get(&k)
            .await
            .map_err(|e| ObjectPlacementError::Upstream(e.to_string()))?;
        Ok(placement)
    }

    async fn clean_server(&self, address: String) -> Result<(), ObjectPlacementError> {
        let k = format!("{}{}", self.key_prefix, address);
        let mut client = self
            .pool
            .get()
            .await
            .map_err(|e| ObjectPlacementError::Upstream(e.to_string()))?;
        let objects_in_server: HashSet<String> = client
            .smembers(&k)
            .await
            .map_err(|e| ObjectPlacementError::Upstream(e.to_string()))?;
        for object_id in objects_in_server.iter() {
            let k = format!("{}{}", self.key_prefix, object_id);
            let _: () = client
                .del(&k)
                .await
                .map_err(|e| ObjectPlacementError::Upstream(e.to_string()))?;
        }
        let _: () = client
            .del(&k)
            .await
            .map_err(|e| ObjectPlacementError::Upstream(e.to_string()))?;
        Ok(())
    }

    async fn remove(&self, object_id: &ObjectId) -> Result<(), ObjectPlacementError> {
        let object_id = format!("{}:{}", object_id.0, object_id.1);
        let k = format!("{}{}", self.key_prefix, object_id);
        let mut client = self
            .pool
            .get()
            .await
            .map_err(|e| ObjectPlacementError::Upstream(e.to_string()))?;
        let _: () = client
            .del(&k)
            .await
            .map_err(|e| ObjectPlacementError::Upstream(e.to_string()))?;
        Ok(())
    }
}
