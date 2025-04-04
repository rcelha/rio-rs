//! ClusterProvider that uses peer-to-peer communication to identify which
//! nodes are healthy.
//!
//! This is a gossip based protocol, similar to the one described in Orleans (MS).

use async_trait::async_trait;
use chrono::Utc;
use log::debug;
use std::time::{Duration, SystemTime};
use std::{net::SocketAddr, str::FromStr};

use crate::client::Client;
use crate::cluster::membership_protocol::ClusterProvider;
use crate::cluster::storage::local::LocalStorage;
use crate::cluster::storage::{Member, MembershipStorage};
use crate::errors::ClusterProviderServeError;

/// Marks a node as inactive if we have more than `num_failures_threshold` in the past
/// `interinterval_secs_threshold` seconds
#[derive(Clone)]
pub struct PeerToPeerClusterConfig {
    pub interval_secs: u64,
    pub num_failures_threshold: u64,
    pub interval_secs_threshold: u64,
    pub limit_monitored_members: Option<usize>,
    pub drop_inactive_after_secs: Option<usize>,
}

impl Default for PeerToPeerClusterConfig {
    fn default() -> Self {
        PeerToPeerClusterConfig {
            interval_secs: 10,
            num_failures_threshold: 3,
            interval_secs_threshold: 60,
            limit_monitored_members: None,
            drop_inactive_after_secs: None,
        }
    }
}

impl PeerToPeerClusterConfig {
    pub fn new() -> PeerToPeerClusterConfig {
        Self::default()
    }
}

/// Gossip-based [ClusterProvider]
#[derive(Clone)]
pub struct PeerToPeerClusterProvider<T>
where
    T: MembershipStorage,
{
    members_storage: T,
    config: PeerToPeerClusterConfig,
}

impl<T> PeerToPeerClusterProvider<T>
where
    T: MembershipStorage,
{
    pub fn new(
        members_storage: T,
        config: PeerToPeerClusterConfig,
    ) -> PeerToPeerClusterProvider<T> {
        PeerToPeerClusterProvider {
            members_storage,
            config,
        }
    }

    async fn get_sorted_members(&self) -> Result<Vec<Member>, ClusterProviderServeError> {
        let mut members = self.members_storage().members().await?;
        members.sort_by_key(|x| x.address());
        Ok(members)
    }

    /// TODO docs
    fn get_members_to_monitor(&self, address: &str, sorted_members: &[Member]) -> Vec<Member> {
        let limit_monitored_members = self.config.limit_monitored_members;
        let mut visited = 0;

        sorted_members
            .iter()
            .filter(|x| x.address() != address)
            .map_while(|x| {
                if let Some(amount_to_monitor) = limit_monitored_members {
                    if visited >= amount_to_monitor {
                        None
                    } else {
                        visited += 1;
                        Some(x.clone())
                    }
                } else {
                    visited += 1;
                    Some(x.clone())
                }
            })
            .collect()
    }

    /// TODO remove very old servers from the list
    async fn test_member(&self, member: &Member) -> Result<(), ClusterProviderServeError> {
        // Client needs a MembershipStorage, so we create a in-memory one
        // for local use only
        let local_storage = LocalStorage::default();
        local_storage.push(member.clone()).await?;
        let mut client = Client::new(local_storage);

        let ping = client.ping().await;
        if ping.is_err() {
            self.members_storage()
                .notify_failure(member.ip(), member.port())
                .await?;
        }
        Ok(())
    }

    /// Marks a node as inactive if we have more than `num_failures_threshold` in the past
    /// `interinterval_secs_threshold` seconds
    ///
    /// TODO review number conversions inside
    async fn is_broken(&self, member: &Member) -> Result<bool, ClusterProviderServeError> {
        let t0 = Utc::now() - chrono::Duration::seconds(self.config.interval_secs_threshold as i64);

        let failures = self
            .members_storage()
            .member_failures(member.ip(), member.port())
            .await?;

        let failures_over_threshold = failures.iter().filter(|&time| time > &t0).count() as u64
            >= self.config.num_failures_threshold;
        Ok(failures_over_threshold)
    }
}

#[async_trait]
impl<T> ClusterProvider<T> for PeerToPeerClusterProvider<T>
where
    T: MembershipStorage,
{
    fn members_storage(&self) -> &T {
        &self.members_storage
    }

    /// Membership Algorithm
    ///
    /// At every `self.config.interval_secs` the server runs a check agains each server in the
    /// cluster
    ///
    /// It creates a task for each cluster member, each task will test connectivity for a given
    /// member and update its state in the storage
    ///
    /// If the test fails `self.config.num_failures_threshold` times, the member is flagged
    /// as inactive in the `MembershipStorage`
    ///
    ///
    /// <div class="warning">
    ///
    /// # TODO
    ///
    /// 1. _If communication with MembershipStorage fails, this server should be able to keep running_
    /// 1. _It shouldn't bring dead servers back to life_
    ///
    /// </div>
    async fn serve(&self, address: &str) -> Result<(), ClusterProviderServeError> {
        let sleep_period = std::time::Duration::from_secs(self.config.interval_secs);
        let socket_address = SocketAddr::from_str(address)
            .or(Err(ClusterProviderServeError::SocketAddrParsingError))?;
        let ip = socket_address.ip().to_string();
        let port = socket_address.port().to_string();

        let mut self_member = Member::new(ip, port);
        self_member.set_active(true);
        self_member.set_last_seen();
        self.members_storage().push(self_member).await?;

        loop {
            let members = self.get_sorted_members().await?;
            let test_members = self.get_members_to_monitor(address, &members);
            let t0 = SystemTime::now();

            // Tests reachability and talks to the MembershipStorage to set
            // servers as active or inactive
            let mut future_member_tests = vec![];

            for test_member in test_members.clone().into_iter() {
                let fut = async move {
                    self.test_member(&test_member).await?;
                    let is_broken = self.is_broken(&test_member).await?;

                    if is_broken {
                        self.members_storage()
                            .set_inactive(test_member.ip(), test_member.port())
                            .await?;

                        if let Some(drop_inactive_after_secs) = self.config.drop_inactive_after_secs
                        {
                            let now = Utc::now();
                            let drop_threshold =
                                now - Duration::from_secs(drop_inactive_after_secs as u64);

                            if test_member.last_seen() < &drop_threshold {
                                self.members_storage()
                                    .remove(test_member.ip(), test_member.port())
                                    .await?;
                            }
                        }

                        return Ok::<_, ClusterProviderServeError>((test_member, false));
                    } else if !test_member.active() {
                        self.members_storage()
                            .set_active(test_member.ip(), test_member.port())
                            .await?;
                    }
                    debug!("[{}] {:?} is OK ", address, test_member.address());
                    Ok::<_, ClusterProviderServeError>((test_member, true))
                };
                future_member_tests.push(fut);
            }
            let states = futures::future::join_all(future_member_tests).await;
            debug!("[{}] STATES={:?}", address, states);

            // Wait for the remaining of 'config.interval_secs'
            let elapsed = t0.elapsed().expect("Fail to get elapsed time");
            let remaning_sleep_period = sleep_period.saturating_sub(elapsed);
            debug!("[{}] - Time delta {:?}", address, remaning_sleep_period);
            if remaning_sleep_period > Duration::ZERO {
                tokio::time::sleep(remaning_sleep_period).await;
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::collections::HashMap;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    async fn storage() -> impl MembershipStorage {
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
    async fn test_member_records_member_failure() -> TestResult {
        let storage = LocalStorage::default();
        let membership =
            PeerToPeerClusterProvider::new(storage, PeerToPeerClusterConfig::default());
        let failures = membership
            .members_storage()
            .member_failures("0.0.0.0", "-1")
            .await
            .unwrap();
        assert_eq!(failures.len(), 0);

        membership
            .test_member(&Member::new("0.0.0.0".to_string(), "-1".to_string()))
            .await?;
        let failures = membership
            .members_storage()
            .member_failures("0.0.0.0", "-1")
            .await?;
        assert_eq!(failures.len(), 1);

        membership
            .test_member(&Member::new("0.0.0.0".to_string(), "-1".to_string()))
            .await?;
        let failures = membership
            .members_storage()
            .member_failures("0.0.0.0", "-1")
            .await?;
        assert_eq!(failures.len(), 2);
        Ok(())
    }

    #[tokio::test]
    async fn test_is_broken() -> TestResult {
        let storage = LocalStorage::default();
        storage.notify_failure("0.0.0.0", "5000").await?;
        storage.notify_failure("0.0.0.0", "5000").await?;
        storage.notify_failure("0.0.0.0", "5001").await?;

        let mut config = PeerToPeerClusterConfig::default();
        let membership = PeerToPeerClusterProvider::new(storage.clone(), config.clone());

        let is_broken = membership
            .is_broken(&Member::new("0.0.0.0".to_string(), "5000".to_string()))
            .await;
        assert!(!is_broken?);

        config.num_failures_threshold = 1;
        let membership = PeerToPeerClusterProvider::new(storage.clone(), config.clone());

        let is_broken = membership
            .is_broken(&Member::new("0.0.0.0".to_string(), "5000".to_string()))
            .await;
        assert!(is_broken?);

        config.interval_secs_threshold = 0;
        let membership = PeerToPeerClusterProvider::new(storage.clone(), config.clone());

        let is_broken = membership
            .is_broken(&Member::new("0.0.0.0".to_string(), "5000".to_string()))
            .await;
        assert!(!is_broken?);
        Ok(())
    }

    #[tokio::test]
    async fn get_members_to_monitor_as_configured() -> TestResult {
        let storage = storage().await;
        let items = storage.members().await?;

        let mut config = PeerToPeerClusterConfig::default();
        config.limit_monitored_members = Some(3);
        let membership = PeerToPeerClusterProvider::new(storage, config);

        // Maps monitored addresses, grouped by server (tester)
        let mut monitored_counter: HashMap<String, usize> = HashMap::new();
        for i in items.iter() {
            let members = membership.get_members_to_monitor(&i.address(), &items);
            monitored_counter.insert(i.address().clone(), members.len());
        }

        assert_eq!(monitored_counter.len(), 6);
        for monitored in monitored_counter.values() {
            // The same value as in `limit_monitored_members`
            assert_eq!(monitored, &3);
        }
        Ok(())
    }

    #[tokio::test]
    async fn get_members_to_monitor_few_members() -> TestResult {
        let storage = LocalStorage::default();
        storage
            .push(Member::new("0.0.0.0".to_string(), "5000".to_string()))
            .await?;
        let items = storage.members().await?;
        let membership =
            PeerToPeerClusterProvider::new(storage, PeerToPeerClusterConfig::default());

        let members = membership.get_members_to_monitor("0.0.0.0:5000", &items);
        assert_eq!(members.len(), 0, "{:?}", members);
        Ok(())
    }
}
