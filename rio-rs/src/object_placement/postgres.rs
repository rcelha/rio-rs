//! SQL implementation of the trait [ObjectPlacement] to work with relational databases
//!
//! This uses [sqlx] under the hood

use async_trait::async_trait;
use sqlx::postgres::PgPoolOptions;
use sqlx::{self, PgPool, Row};

use super::{ObjectPlacement, ObjectPlacementItem};
use crate::sql_migration::SqlMigrations;
use crate::ObjectId;

pub struct PgObjectPlacementMigrations {}

impl SqlMigrations for PgObjectPlacementMigrations {
    fn queries() -> Vec<String> {
        let migrations = include_str!("./migrations/0001-postgres-init.sql")
            .split(";")
            .map(|x| x.to_string())
            .collect();
        migrations
    }
}

#[derive(Clone, Debug)]
pub struct PostgresObjectPlacement {
    pool: PgPool,
}

impl PostgresObjectPlacement {
    pub fn new(pool: PgPool) -> Self {
        PostgresObjectPlacement { pool }
    }

    /// Pool builder, so one doesn't need to include sqlx as a dependency
    ///
    /// # Example
    ///
    /// ```
    /// # use rio_rs::object_placement::postgres::PostgresObjectPlacement;
    /// # async fn test_fn() {
    /// let pool = PostgresObjectPlacement::pool()
    ///     .connect("sqlite::memory:")
    ///     .await
    ///     .expect("Connection failure");
    /// let object_placement = PostgresObjectPlacement::new(pool);
    /// # }
    /// ```
    pub fn pool() -> PgPoolOptions {
        PgPoolOptions::new()
    }
}

#[async_trait]
impl ObjectPlacement for PostgresObjectPlacement {
    /// Run the schema/data migrations for this membership storage.
    ///
    /// For now, the Rio server doesn't run this at start-up and it needs
    /// to be invoked on manually in the server's setup.
    async fn prepare(&self) {
        let mut transaction = self.pool.begin().await.unwrap();
        let queries = PgObjectPlacementMigrations::queries();
        for query in queries {
            sqlx::query(&query)
                .execute(&mut *transaction)
                .await
                .unwrap();
        }
        transaction.commit().await.unwrap();
    }

    async fn update(&self, object_placement: ObjectPlacementItem) {
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

/// TODO - Add tests, using sqlite as reference
#[cfg(test)]
mod test {}
