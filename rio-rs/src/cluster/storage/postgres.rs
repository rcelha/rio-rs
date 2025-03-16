//! MembershipStorage implementation to work with relational databases
//!
//! This uses [sqlx] under the hood

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use futures::TryFutureExt;
use sqlx::postgres::{PgPoolOptions, PgRow};
use sqlx::{self, PgPool, Row};

use crate::sql_migration::SqlMigrations;

use super::{Member, MembershipResult, MembershipStorage, MembershipUnitResult};

pub struct PgMembershipStorageMigrations {}

impl SqlMigrations for PgMembershipStorageMigrations {
    fn queries() -> Vec<String> {
        let migrations: Vec<_> = include_str!("./migrations/0001-postgres-init.sql")
            .split(";")
            .map(|x| x.to_string())
            .collect();
        migrations
    }
}

/// MembershipStorage implementation to work with relational databases
#[derive(Clone)]
pub struct PostgresMembershipStorage {
    pool: PgPool,
}

impl PostgresMembershipStorage {
    /// Builds a [SqlMembershipStorage] from a [sqlx]'s [AnyPool]
    pub fn new(pool: PgPool) -> PostgresMembershipStorage {
        PostgresMembershipStorage { pool }
    }

    /// Pool builder, so one doesn't need to include sqlx as a dependency
    ///
    /// # Example
    ///
    /// ```
    /// # use rio_rs::cluster::storage::postgres::PostgresMembershipStorage;
    /// # async fn test_fn() {
    /// let pool = PostgresMembershipStorage::pool()
    ///     .connect("sqlite::memory:")
    ///     .await
    ///     .expect("Connection failure");
    /// let members_storage = PostgresMembershipStorage::new(pool);
    /// # }
    /// ```
    pub fn pool() -> PgPoolOptions {
        PgPoolOptions::new()
    }
}

#[async_trait]
impl MembershipStorage for PostgresMembershipStorage {
    /// Run the schema/data migrations for this membership storage.
    async fn prepare(&self) {
        let mut transaction = self.pool.begin().await.unwrap();
        let queries = PgMembershipStorageMigrations::queries();
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
            .map(|x: PgRow| {
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
            .map(|x: PgRow| x.get("time"))
            .fetch_all(&self.pool)
            .err_into()
            .await
    }
}

/// TODO create the tests based on the sqlite implementation
#[cfg(test)]
mod test {}
