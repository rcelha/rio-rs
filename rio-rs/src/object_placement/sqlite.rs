//! SQL implementation of the trait [ObjectPlacementProvider] to work with relational databases
//!
//! This uses [sqlx] under the hood

use async_trait::async_trait;
use sqlx::sqlite::SqlitePoolOptions;
use sqlx::{self, Row, SqlitePool};

use super::{ObjectPlacement, ObjectPlacementProvider};
use crate::sql_migration::SqlMigrations;
use crate::ObjectId;

pub struct SqliteObjectPlacementMigrations {}

impl SqlMigrations for SqliteObjectPlacementMigrations {
    fn queries() -> Vec<String> {
        let migration_001 = include_str!("./migrations/0001-sqlite-init.sql");
        vec![migration_001.to_string()]
    }
}

#[derive(Clone)]
pub struct SqliteObjectPlacementProvider {
    pool: SqlitePool,
}

impl SqliteObjectPlacementProvider {
    pub fn new(pool: SqlitePool) -> Self {
        SqliteObjectPlacementProvider { pool }
    }

    /// Pool builder, so one doesn't need to include sqlx as a dependency
    ///
    /// # Example
    ///
    /// ```
    /// # use rio_rs::object_placement::sqlite::SqliteObjectPlacementProvider;
    /// # async fn test_fn() {
    /// let pool = SqliteObjectPlacementProvider::pool()
    ///     .connect("sqlite::memory:")
    ///     .await
    ///     .expect("Connection failure");
    /// let object_placement = SqliteObjectPlacementProvider::new(pool);
    /// # }
    /// ```
    pub fn pool() -> SqlitePoolOptions {
        SqlitePoolOptions::new()
    }
}

#[async_trait]
impl ObjectPlacementProvider for SqliteObjectPlacementProvider {
    /// Run the schema/data migrations for this membership storage.
    ///
    /// For now, the Rio server doesn't run this at start-up and it needs
    /// to be invoked on manually in the server's setup.
    async fn prepare(&self) {
        let mut transaction = self.pool.begin().await.unwrap();
        let queries = SqliteObjectPlacementMigrations::queries();
        for query in queries {
            sqlx::query(&query)
                .execute(&mut *transaction)
                .await
                .unwrap();
        }
        transaction.commit().await.unwrap();
    }

    async fn update(&self, object_placement: ObjectPlacement) {
        sqlx::query(
            r#"
            INSERT INTO
            object_placement(struct_name, object_id, server_address)
            VALUES ($1, $2, $3)
            ON CONFLICT(struct_name, object_id) DO UPDATE SET server_address=$3"#,
        )
        .bind(&object_placement.object_id.0)
        .bind(&object_placement.object_id.1)
        .bind(&object_placement.server_address)
        .execute(&self.pool)
        .await
        .unwrap();
    }
    async fn lookup(&self, object_id: &ObjectId) -> Option<String> {
        let row = sqlx::query(
            r#"
            SELECT server_address
            FROM object_placement
            WHERE struct_name = $1 and object_id = $2
            "#,
        )
        .bind(&object_id.0)
        .bind(&object_id.1)
        .fetch_one(&self.pool)
        .await
        .ok();
        row.map(|row| row.get("server_address"))
    }
    async fn clean_server(&self, address: String) {
        sqlx::query(
            r#"
            DELETE FROM object_placement
            WHERE server_address = $1
            "#,
        )
        .bind(&address)
        .execute(&self.pool)
        .await
        .unwrap();
    }

    async fn remove(&self, object_id: &ObjectId) {
        sqlx::query(
            r#"
            DELETE FROM object_placement
            WHERE struct_name = $1 and object_id = $2
            "#,
        )
        .bind(&object_id.0)
        .bind(&object_id.1)
        .execute(&self.pool)
        .await
        .unwrap();
    }
}

#[cfg(test)]
mod test {

    use super::*;

    async fn pool() -> SqlitePool {
        SqlitePoolOptions::new()
            .max_connections(5)
            .connect("sqlite::memory:")
            .await
            .expect("TODO: Connection failure")
    }

    async fn object_placement_provider() -> (SqlitePool, impl ObjectPlacementProvider) {
        let pool = pool().await;
        let object_placement_provider = SqliteObjectPlacementProvider::new(pool.clone());
        object_placement_provider.prepare().await;
        (pool, object_placement_provider)
    }

    #[tokio::test]
    async fn test_sanity() {
        let (_, object_placement_provider) = object_placement_provider().await;
        let placement = object_placement_provider
            .lookup(&ObjectId::new("Test", "1"))
            .await;
        assert_eq!(placement, None);

        let object_placement =
            ObjectPlacement::new(ObjectId::new("Test", "1"), Some("0.0.0.0:5000".to_string()));
        object_placement_provider.update(object_placement).await;
        let placement = object_placement_provider
            .lookup(&ObjectId::new("Test", "1"))
            .await
            .unwrap();
        assert_eq!(placement, "0.0.0.0:5000");

        let object_placement =
            ObjectPlacement::new(ObjectId::new("Test", "1"), Some("0.0.0.0:5001".to_string()));
        object_placement_provider.update(object_placement).await;
        let placement = object_placement_provider
            .lookup(&ObjectId::new("Test", "1"))
            .await
            .unwrap();
        assert_eq!(placement, "0.0.0.0:5001");

        object_placement_provider
            .clean_server("0.0.0.0:5001".to_string())
            .await;
        let placement = object_placement_provider
            .lookup(&ObjectId::new("Test", "1"))
            .await;
        assert_eq!(placement, None);
    }
}
