use crate::errors::LoadStateError;
use async_trait::async_trait;
use futures::TryFutureExt;
use serde::{de::DeserializeOwned, Serialize};
use sqlx::{self, any::AnyRow, AnyPool, Row};

use super::{StateLoader, StateSaver};

#[derive(Debug)]
pub struct SqlState {
    pool: AnyPool,
}

impl SqlState {
    pub fn new(pool: AnyPool) -> Self {
        Self { pool }
    }

    pub async fn migrate(&self) {
        let mut transaction = self.pool.begin().await.unwrap();

        let queries = [
            // Table with all state objects
            r#"CREATE TABLE IF NOT EXISTS state_provider_object_state
               (
                   grain_type       TEXT                              NOT NULL,
                   grain_id         TEXT                              NOT NULL,
                   state_type       TEXT                              NOT NULL,
                   serialized_state BLOB                              NOT NULL,
                   PRIMARY KEY (grain_type, grain_id, state_type)
               )"#,
        ];
        for query in queries {
            sqlx::query(query).execute(&mut transaction).await.unwrap();
        }
        transaction.commit().await.unwrap();
    }
}

#[async_trait]
impl StateLoader for SqlState {
    async fn load<T: DeserializeOwned>(
        &self,
        grain_type: &str,
        grain_id: &str,
        state_type: &str,
    ) -> Result<T, LoadStateError> {
        let items = sqlx::query(
            r#"
            SELECT serialized_state
            FROM state_provider_object_state
            WHERE grain_type=$1 AND grain_id=$2 AND state_type = $3
            "#,
        )
        .bind(grain_type)
        .bind(grain_id)
        .bind(state_type)
        .map(|x: AnyRow| -> String { x.get("serialized_state") })
        .fetch_one(&self.pool)
        .map_err(|_| LoadStateError::ObjectNotFound)
        .await?;
        let data = serde_json::from_str(&items).expect("TODO");
        Ok(data)
    }
}

#[async_trait]
impl StateSaver for SqlState {
    async fn save(
        &self,
        grain_type: &str,
        grain_id: &str,
        state_type: &str,
        data: &(impl Serialize + Send + Sync),
    ) -> Result<(), LoadStateError> {
        let serialized_data = serde_json::to_string(&data).expect("TODO");
        sqlx::query(
            r#"
            INSERT INTO
                state_provider_object_state(grain_type, grain_id, state_type, serialized_state)
            VALUES ($1, $2, $3, $4)
            ON CONFLICT(grain_type, grain_id, state_type) DO UPDATE SET serialized_state=$4
            "#,
        )
        .bind(grain_type)
        .bind(grain_id)
        .bind(state_type)
        .bind(serialized_data)
        .execute(&self.pool)
        .map_err(|_| LoadStateError::Unknown)
        .await
        .map(|_| ())
    }
}
