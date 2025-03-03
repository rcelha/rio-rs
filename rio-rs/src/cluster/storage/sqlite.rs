//! MembersStorage implementation to work with relational databases
//!
//! This uses [sqlx] under the hood

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use futures::TryFutureExt;
use sqlx::sqlite::{SqlitePoolOptions, SqliteRow};
use sqlx::{self, Row, SqlitePool};

use crate::sql_migration::SqlMigrations;

use super::{Member, MembersStorage, MembershipResult, MembershipUnitResult};

pub struct SqliteMembersStorageMigrations {}

impl SqlMigrations for SqliteMembersStorageMigrations {
    fn queries() -> Vec<String> {
        let migrations: Vec<_> = include_str!("./migrations/0001-sqlite-init.sql")
            .split(";")
            .map(|x| x.to_string())
            .collect();
        migrations
    }
}

/// MembersStorage implementation to work with relational databases
#[derive(Clone)]
pub struct SqliteMembersStorage {
    pool: SqlitePool,
}

impl SqliteMembersStorage {
    /// Builds a [SqlMembersStorage] from a [sqlx]'s [AnyPool]
    pub fn new(pool: SqlitePool) -> SqliteMembersStorage {
        SqliteMembersStorage { pool }
    }

    /// Pool builder, so one doesn't need to include sqlx as a dependency
    ///
    /// # Example
    ///
    /// ```
    /// # use rio_rs::cluster::storage::sqlite::SqliteMembersStorage;
    /// # async fn test_fn() {
    /// let pool = SqliteMembersStorage::pool()
    ///     .connect("sqlite::memory:")
    ///     .await
    ///     .expect("Connection failure");
    /// let members_storage = SqliteMembersStorage::new(pool);
    /// # }
    /// ```
    pub fn pool() -> SqlitePoolOptions {
        SqlitePoolOptions::new()
    }
}

#[async_trait]
impl MembersStorage for SqliteMembersStorage {
    /// Run the schema/data migrations for this membership storage.
    async fn prepare(&self) {
        let mut transaction = self.pool.begin().await.unwrap();
        let queries = SqliteMembersStorageMigrations::queries();

        for query in queries {
            sqlx::query(&query)
                .execute(&mut *transaction)
                .await
                .unwrap();
        }
        transaction.commit().await.unwrap();
    }

    async fn push(&self, member: Member) -> MembershipUnitResult {
        let last_seen = Utc::now();
        sqlx::query(
            r#"
            INSERT INTO
                cluster_provider_members (ip, port, last_seen, active)
            VALUES ($1, $2, $3, $4)
            ON CONFLICT(ip, port) DO UPDATE SET last_seen=$3, active=$4
            "#,
        )
        .bind(member.ip)
        .bind(member.port)
        .bind(last_seen)
        .bind(member.active)
        .execute(&self.pool)
        .err_into()
        .await
        .map(|_| ())
    }

    async fn remove(&self, ip: &str, port: &str) -> MembershipUnitResult {
        sqlx::query("DELETE FROM cluster_provider_members WHERE ip = $1 AND port = $2")
            .bind(ip)
            .bind(port)
            .execute(&self.pool)
            .err_into()
            .await
            .map(|_| ())
    }

    async fn set_is_active(&self, ip: &str, port: &str, is_active: bool) -> MembershipUnitResult {
        let last_seen = Utc::now();
        sqlx::query("UPDATE cluster_provider_members SET active = $3, last_seen = $4 WHERE ip = $1 and port = $2")
            .bind(ip)
            .bind(port)
            .bind(is_active)
            .bind(last_seen)
            .execute(&self.pool)
            .err_into()
            .await
            .map(|_| ())
    }

    async fn members(&self) -> MembershipResult<Vec<Member>> {
        let items = sqlx::query(
            "SELECT ip, port, active, last_seen FROM cluster_provider_members ORDER BY last_seen DESC",
        )
        .fetch_all(&self.pool)
        .await?;

        let items = items
            .iter()
            .map(|x| {
                let mut new_item = Member::new(x.get("ip"), x.get("port"));
                new_item.last_seen = x.get("last_seen");
                new_item.set_active(x.get("active"));
                new_item
            })
            .collect();
        Ok(items)
    }

    async fn active_members(&self) -> MembershipResult<Vec<Member>> {
        let items = sqlx::query("SELECT ip, port, active, last_seen FROM cluster_provider_members WHERE active ORDER BY last_seen DESC")
            .map(|x: SqliteRow| {
                let mut new_item = Member::new(x.get("ip"), x.get("port"));
                new_item.last_seen = x.get("last_seen");
                new_item.set_active(x.get("active"));
                new_item
            })
            .fetch_all(&self.pool)
            .await?;
        Ok(items)
    }

    async fn notify_failure(&self, ip: &str, port: &str) -> MembershipUnitResult {
        let query = r#"
            INSERT INTO
                cluster_provider_member_failures (ip, port)
            VALUES ($1, $2)
        "#;
        sqlx::query(query)
            .bind(ip)
            .bind(port)
            .execute(&self.pool)
            .err_into()
            .await
            .map(|_| ())
    }

    /// TODO configure LIMIT
    async fn member_failures(&self, ip: &str, port: &str) -> MembershipResult<Vec<DateTime<Utc>>> {
        let query = r#"
            SELECT time FROM
                cluster_provider_member_failures
            WHERE ip = $1 AND port = $2
            ORDER BY time DESC LIMIT 100
        "#;
        sqlx::query(query)
            .bind(ip)
            .bind(port)
            .map(|x: SqliteRow| x.get("time"))
            .fetch_all(&self.pool)
            .err_into()
            .await
    }
}

#[cfg(test)]
mod test {
    use super::*;

    async fn members_storage() -> impl MembersStorage {
        let pool = SqliteMembersStorage::pool()
            .connect("sqlite::memory:")
            .await
            .expect("TODO: Connection failure");
        let members_storage = SqliteMembersStorage::new(pool);
        members_storage.prepare().await;
        members_storage
    }

    async fn members_with_value() -> impl MembersStorage {
        let members_storage = members_storage().await;
        let mut active_member = Member::new("0.0.0.0".to_string(), "5000".to_string());
        active_member.set_active(true);
        members_storage.push(active_member).await.unwrap();
        members_storage
            .push(Member::new("0.0.0.0".to_string(), "5001".to_string()))
            .await
            .unwrap();
        members_storage
    }

    #[tokio::test]
    async fn test_insert() {
        let members_storage = members_storage().await;
        members_storage
            .push(Member::new("0.0.0.0".to_string(), "5000".to_string()))
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn test_list() {
        let members_storage = members_with_value().await;
        let members = members_storage.members().await.unwrap();
        assert_eq!(members.len(), 2);
    }

    #[tokio::test]
    async fn test_remove() {
        let members_storage = members_with_value().await;
        members_storage.remove("0.0.0.0", "5001").await.unwrap();
        let members = members_storage.members().await.unwrap();
        assert_eq!(members.len(), 1);
    }

    #[tokio::test]
    async fn test_get_active() {
        let members_storage = members_with_value().await;
        let members = members_storage.active_members().await.unwrap();
        assert_eq!(members.len(), 1);
    }

    #[tokio::test]
    async fn test_insert_update_ttl() {
        let members_storage = members_storage().await;
        let t0 = Utc::now();

        let mut active_member = Member::new("0.0.0.0".to_string(), "5000".to_string());
        active_member.set_active(true);
        members_storage.push(active_member).await.unwrap();
        let members = members_storage.members().await.unwrap();
        let member = members.first().unwrap();

        let t1 = Utc::now();
        assert!(member.last_seen() > &t0);
        assert!(member.last_seen() < &t1);
    }
}
