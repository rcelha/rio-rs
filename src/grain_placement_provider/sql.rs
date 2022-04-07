use async_trait::async_trait;
use sqlx::any::{AnyPool, AnyPoolOptions};
use sqlx::{self, Row};

use super::{GrainPlacement, GrainPlacementProvider};
use crate::GrainId;

pub struct SqlGrainPlacementProvider {
    pool: AnyPool,
}

impl SqlGrainPlacementProvider {
    pub fn pool() -> AnyPoolOptions {
        AnyPoolOptions::new()
    }

    pub fn new(pool: AnyPool) -> Self {
        SqlGrainPlacementProvider { pool }
    }

    pub async fn migrate(&self) {
        let mut transaction = self.pool.begin().await.unwrap();
        sqlx::query(
            r#"CREATE TABLE IF NOT EXISTS grain_placement
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

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_grain_placement_silo_address on grain_placement(silo_address)")
            .execute(&mut transaction)
            .await
            .unwrap();

        transaction.commit().await.unwrap();
    }
}

#[async_trait]
impl GrainPlacementProvider for SqlGrainPlacementProvider {
    async fn update(&self, grain_placement: GrainPlacement) {
        sqlx::query(
            r#"
            INSERT INTO
            grain_placement(struct_name, object_id, silo_address)
            VALUES ($1, $2, $3)
            ON CONFLICT(struct_name, object_id) DO UPDATE SET silo_address=$4"#,
        )
        .bind(&grain_placement.grain_id.0)
        .bind(&grain_placement.grain_id.1)
        .bind(&grain_placement.silo_address)
        .bind(&grain_placement.silo_address)
        .execute(&self.pool)
        .await
        .unwrap();
    }
    async fn lookup(&self, grain_id: &GrainId) -> Option<String> {
        let row = sqlx::query(
            r#"
            SELECT silo_address
            FROM grain_placement
            WHERE struct_name = $1 and object_id = $2
            "#,
        )
        .bind(&grain_id.0)
        .bind(&grain_id.1)
        .fetch_one(&self.pool)
        .await
        .ok();
        row.map(|row| row.get("silo_address"))
    }
    async fn clean_silo(&self, address: String) {
        sqlx::query(
            r#"
            DELETE FROM grain_placement
            WHERE silo_address = $1
            "#,
        )
        .bind(&address)
        .execute(&self.pool)
        .await
        .unwrap();
    }

    async fn remove(&self, grain_id: &GrainId) {
        sqlx::query(
            r#"
            DELETE FROM grain_placement
            WHERE struct_name = $1 and object_id = $2
            "#,
        )
        .bind(&grain_id.0)
        .bind(&grain_id.1)
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

    async fn grain_placement_provider() -> (AnyPool, impl GrainPlacementProvider) {
        let pool = pool().await;
        let grain_placement_provider = SqlGrainPlacementProvider::new(pool.clone());
        grain_placement_provider.migrate().await;
        (pool, grain_placement_provider)
    }

    #[tokio::test]
    async fn test_sanity() {
        let (_, grain_placement_provider) = grain_placement_provider().await;
        let placement = grain_placement_provider
            .lookup(&GrainId::new("Test", "1"))
            .await;
        assert_eq!(placement, None);

        let grain_placement =
            GrainPlacement::new(GrainId::new("Test", "1"), Some("0.0.0.0:5000".to_string()));
        grain_placement_provider.update(grain_placement).await;
        let placement = grain_placement_provider
            .lookup(&GrainId::new("Test", "1"))
            .await
            .unwrap();
        assert_eq!(placement, "0.0.0.0:5000");

        let grain_placement =
            GrainPlacement::new(GrainId::new("Test", "1"), Some("0.0.0.0:5001".to_string()));
        grain_placement_provider.update(grain_placement).await;
        let placement = grain_placement_provider
            .lookup(&GrainId::new("Test", "1"))
            .await
            .unwrap();
        assert_eq!(placement, "0.0.0.0:5001");

        grain_placement_provider
            .clean_silo("0.0.0.0:5001".to_string())
            .await;
        let placement = grain_placement_provider
            .lookup(&GrainId::new("Test", "1"))
            .await;
        assert_eq!(placement, None);
    }
}
