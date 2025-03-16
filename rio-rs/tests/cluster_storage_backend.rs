use rio_rs::{cluster::storage::Member, prelude::MembershipStorage};

#[cfg(any(feature = "sqlite", feature = "postgres"))]
mod db_utils;

async fn members_sanity_check<T: MembershipStorage>(storage: T) {
    storage.prepare().await;

    let members = storage.members().await.unwrap();
    assert_eq!(members.len(), 0);

    storage
        .push(Member::new("0.0.0.0".to_string(), "9090".to_string()))
        .await
        .unwrap();

    let members = storage.members().await.unwrap();
    assert_eq!(members.len(), 1);

    storage.set_active("0.0.0.0", "9090").await.unwrap();
    let members = storage.active_members().await.unwrap();
    assert_eq!(members.len(), 1);

    storage.set_inactive("0.0.0.0", "9090").await.unwrap();
    let members = storage.active_members().await.unwrap();
    assert_eq!(members.len(), 0);
}

async fn failures_sanity_check<T: MembershipStorage>(storage: T) {
    storage.prepare().await;

    let failures = storage.member_failures("0.0.0.0", "9090").await.unwrap();
    assert_eq!(failures.len(), 0);

    storage.notify_failure("0.0.0.0", "9090").await.unwrap();

    let failures = storage.member_failures("0.0.0.0", "9090").await.unwrap();
    assert_eq!(failures.len(), 1);
}

#[cfg(feature = "redis")]
mod redis {
    use rio_rs::cluster::storage::redis::RedisMembershipStorage;

    #[tokio::test]
    async fn members_sanity_check() {
        let prefix = chrono::Local::now().timestamp().to_string();
        let storage =
            RedisMembershipStorage::from_connect_string("redis://localhost:16379", Some(prefix))
                .await;
        super::members_sanity_check(storage).await;
    }

    #[tokio::test]
    async fn failures_sanity_check() {
        let prefix = chrono::Local::now().timestamp().to_string();
        let storage =
            RedisMembershipStorage::from_connect_string("redis://localhost:16379", Some(prefix))
                .await;
        super::failures_sanity_check(storage).await;
    }
}

#[cfg(feature = "sql")]
mod sqlite {
    use super::db_utils::sqlite::pool;
    use rio_rs::cluster::storage::sqlite::SqliteMembershipStorage;

    #[tokio::test]
    async fn members_sanity_check() {
        let pool = pool().await;
        let storage = SqliteMembershipStorage::new(pool);
        super::members_sanity_check(storage).await;
    }

    #[tokio::test]
    async fn failures_sanity_check() {
        let pool = SqliteMembershipStorage::pool()
            .connect("sqlite://:memory:")
            .await
            .unwrap();
        let storage = SqliteMembershipStorage::new(pool);
        super::failures_sanity_check(storage).await;
    }
}

#[cfg(feature = "sql")]
mod pgsql {
    use super::db_utils::pgsql::pool;
    use rio_rs::cluster::storage::postgres::PostgresMembershipStorage;

    #[tokio::test]
    async fn members_sanity_check() {
        let pool = pool("members_sanity_check").await;
        let storage = PostgresMembershipStorage::new(pool);
        super::members_sanity_check(storage).await;
    }

    #[tokio::test]
    async fn failures_sanity_check() {
        let pool = pool("failure_sanity_check").await;
        let storage = PostgresMembershipStorage::new(pool);
        super::failures_sanity_check(storage).await;
    }
}
