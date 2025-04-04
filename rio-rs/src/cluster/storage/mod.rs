use async_trait::async_trait;
use chrono::{DateTime, TimeZone, Utc};

use crate::errors::MembershipError;

#[cfg(feature = "http")]
pub mod http;
#[cfg(feature = "local")]
pub mod local;
#[cfg(feature = "postgres")]
pub mod postgres;
#[cfg(feature = "redis")]
pub mod redis;
#[cfg(feature = "sqlite")]
pub mod sqlite;

/// Represents a running [Server](crate::server::Server).
#[derive(Clone, Debug)]
pub struct Member {
    ip: String,
    port: String,
    active: bool,
    last_seen: DateTime<Utc>,
}

impl Member {
    pub fn new(ip: String, port: String) -> Member {
        Member {
            ip,
            port,
            active: false,
            last_seen: Utc.timestamp_opt(0, 0).unwrap(),
        }
    }
    pub fn ip(&self) -> &str {
        &self.ip
    }
    pub fn port(&self) -> &str {
        &self.port
    }
    pub fn active(&self) -> bool {
        self.active
    }
    pub fn set_active(&mut self, active: bool) {
        self.active = active;
    }
    pub fn last_seen(&self) -> &DateTime<Utc> {
        &self.last_seen
    }
    pub fn set_last_seen(&mut self) {
        let now = Utc::now();
        self.last_seen = now
    }
    pub fn address(&self) -> String {
        format!("{}:{}", self.ip, self.port)
    }
}

pub type MembershipResult<T> = Result<T, MembershipError>;
pub type MembershipUnitResult = Result<(), MembershipError>;

/// `MembersshipStorage` is a trait describing how to manage a list of servers and their respective
/// status.
///
/// It is **not** reponsible for asserting the nodes' state, only to store their state.
/// This is done by the [crate::cluster::membership_protocol::ClusterProvider]
#[async_trait]
pub trait MembershipStorage: Send + Sync + Clone {
    async fn prepare(&self) {}

    /// Saves a new member to the storage
    async fn push(&self, member: Member) -> MembershipUnitResult;

    /// Remove a member by its public ip + port identification
    async fn remove(&self, ip: &str, port: &str) -> MembershipUnitResult;

    /// Changes status for a given Member (lookup by public ip + port)
    async fn set_is_active(&self, ip: &str, port: &str, is_active: bool) -> MembershipUnitResult;

    /// List all members in the storage
    async fn members(&self) -> MembershipResult<Vec<Member>>;

    /// Flag a failure to a given member. Note this method doesn't change the member's activity
    /// status
    async fn notify_failure(&self, ip: &str, port: &str) -> MembershipUnitResult;

    /// List all failures of a given member
    ///
    /// TODO: Limit
    async fn member_failures(&self, ip: &str, port: &str) -> MembershipResult<Vec<DateTime<Utc>>>;

    /// List of active members only
    async fn active_members(&self) -> MembershipResult<Vec<Member>> {
        let mut members = self.members().await?;
        members.retain(|x| x.active);
        Ok(members)
    }

    /// Tests a member inactive (loopkup by ip + port)
    async fn is_active(&self, ip: &str, port: &str) -> MembershipResult<bool> {
        let active_members = self.active_members().await?;
        for member in active_members {
            if member.ip == ip && member.port == port {
                return Ok(true);
            }
        }
        Ok(false)
    }

    /// Sets a member inactive (loopkup by ip + port)
    async fn set_inactive(&self, ip: &str, port: &str) -> MembershipUnitResult {
        self.set_is_active(ip, port, false).await
    }

    /// Sets a member active (loopkup by ip + port)
    async fn set_active(&self, ip: &str, port: &str) -> MembershipUnitResult {
        self.set_is_active(ip, port, true).await
    }
}
