use crate::{errors::LoadStateError, sql_migration::SqlMigrations};
use async_trait::async_trait;
use futures::TryFutureExt;
use serde::{de::DeserializeOwned, Serialize};
use sqlx::{
    self,
    sqlite::{SqlitePoolOptions, SqliteRow},
    Row, SqlitePool,
};

use super::{StateLoader, StateSaver};

pub struct SqliteStateMigrations {}

impl SqlMigrations for SqliteStateMigrations {
    fn queries() -> Vec<String> {
        let migration_001 = include_str!("./migrations/0001-sqlite-init.sql");
        vec![migration_001.to_string()]
    }
}

#[derive(Debug)]
pub struct SqliteState {
    pool: SqlitePool,
}

impl SqliteState {
    pub fn pool() -> SqlitePoolOptions {
        SqlitePoolOptions::new()
    }

    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn migrate(&self) {
        let mut transaction = self.pool.begin().await.unwrap();
        let queries = SqliteStateMigrations::queries();
        for query in queries {
            sqlx::query(&query).execute(&mut transaction).await.unwrap();
        }
        transaction.commit().await.unwrap();
    }
}

#[async_trait]
impl StateLoader for SqliteState {
    async fn prepare(&self) {
        self.migrate().await;
    }

    async fn load<T: DeserializeOwned>(
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
        .map(|x: SqliteRow| -> String {
            let tmp = x.get::<Vec<u8>, _>("serialized_state");
            String::from_utf8(tmp).expect("TODO")
        })
        .fetch_one(&self.pool)
        .map_err(|_| LoadStateError::ObjectNotFound)
        .await?;
        let data = serde_json::from_str(&items).expect("TODO");
        Ok(data)
    }
}

#[async_trait]
impl StateSaver for SqliteState {
    async fn prepare(&self) {
        self.migrate().await;
    }

    async fn save(
        &self,
        object_kind: &str,
        object_id: &str,
        state_type: &str,
        data: &(impl Serialize + Send + Sync),
    ) -> Result<(), LoadStateError> {
        let serialized_data = serde_json::to_string(&data).expect("TODO");
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
