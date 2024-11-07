use async_trait::async_trait;
use bb8_redis::{bb8::Pool, RedisConnectionManager};
use redis::AsyncCommands;
use serde::de::DeserializeOwned;
use serde::Serialize;

use super::{StateLoader, StateSaver};
use crate::errors::LoadStateError;

/// State Storage using Redis/Valkey
#[derive(Clone)]
pub struct RedisState {
    pool: Pool<RedisConnectionManager>,
    key_prefix: String,
}

impl RedisState {
    pub async fn from_connect_string(connection_string: &str, key_prefix: Option<String>) -> Self {
        let conn_manager = RedisConnectionManager::new(connection_string).expect("TODO");
        let pool = Pool::builder().build(conn_manager).await.expect("TODO");
        let key_prefix = key_prefix.unwrap_or_default();
        Self { pool, key_prefix }
    }
}

#[async_trait]
impl<T> StateLoader<T> for RedisState
where
    T: DeserializeOwned,
{
    async fn load(
        &self,
        object_kind: &str,
        object_id: &str,
        state_type: &str,
    ) -> Result<T, LoadStateError> {
        let object_kind = object_kind.to_string();
        let object_id = object_id.to_string();
        let state_type = state_type.to_string();
        let key = format!(
            "{}state:{}:{}:{}",
            self.key_prefix, object_kind, object_id, state_type
        );
        let mut client = self.pool.get().await.map_err(|_| LoadStateError::Unknown)?;
        let se_data: Option<String> = client.get(key).await.expect("TODO");
        if let Some(x) = se_data {
            let data = serde_json::from_str(&x).map_err(|_| LoadStateError::DeserializationError);
            data
        } else {
            Err(LoadStateError::ObjectNotFound)
        }
    }
}

#[async_trait]
impl StateSaver for RedisState {
    async fn save(
        &self,
        object_kind: &str,
        object_id: &str,
        state_type: &str,
        data: &(impl Serialize + Send + Sync),
    ) -> Result<(), LoadStateError> {
        let object_kind = object_kind.to_string();
        let object_id = object_id.to_string();
        let state_type = state_type.to_string();
        let key = format!(
            "{}state:{}:{}:{}",
            self.key_prefix, object_kind, object_id, state_type
        );
        let ser_data =
            serde_json::to_string(&data).map_err(|_| LoadStateError::SerializationError)?;
        let mut client = self.pool.get().await.map_err(|_| LoadStateError::Unknown)?;
        let _: () = client
            .set(key, ser_data)
            .await
            .map_err(|_| LoadStateError::Unknown)?;
        Ok(())
    }
}
