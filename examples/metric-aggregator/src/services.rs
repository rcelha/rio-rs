use async_trait::async_trait;
use rio_rs::cluster::storage::sql::SqlMembersStorage;
use rio_rs::prelude::*;
use rio_rs::state::sql::SqlState;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use super::*;

// AppData
pub struct Counter(pub AtomicUsize);

#[derive(Default, Debug, Serialize, Deserialize, TypeName)]
pub struct MetricStats {
    pub sum: i32,
    pub count: i32,
    pub max: i32,
    pub min: i32,
}

#[derive(Debug, Default, TypeName, FromId, ManagedState)]
pub struct MetricAggregator {
    pub id: String,
    #[managed_state(provider = SqlState)]
    pub metric_stats: Option<MetricStats>,
}

impl MetricAggregator {
    async fn propagate_to_tags(&self, app_data: &Arc<AppData>, tags: &str, value: i32) {
        let _: Vec<messages::MetricResponse> =
            futures::future::join_all(tags.split(",").filter(|x| !x.trim().is_empty()).map(
                |i| async {
                    let sub_message = messages::Metric {
                        tags: "".to_string(),
                        value,
                    };
                    Self::send::<SqlMembersStorage, _, _>(
                        &app_data,
                        "MetricAggregator".to_string(),
                        i.to_string(),
                        &sub_message,
                    )
                    .await
                    .expect("send fail")
                },
            ))
            .await;
    }
}

#[async_trait]
impl ServiceObject for MetricAggregator {
    async fn after_load(&mut self, _: &AppData) -> Result<(), ServiceObjectLifeCycleError> {
        if self.metric_stats.is_none() {
            self.metric_stats = Some(MetricStats::default())
        }
        Ok(())
    }
}

#[async_trait]
impl Handler<messages::Metric> for MetricAggregator {
    type Returns = messages::MetricResponse;
    async fn handle(
        &mut self,
        message: messages::Metric,
        app_data: Arc<AppData>,
    ) -> Result<Self::Returns, HandlerError> {
        let state_saver = app_data.get::<SqlState>();
        {
            let counter = app_data.get::<Counter>();
            let value = counter.0.fetch_add(1, Ordering::SeqCst);
            println!("request-count={}", value);
        };

        self.propagate_to_tags(&app_data, &message.tags, message.value)
            .await;

        match self.metric_stats.as_mut() {
            Some(mut stats) => {
                stats.count += 1;
                stats.sum += message.value;
                stats.min = i32::min(stats.min, message.value);
                stats.max = i32::max(stats.max, message.value);
            }
            None => {
                println!("no stats found");
                return Err(HandlerError::LyfecycleError(
                    rio_rs::errors::ServiceObjectLifeCycleError::Unknown,
                ));
            }
        }

        self.save_state(state_saver).await.map_err(|_| {
            println!("save error");
            HandlerError::LyfecycleError(rio_rs::errors::ServiceObjectLifeCycleError::Unknown)
        })?;

        self.metric_stats
            .as_ref()
            .map(|stats| {
                Ok(messages::MetricResponse {
                    sum: stats.sum,
                    avg: stats.sum / stats.count,
                    max: stats.max,
                    min: stats.min,
                })
            })
            .unwrap()
    }
}

#[async_trait]
impl Handler<messages::GetMetric> for MetricAggregator {
    type Returns = messages::MetricResponse;
    async fn handle(
        &mut self,
        _: messages::GetMetric,
        _: Arc<AppData>,
    ) -> Result<Self::Returns, HandlerError> {
        let stats = self.metric_stats.as_ref().ok_or(HandlerError::Unknown)?;
        Ok(messages::MetricResponse {
            sum: stats.sum,
            avg: if stats.count == 0 {
                0
            } else {
                stats.sum / stats.count
            },
            max: stats.max,
            min: stats.min,
        })
    }
}

#[async_trait]
impl Handler<messages::Ping> for MetricAggregator {
    type Returns = messages::Pong;
    async fn handle(
        &mut self,
        message: messages::Ping,
        _: Arc<AppData>,
    ) -> Result<Self::Returns, HandlerError> {
        Ok(messages::Pong {
            ping_id: message.ping_id,
        })
    }
}

#[async_trait]
impl Handler<messages::Drop> for MetricAggregator {
    type Returns = ();
    async fn handle(
        &mut self,
        _: messages::Drop,
        app_data: Arc<AppData>,
    ) -> Result<Self::Returns, HandlerError> {
        println!("got shudown");
        self.shutdown(&app_data).await.expect("TODO shutdown");
        Ok(())
    }
}
