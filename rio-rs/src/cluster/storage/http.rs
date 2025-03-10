//! Partial implementation of the cluster storage to be exposed
//! as a HTTP service
//!
//! This is useful when you want to allow clients to connect directly
//! to your cluster

use async_trait::async_trait;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing, Json, Router,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::net::ToSocketAddrs;

use crate::errors::MembershipError;

use super::{Member, MembersStorage, MembershipResult, MembershipUnitResult};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct HttpMember {
    ip: String,
    port: String,
    active: bool,
    last_seen: String,
}

#[derive(Clone)]
struct AppData<S: MembersStorage + 'static> {
    pub inner: S,
}

pub async fn serve(
    bind: impl ToSocketAddrs,
    backend: impl MembersStorage + 'static,
) -> MembershipUnitResult {
    // build our application with a route
    let app = Router::new()
        .route("/members", routing::get(list_members))
        .route("/members/:ip/:port/", routing::get(member_failures))
        .with_state(AppData { inner: backend });

    // run our app with hyper, listening globally on port 3000
    let listener = tokio::net::TcpListener::bind(bind).await.unwrap();
    axum::serve(listener, app).await.unwrap();
    let _ = bind;
    Ok(())
}

async fn list_members<S: MembersStorage + 'static>(
    State(app_data): State<AppData<S>>,
) -> (StatusCode, Json<Vec<HttpMember>>) {
    let members: Vec<_> = app_data
        .inner
        .members()
        .await
        .expect("TODO")
        .iter()
        .map(|x| HttpMember {
            active: x.active,
            ip: x.ip.clone(),
            last_seen: x.last_seen.to_string(),
            port: x.port.clone(),
        })
        .collect();

    (StatusCode::OK, Json(members))
}

async fn member_failures<S: MembersStorage + 'static>(
    Path((ip, port)): Path<(String, String)>,
    State(app_data): State<AppData<S>>,
) -> (StatusCode, Json<Vec<String>>) {
    let data = app_data.inner.member_failures(&ip, &port).await;
    let data = match data {
        Ok(x) => x,
        Err(_err) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(vec![])),
    };
    let ret = data.iter().map(|i| i.to_string()).collect();
    (StatusCode::OK, Json(ret))
}

#[derive(Clone, Default)]
pub struct HttpMembersStorage {
    pub remote_address: String,
}

#[async_trait]
impl MembersStorage for HttpMembersStorage {
    async fn push(&self, _member: Member) -> MembershipUnitResult {
        Err(MembershipError::ReadOnly("push".to_string()))
    }

    async fn remove(&self, _ip: &str, _port: &str) -> MembershipUnitResult {
        Err(MembershipError::ReadOnly("remove".to_string()))
    }

    async fn set_is_active(
        &self,
        _ip: &str,
        _port: &str,
        _is_active: bool,
    ) -> MembershipUnitResult {
        Err(MembershipError::ReadOnly("set_is_active".to_string()))
    }

    async fn notify_failure(&self, _ip: &str, _port: &str) -> MembershipUnitResult {
        Err(MembershipError::ReadOnly("notify_failure".to_string()))
    }

    async fn member_failures(&self, ip: &str, port: &str) -> MembershipResult<Vec<DateTime<Utc>>> {
        let url = format!("{}/members/{}/{}", self.remote_address, ip, port);
        let resp = reqwest::get(url)
            .await
            .map_err(|err| MembershipError::Upstream(err.to_string()))?;
        let str_failures: Vec<String> = resp
            .json()
            .await
            .map_err(|err| MembershipError::Unknown(err.to_string()))?;

        let datetime_failures: Result<Vec<DateTime<Utc>>, _> =
            str_failures.into_iter().map(|x| x.parse()).collect();

        datetime_failures.map_err(|err| MembershipError::Unknown(err.to_string()))
    }

    async fn members(&self) -> MembershipResult<Vec<Member>> {
        let url = format!("{}/members", self.remote_address);
        let resp = reqwest::get(url)
            .await
            .map_err(|err| MembershipError::Upstream(err.to_string()))?;
        let http_members: Vec<HttpMember> = resp
            .json()
            .await
            .map_err(|err| MembershipError::Unknown(err.to_string()))?;

        let members: Vec<Member> = http_members
            .iter()
            .map(|x| Member {
                active: x.active,
                ip: x.ip.clone(),
                last_seen: x.last_seen.parse().unwrap(),
                port: x.port.clone(),
            })
            .collect();
        Ok(members)
    }
}
