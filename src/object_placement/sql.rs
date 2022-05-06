use async_trait::async_trait;
use sqlx::any::{AnyPool, AnyPoolOptions};
use sqlx::{self, Row};

use super::{ObjectPlacement, ObjectPlacementProvider};
use crate::ObjectId;

pub struct SqlObjectPlacementProvider {
    pool: AnyPool,
}

impl SqlObjectPlacementProvider {
    pub fn pool() -> AnyPoolOptions {
        AnyPoolOptions::new()
    }

    pub fn new(pool: AnyPool) -> Self {
        SqlObjectPlacementProvider { pool }
    }

    pub async fn migrate(&self) {
        let mut transaction = self.pool.begin().await.unwrap();
        sqlx::query(
            r#"CREATE TABLE IF NOT EXISTS object_placement
            (
                struct_name     TEXT                NOT NULL,
                object_id       TEXT                NOT NULL,
                silo_address    TEXT                NULL,

                PRIMARY KEY (struct_name, object_id)
            )"#,
        )
        .execute(&mut transaction)
        .await
        .unwrap();

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_object_placement_silo_address on object_placement(silo_address)")
            .execute(&mut transaction)
            .await
            .unwrap();

        transaction.commit().await.unwrap();
    }
}

#[async_trait]
impl ObjectPlacementProvider for SqlObjectPlacementProvider {
    async fn update(&self, object_placement: ObjectPlacement) {
        sqlx::query(
            r#"
            INSERT INTO
            object_placement(struct_name, object_id, silo_address)
            VALUES ($1, $2, $3)
            ON CONFLICT(struct_name, object_id) DO UPDATE SET silo_address=$4"#,
        )
        .bind(&object_placement.object_id.0)
        .bind(&object_placement.object_id.1)
        .bind(&object_placement.silo_address)
        .bind(&object_placement.silo_address)
        .execute(&self.pool)
        .await
        .unwrap();
    }
    async fn lookup(&self, object_id: &ObjectId) -> Option<String> {
        let row = sqlx::query(
            r#"
            SELECT silo_address
            FROM object_placement
            WHERE struct_name = $1 and object_id = $2
            "#,
        )
        .bind(&object_id.0)
        .bind(&object_id.1)
        .fetch_one(&self.pool)
        .await
        .ok();
        row.map(|row| row.get("silo_address"))
    }
    async fn clean_silo(&self, address: String) {
        sqlx::query(
            r#"
            DELETE FROM object_placement
            WHERE silo_address = $1
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
    use sqlx::any::AnyPoolOptions;

    use super::*;

    async fn pool() -> AnyPool {
        AnyPoolOptions::new()
            .max_connections(5)
            .connect("sqlite::memory:")
            .await
            .expect("TODO: Connection failure")
    }

    async fn object_placement_provider() -> (AnyPool, impl ObjectPlacementProvider) {
        let pool = pool().await;
        let object_placement_provider = SqlObjectPlacementProvider::new(pool.clone());
        object_placement_provider.migrate().await;
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
            .clean_silo("0.0.0.0:5001".to_string())
            .await;
        let placement = object_placement_provider
            .lookup(&ObjectId::new("Test", "1"))
            .await;
        assert_eq!(placement, None);
    }
}
