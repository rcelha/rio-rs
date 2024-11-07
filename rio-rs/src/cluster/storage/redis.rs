use async_trait::async_trait;
use bb8_redis::{bb8::Pool, RedisConnectionManager};
use chrono::{DateTime, Utc};
use redis::AsyncCommands;

use super::{Member, MembersStorage, MembershipResult, MembershipUnitResult};

#[derive(Clone)]
pub struct RedisMembersStorage {
    pool: Pool<RedisConnectionManager>,
    key_prefix: String,
}

impl RedisMembersStorage {
    pub async fn from_connect_string(connection_string: &str, key_prefix: Option<String>) -> Self {
        let conn_manager = RedisConnectionManager::new(connection_string).expect("TODO");
        let pool = Pool::builder().build(conn_manager).await.expect("TODO");
        let key_prefix = key_prefix.unwrap_or_default();
        RedisMembersStorage { pool, key_prefix }
    }
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

fn parse_member(member: &str) -> Member {
    let mut split_member = member.split(";");
    let ip = split_member.next().expect("TODO").to_string();
    let port = split_member.next().expect("TODO").to_string();
    let mut parsed_member = Member::new(ip, port);
    parsed_member.active = split_member
        .next()
        .expect("TODO next")
        .parse()
        .expect("TODO parse");
    let last_seen = split_member.next().expect("TODO");
    parsed_member.last_seen = DateTime::parse_from_rfc3339(&last_seen)
        .expect("TODO")
        .to_utc();
    parsed_member
}

#[async_trait]
impl MembersStorage for RedisMembersStorage {
    async fn push(&self, member: Member) -> MembershipUnitResult {
        let mut client = self.pool.get().await.expect("TODO");
        let member_id = format!("{}:{}", member.ip, member.port,);
        let member_val = member_to_string(&member);

        let key = format!("{}members", self.key_prefix);
        let _: () = client
            .hset(&key, member_id, member_val)
            .await
            .expect("TODO");
        Ok(())
    }

    async fn remove(&self, ip: &str, port: &str) -> MembershipUnitResult {
        let mut client = self.pool.get().await.expect("TODO");
        let member_id = format!("{};{}", ip, port);
        let key = format!("{}members", self.key_prefix);
        let _: () = client.hdel(&key, member_id).await.expect("TODO");
        Ok(())
    }

    async fn set_is_active(&self, ip: &str, port: &str, is_active: bool) -> MembershipUnitResult {
        let mut client = self.pool.get().await.expect("TODO");
        let member_id = format!("{};{}", ip, port);
        let key = format!("{}members", self.key_prefix);
        let raw_member: Option<String> = client.hget(&key, &member_id).await.expect("TODO");
        let mut member = raw_member
            .map(|x| parse_member(&x))
            .unwrap_or_else(|| Member::new(ip.to_string(), port.to_string()));
        member.active = is_active;
        self.push(member).await?;
        Ok(())
    }

    async fn members(&self) -> MembershipResult<Vec<Member>> {
        let mut client = self.pool.get().await.expect("TODO");
        let key = format!("{}members", self.key_prefix);
        let members_raw: Vec<(String, String)> = client.hgetall(&key).await.expect("TODO");
        let members: Vec<Member> = members_raw.iter().map(|(_, x)| parse_member(x)).collect();
        Ok(members)
    }

    async fn notify_failure(&self, ip: &str, port: &str) -> MembershipUnitResult {
        let mut client = self.pool.get().await.expect("TODO");
        let k = format!("{}member_failures;{};{}", self.key_prefix, ip, port);
        let now = chrono::Local::now().to_utc();
        let ts = now.timestamp();
        let _: () = client.rpush(&k, ts).await.expect("TODO");
        let _: () = client.ltrim(&k, 0, 99).await.expect("TODO");
        Ok(())
    }

    async fn member_failures(&self, ip: &str, port: &str) -> MembershipResult<Vec<DateTime<Utc>>> {
        let mut client = self.pool.get().await.expect("TODO");
        let k = format!("{}member_failures;{};{}", self.key_prefix, ip, port);
        let values: Vec<String> = client.lrange(&k, 0, -1).await.expect("TODO");
        let parsed_values = values
            .iter()
            .map(|x| {
                let ts: i64 = x.parse().expect("TODO");
                DateTime::from_timestamp(ts, 0).expect("TODO").to_utc()
            })
            .collect();
        Ok(parsed_values)
    }
}
