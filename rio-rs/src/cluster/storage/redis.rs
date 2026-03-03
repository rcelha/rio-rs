use std::fmt::Display;

use async_trait::async_trait;
use bb8::Builder;
use bb8_redis::{bb8::Pool, RedisConnectionManager};
use chrono::{DateTime, Utc};
use redis::{AsyncCommands, RedisError};

use crate::errors::MembershipError;

use super::{Member, MembershipResult, MembershipStorage, MembershipUnitResult};

#[derive(Clone, Debug)]
pub struct RedisMembershipStorage {
    pool: Pool<RedisConnectionManager>,
    key_prefix: String,
}

impl RedisMembershipStorage {
    fn members_key(&self) -> String {
        format!("{}members", self.key_prefix)
    }

    fn member_failures_key(&self, ip: impl Display, port: impl Display) -> String {
        format!("{}member_failures;{};{}", self.key_prefix, ip, port)
    }
}

impl RedisMembershipStorage {
    pub fn new(pool: Pool<RedisConnectionManager>, key_prefix: Option<String>) -> Self {
        let key_prefix = key_prefix.unwrap_or_default();
        Self { pool, key_prefix }
    }

    pub fn pool() -> Builder<RedisConnectionManager> {
        Pool::builder()
    }

    pub fn connection_manager(url: impl ToString) -> Result<RedisConnectionManager, RedisError> {
        RedisConnectionManager::new(url.to_string())
    }
}

fn member_key(ip: impl Display, port: impl Display) -> String {
    format!("{}:{}", ip, port)
}

fn member_to_string(member: &Member) -> String {
    let member = format!(
        "{};{};{};{}",
        member.ip,
        member.port,
        member.active(),
        member.last_seen().to_rfc3339()
    );
    member
}

fn parse_member(member: &str) -> MembershipResult<Member> {
    let mut split_member = member.split(";");
    let ip = split_member
        .next()
        .ok_or(MembershipError::DeserializationError)?
        .to_string();
    let port = split_member
        .next()
        .ok_or(MembershipError::DeserializationError)?
        .to_string();
    let mut parsed_member = Member::new(ip, port);
    parsed_member.active = split_member
        .next()
        .ok_or(MembershipError::DeserializationError)?
        .parse()
        .map_err(|_| MembershipError::DeserializationError)?;
    let last_seen = split_member
        .next()
        .ok_or(MembershipError::DeserializationError)?;
    parsed_member.last_seen = DateTime::parse_from_rfc3339(last_seen)
        .map_err(|_| MembershipError::DeserializationError)?
        .to_utc();
    Ok(parsed_member)
}

#[async_trait]
impl MembershipStorage for RedisMembershipStorage {
    async fn push(&self, member: Member) -> MembershipUnitResult {
        let mut client = self.pool.get().await?;
        let member_key = member_key(&member.ip, &member.port);
        let member_val = member_to_string(&member);

        let key = self.members_key();
        let _: () = client.hset(&key, member_key, member_val).await?;
        Ok(())
    }

    async fn remove(&self, ip: &str, port: &str) -> MembershipUnitResult {
        let mut client = self.pool.get().await?;
        let member_key = member_key(ip, port);
        let key = self.members_key();
        let _: () = client.hdel(&key, member_key).await?;
        Ok(())
    }

    async fn set_is_active(&self, ip: &str, port: &str, is_active: bool) -> MembershipUnitResult {
        let last_seen = Utc::now();
        let mut client = self.pool.get().await?;
        let member_key = member_key(ip, port);
        let key = self.members_key();
        let raw_member: Option<String> = client.hget(&key, &member_key).await?;
        let mut member = raw_member
            .map(|x| parse_member(&x))
            .transpose()?
            .unwrap_or_else(|| Member::new(ip.to_string(), port.to_string()));
        if is_active {
            member.last_seen = last_seen;
        }
        member.active = is_active;
        self.push(member).await?;
        Ok(())
    }

    async fn members(&self) -> MembershipResult<Vec<Member>> {
        let mut client = self.pool.get().await?;
        let key = self.members_key();
        let members_raw: Vec<(String, String)> = client.hgetall(&key).await?;
        let members: Vec<Member> = members_raw
            .iter()
            .map(|(_, x)| parse_member(x))
            .collect::<MembershipResult<Vec<Member>>>()?;
        Ok(members)
    }

    async fn notify_failure(&self, ip: &str, port: &str) -> MembershipUnitResult {
        let mut client = self.pool.get().await?;
        let key = self.member_failures_key(ip, port);
        let now = chrono::Local::now().to_utc();
        let ts = now.timestamp();
        let _: () = client.rpush(&key, ts).await?;
        let _: () = client.ltrim(&key, 0, 1_000).await?;
        Ok(())
    }

    async fn member_failures(&self, ip: &str, port: &str) -> MembershipResult<Vec<DateTime<Utc>>> {
        let mut client = self.pool.get().await?;
        let key = self.member_failures_key(ip, port);
        let values: Vec<String> = client.lrange(&key, 0, -1).await?;
        let parsed_values = values
            .iter()
            .map(|x| {
                let ts: i64 = x
                    .parse()
                    .map_err(|_| MembershipError::DeserializationError)?;
                DateTime::from_timestamp(ts, 0)
                    .map(|dt| dt.to_utc())
                    .ok_or(MembershipError::DeserializationError)
            })
            .collect::<MembershipResult<Vec<_>>>()?;
        Ok(parsed_values)
    }
}
