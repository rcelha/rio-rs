use async_trait::async_trait;
use rio_rs::{
    derive::TypeName,
    errors::LoadStateError,
    sql_migration::SqlMigrations,
    state::{StateLoader, StateSaver},
};
use serde::{Deserialize, Serialize};
use sqlx::{
    self,
    sqlite::{SqlitePoolOptions, SqliteRow},
    Row, SqlitePool,
};

/// This is the fragment that needs to be associated with your
/// Service
#[derive(Default, Debug, Serialize, Deserialize, TypeName)]
pub struct PingAttributeState {
    pub request_count: u32,
}

pub struct Migrations {}

impl SqlMigrations for Migrations {
    fn queries() -> Vec<String> {
        let migration_001 = r#"
            CREATE TABLE IF NOT EXISTS ping_state
            (
                object_id       TEXT                NOT NULL,
                request_count   UNSIGNED INT        NOT NULL,
                PRIMARY KEY (object_id)
            );
            "#;
        vec![migration_001.to_string()]
    }
}

pub struct PingState {
    pool: SqlitePool,
}

impl PingState {
    pub fn pool() -> SqlitePoolOptions {
        SqlitePoolOptions::new()
    }

    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn migrate(&self) {
        let mut transaction = self.pool.begin().await.unwrap();
        let queries = Migrations::queries();
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
impl StateLoader<PingAttributeState> for PingState {
    async fn prepare(&self) {
        self.migrate().await
    }

    async fn load(
        &self,
        _object_kind: &str,
        object_id: &str,
        _state_type: &str,
    ) -> Result<PingAttributeState, LoadStateError> {
        let items = sqlx::query(
            r#"
            SELECT request_count
            FROM ping_state
            WHERE object_id=$1
            "#,
        )
        .bind(object_id)
        .map(|x: SqliteRow| -> u32 { x.get("request_count") })
        .fetch_one(&self.pool)
        .await
        .map_err(|_| LoadStateError::ObjectNotFound)?;
        let data = PingAttributeState {
            request_count: items,
        };
        Ok(data)
    }
}

#[async_trait]
impl StateSaver<PingAttributeState> for PingState {
    async fn prepare(&self) {
        self.migrate().await
    }

    async fn save(
        &self,
        _object_kind: &str,
        object_id: &str,
        _state_type: &str,
        data: &PingAttributeState,
    ) -> Result<(), LoadStateError> {
        sqlx::query(
            r#"
            INSERT INTO
                ping_state(object_id, request_count)
            VALUES ($1, $2)
            ON CONFLICT(object_id) DO UPDATE SET request_count=$2
            "#,
        )
        .bind(object_id)
        .bind(data.request_count)
        .execute(&self.pool)
        .await
        .map_err(|e| {
            eprintln!("{:?}", e);
            LoadStateError::Unknown
        })?;
        Ok(())
    }
}
