#[cfg(feature = "sql")]
mod db_utils;

use rio_rs::{
    object_placement::{ObjectPlacement, ObjectPlacementProvider},
    ObjectId,
};

async fn no_placement<S: ObjectPlacementProvider>(provider: S) {
    provider.prepare().await;

    let server_addr = provider.lookup(&ObjectId::new("obj", "1")).await;
    assert!(server_addr.is_none());
}

async fn save_and_load<S: ObjectPlacementProvider>(provider: S) {
    provider.prepare().await;

    let obj_id = ObjectId::new("obj", "1");
    let placement = ObjectPlacement::new(obj_id, Some("0.0.0.0:8888".to_string()));
    provider.update(placement).await;

    let server_addr = provider.lookup(&ObjectId::new("obj", "1")).await;
    assert_eq!(server_addr.as_ref().unwrap(), "0.0.0.0:8888");

    provider.clean_server("0.0.0.0:8888".to_string()).await;
    let server_addr = provider.lookup(&ObjectId::new("obj", "1")).await;
    assert!(server_addr.is_none());
}

#[cfg(feature = "redis")]
mod redis {
    use rio_rs::object_placement::redis::RedisObjectPlacementProvider;

    #[tokio::test]
    async fn no_placement() {
        let prefix = chrono::Local::now().timestamp_micros().to_string();
        let provider = RedisObjectPlacementProvider::from_connect_string(
            "redis://localhost:16379",
            Some(prefix),
        )
        .await;
        super::no_placement(provider).await;
    }

    #[tokio::test]
    async fn save_and_load() {
        let prefix = chrono::Local::now().timestamp_micros().to_string();
        let provider = RedisObjectPlacementProvider::from_connect_string(
            "redis://localhost:16379",
            Some(prefix),
        )
        .await;
        super::save_and_load(provider).await;
    }
}

#[cfg(feature = "sqlite")]
mod sqlite {
    use super::db_utils::sqlite::pool;
    use rio_rs::object_placement::sqlite::SqliteObjectPlacementProvider;

    #[tokio::test]
    async fn no_placement() {
        let pool = pool().await;
        let provider = SqliteObjectPlacementProvider::new(pool);
        super::no_placement(provider).await;
    }

    #[tokio::test]
    async fn save_and_load() {
        let pool = pool().await;
        let provider = SqliteObjectPlacementProvider::new(pool);
        super::save_and_load(provider).await;
    }
}

#[cfg(feature = "postgres")]
mod pgsql {
    use super::db_utils::pgsql::pool;
    use rio_rs::object_placement::postgres::PostgresObjectPlacementProvider;

    #[tokio::test]
    async fn no_placement() {
        let pool = pool("no_placement").await;
        let provider = PostgresObjectPlacementProvider::new(pool);
        super::no_placement(provider).await;
    }

    #[tokio::test]
    async fn save_and_load() {
        let pool = pool("save_and_load").await;
        let provider = PostgresObjectPlacementProvider::new(pool);
        super::save_and_load(provider).await;
    }
}

#[cfg(feature = "local")]
mod local {
    use rio_rs::object_placement::local::LocalObjectPlacementProvider;

    #[tokio::test]
    async fn no_placement() {
        let provider = LocalObjectPlacementProvider::default();
        super::no_placement(provider).await;
    }

    #[tokio::test]
    async fn save_and_load() {
        let provider = LocalObjectPlacementProvider::default();
        super::save_and_load(provider).await;
    }
}
