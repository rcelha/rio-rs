//! In-Memory implementation of [MembersStorage]
use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use tokio::sync::RwLock;

use super::{Member, MembersStorage, MembershipResult, MembershipUnitResult};

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
        guard.retain(|x| x.ip() != ip || x.port() != port);
        Ok(())
    }

    async fn set_is_active(&self, ip: &str, port: &str, is_active: bool) -> MembershipUnitResult {
        let last_seen = Utc::now();
        let mut guard = self.members.write().await;
        for i in guard.iter_mut() {
            if i.ip() == ip && i.port() == port {
                i.set_active(is_active);
                i.last_seen = last_seen.clone();
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
            .filter(|(ip_, port_, ..)| ip_ == ip && port_ == port)
            .cloned()
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
    async fn local_storage_clone_keep_members() {
        let storage = storage().await;
        let second_storage = storage.clone();

        let members = storage.members().await.unwrap();
        assert_eq!(members.len(), 6);
        let members = second_storage.members().await.unwrap();
        assert_eq!(members.len(), 6);

        second_storage.remove("0.0.0.0", "5005").await.unwrap();

        let members = storage.members().await.unwrap();
        assert_eq!(members.len(), 5);
        let members = second_storage.members().await.unwrap();
        assert_eq!(members.len(), 5);
    }
}
