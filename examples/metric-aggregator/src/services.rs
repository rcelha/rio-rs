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

#[derive(Debug, Default, TypeName, WithId, ManagedState)]
pub struct MetricAggregator {
    pub id: String,
    #[managed_state(provider = SqlState)]
    pub metric_stats: MetricStats,
}

impl MetricAggregator {
    async fn propagate_to_tags(&self, app_data: &Arc<AppData>, tags: &str, value: i32) {
        let _: Vec<messages::MetricResponse> =
            futures::future::join_all(tags.split(",").filter(|x| !x.trim().is_empty()).map(
                |i| async move {
                    let sub_message = messages::Metric {
                        tags: "".to_string(),
                        value,
                    };
                    Self::send::<SqlMembersStorage, _, _>(
                        &app_data,
                        &"MetricAggregator",
                        &i,
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
    async fn after_load(&mut self, _: Arc<AppData>) -> Result<(), ServiceObjectLifeCycleError> {
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

        self.metric_stats.count += 1;
        self.metric_stats.sum += message.value;
        self.metric_stats.min = i32::min(self.metric_stats.min, message.value);
        self.metric_stats.max = i32::max(self.metric_stats.max, message.value);

        self.save_state(state_saver).await.map_err(|_| {
            println!("save error");
            HandlerError::LyfecycleError(rio_rs::errors::ServiceObjectLifeCycleError::Unknown)
        })?;

        Ok(messages::MetricResponse {
            sum: self.metric_stats.sum,
            avg: self.metric_stats.sum / self.metric_stats.count,
            max: self.metric_stats.max,
            min: self.metric_stats.min,
        })
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
        Ok(messages::MetricResponse {
            sum: self.metric_stats.sum,
            avg: if self.metric_stats.count == 0 {
                0
            } else {
                self.metric_stats.sum / self.metric_stats.count
            },
            max: self.metric_stats.max,
            min: self.metric_stats.min,
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
        self.shutdown(app_data).await.expect("TODO shutdown");
        Ok(())
    }
}
