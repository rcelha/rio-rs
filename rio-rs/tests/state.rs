use rio_rs::errors::LoadStateError;
use rio_rs::state::{StateLoader, StateSaver};
use serde::{Deserialize, Serialize};

#[cfg(feature = "sql")]
mod db_utils;

#[derive(Serialize, Deserialize, PartialEq, Debug)]
struct State {
    id: usize,
    name: String,
    labels: Vec<String>,
}

async fn state_save_sanity_check<S: StateSaver<State> + StateLoader<State>>(storage: S) {
    StateSaver::prepare(&storage).await;

    let state = State {
        id: 123,
        name: "Something".to_string(),
        labels: vec!["lbl1".to_string(), "lbl2".to_string()],
    };
    storage
        .save("ObjectWithState", "123", "state_attr", &state)
        .await
        .unwrap();
    let loaded_state: State = storage
        .load("ObjectWithState", "123", "state_attr")
        .await
        .unwrap();
    assert_eq!(state, loaded_state);
}

async fn state_load_not_found<S: StateSaver<State> + StateLoader<State>>(storage: S) {
    StateSaver::prepare(&storage).await;

    let loaded_state: Result<State, _> = storage.load("ObjectWithState", "999", "state_attr").await;
    assert_eq!(loaded_state, Err(LoadStateError::ObjectNotFound));
}

#[cfg(feature = "redis")]
mod redis {
    use rio_rs::state::redis::RedisState;

    #[tokio::test]
    async fn state_save_sanity_check() {
        let prefix = chrono::Local::now().timestamp_micros().to_string();
        let storage =
            RedisState::from_connect_string("redis://localhost:16379", Some(prefix)).await;
        super::state_save_sanity_check(storage).await;
    }

    #[tokio::test]
    async fn state_load_not_found() {
        let prefix = chrono::Local::now().timestamp_micros().to_string();
        let storage =
            RedisState::from_connect_string("redis://localhost:16379", Some(prefix)).await;
        super::state_load_not_found(storage).await;
    }
}

#[cfg(feature = "sqlite")]
mod sqlite {
    use rio_rs::state::sqlite::SqliteState;

    use super::db_utils::sqlite::pool;

    #[tokio::test]
    async fn state_save_sanity_check() {
        let pool = pool().await;
        let storage = SqliteState::new(pool);
        super::state_save_sanity_check(storage).await;
    }

    #[tokio::test]
    async fn state_load_not_found() {
        let pool = pool().await;
        let storage = SqliteState::new(pool);
        super::state_load_not_found(storage).await;
    }
}

#[cfg(feature = "postgres")]
mod pgsql {
    use rio_rs::state::postgres::PostgresState;

    use super::db_utils::pgsql::pool;

    #[tokio::test]
    async fn state_save_sanity_check() {
        let pool = pool("state_save_sanity_check").await;
        let storage = PostgresState::new(pool);
        super::state_save_sanity_check(storage).await;
    }

    #[tokio::test]
    async fn state_load_not_found() {
        let pool = pool("state_load_not_found").await;
        let storage = PostgresState::new(pool);
        super::state_load_not_found(storage).await;
    }
}

#[cfg(feature = "local")]
mod local {
    use rio_rs::state::local::LocalState;

    #[tokio::test]
    async fn state_save_sanity_check() {
        let storage = LocalState::default();
        super::state_save_sanity_check(storage).await;
    }

    #[tokio::test]
    async fn state_load_not_found() {
        let storage = LocalState::default();
        super::state_load_not_found(storage).await;
    }
}
