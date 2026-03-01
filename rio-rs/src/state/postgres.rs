use crate::{errors::LoadStateError, sql_migration::SqlMigrations};
use async_trait::async_trait;
use futures::TryFutureExt;
use serde::{de::DeserializeOwned, Serialize};
use sqlx::{
    self,
    postgres::{PgPoolOptions, PgRow},
    PgPool, Row,
};

use super::{StateLoader, StateSaver};

pub struct PgStageMigrations {}

impl SqlMigrations for PgStageMigrations {
    fn queries() -> Vec<String> {
        let migration_001 = include_str!("./migrations/0001-postgres-init.sql");
        vec![migration_001.to_string()]
    }
}

#[derive(Debug)]
pub struct PostgresState {
    pool: PgPool,
}

impl PostgresState {
    pub fn pool() -> PgPoolOptions {
        PgPoolOptions::new()
    }

    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn migrate(&self) {
        let mut transaction = self.pool.begin().await.unwrap();
        let queries = PgStageMigrations::queries();
        for query in queries {
            sqlx::query(&query)
                .execute(&mut *transaction)
                .await
                .unwrap();
        }
        transaction.commit().await.unwrap();
    }
}

#[async_trait]
impl<T: DeserializeOwned> StateLoader<T> for PostgresState {
    async fn prepare(&self) {
        self.migrate().await;
    }

    async fn load(
        &self,
        object_kind: &str,
        object_id: &str,
        state_type: &str,
    ) -> Result<T, LoadStateError> {
        let items = sqlx::query(
            r#"
            SELECT serialized_state
            FROM state_provider_object_state
            WHERE object_kind=$1 AND object_id=$2 AND state_type = $3
            "#,
        )
        .bind(object_kind)
        .bind(object_id)
        .bind(state_type)
        .map(|x: PgRow| -> Vec<u8> { x.get::<Vec<u8>, _>("serialized_state") })
        .fetch_one(&self.pool)
        .map_err(|_| LoadStateError::ObjectNotFound)
        .await?;
        let items = String::from_utf8(items).map_err(|_| LoadStateError::DeserializationError)?;
        let data =
            serde_json::from_str(&items).map_err(|_| LoadStateError::DeserializationError)?;
        Ok(data)
    }
}

#[async_trait]
impl<T: Serialize + Send + Sync> StateSaver<T> for PostgresState {
    async fn prepare(&self) {
        self.migrate().await;
    }

    async fn save(
        &self,
        object_kind: &str,
        object_id: &str,
        state_type: &str,
        data: &T,
    ) -> Result<(), LoadStateError> {
        let serialized_data =
            serde_json::to_string(&data).map_err(|_| LoadStateError::SerializationError)?;
        sqlx::query(
            r#"
            INSERT INTO
                state_provider_object_state(object_kind, object_id, state_type, serialized_state)
            VALUES ($1, $2, $3, $4)
            ON CONFLICT(object_kind, object_id, state_type) DO UPDATE SET serialized_state=$4
            "#,
        )
        .bind(object_kind)
        .bind(object_id)
        .bind(state_type)
        .bind(serialized_data.bytes().collect::<Vec<_>>())
        .execute(&self.pool)
        .map_err(|e| {
            eprintln!("{:?}", e);
            LoadStateError::Unknown
        })
        .await
        .map(|_| ())
    }
}
