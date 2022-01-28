//! Rendevouz + Membership APIs to keep a list of running servers

use chrono::{DateTime, TimeZone, Utc};
use dyn_clone::DynClone;
use std::sync::Arc;
use tokio::sync::RwLock;
pub mod sql;

use async_trait::async_trait;

use crate::errors::MembershipError;

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
            last_seen: Utc.timestamp(1_500_000_000, 0),
        }
    }
    pub fn set_active(&mut self, active: bool) {
        self.active = active;
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
    pub fn last_seen(&self) -> &DateTime<Utc> {
        &self.last_seen
    }
    pub fn address(&self) -> String {
        format!("{}:{}", self.ip, self.port)
    }
}

pub type MembershipResult<T> = Result<T, MembershipError>;
pub type MembershipUnitResult = Result<(), MembershipError>;

#[async_trait]
pub trait MembersStorage: Send + Sync + DynClone {
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

dyn_clone::clone_trait_object!(MembersStorage);

type ArcMembers = Arc<RwLock<Vec<Member>>>;
type ArcFailures = Arc<RwLock<Vec<(String, String, DateTime<Utc>)>>>;

#[derive(Clone, Default)]
pub struct LocalStorage {
    members: ArcMembers,
    failures: ArcFailures,
}

#[async_trait]
impl MembersStorage for LocalStorage {
    async fn push(&self, member: Member) -> MembershipUnitResult {
        self.members.write().await.push(member);
        Ok(())
    }

    async fn remove(&self, ip: &str, port: &str) -> MembershipUnitResult {
        let mut guard = self.members.write().await;
        guard.retain(|x| x.ip() == ip && x.port() == port);
        Ok(())
    }

    async fn set_is_active(&self, ip: &str, port: &str, is_active: bool) -> MembershipUnitResult {
        let mut guard = self.members.write().await;
        for i in guard.iter_mut() {
            if i.ip() == ip && i.port() == port {
                i.set_active(is_active);
            }
        }
        Ok(())
    }

    async fn notify_failure(&self, ip: &str, port: &str) -> MembershipUnitResult {
        let now = Utc::now();
        let mut guard = self.failures.write().await;
        guard.push((ip.to_string(), port.to_string(), now));
        Ok(())
    }

    async fn member_failures(&self, ip: &str, port: &str) -> MembershipResult<Vec<DateTime<Utc>>> {
        let guard = self.failures.read().await;
        let items = guard
            .iter()
            .cloned()
            .filter(|(ip_, port_, ..)| ip_ == ip && port_ == port)
            .map(|x| x.2)
            .collect();
        Ok(items)
    }

    async fn members(&self) -> MembershipResult<Vec<Member>> {
        Ok(self.members.read().await.clone())
    }
}

#[cfg(test)]
mod test {
    use super::*;

    async fn storage() -> impl MembersStorage {
        let storage = LocalStorage::default();
        for (ip, port) in [
            ("0.0.0.0", "5000"),
            ("0.0.0.0", "5001"),
            ("0.0.0.0", "5002"),
            ("0.0.0.0", "5003"),
            ("0.0.0.0", "5004"),
            ("0.0.0.0", "5005"),
        ] {
            storage
                .push(Member::new(ip.to_string(), port.to_string()))
                .await
                .unwrap();
        }
        storage
    }

    #[tokio::test]
    async fn sanity_check() {
        let _storage = storage().await;
    }
}
