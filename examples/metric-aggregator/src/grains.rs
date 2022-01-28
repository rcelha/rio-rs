use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

use super::*;
use async_trait::async_trait;
use rio_rs::{membership_provider::sql::SqlMembersStorage, prelude::*};

// AppData
pub struct Counter(pub AtomicUsize);

#[derive(TypeName, FromId, Debug, Default)]
pub struct MetricAggregator {
    pub id: String,
    pub sum: i32,
    pub count: i32,
    pub max: i32,
    pub min: i32,
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

impl Grain for MetricAggregator {}

#[async_trait]
impl Handler<messages::Metric> for MetricAggregator {
    type Returns = messages::MetricResponse;
    async fn handle(
        &mut self,
        message: messages::Metric,
        app_data: Arc<AppData>,
    ) -> Result<Self::Returns, HandlerError> {
        {
            let counter = app_data.get::<Counter>();
            let value = counter.0.fetch_add(1, Ordering::SeqCst);
            println!("request-count={}", value);
        };

        self.propagate_to_tags(&app_data, &message.tags, message.value)
            .await;

        self.count += 1;
        self.sum += message.value;
        self.min = i32::min(self.min, message.value);
        self.max = i32::max(self.max, message.value);
        Ok(messages::MetricResponse {
            sum: self.sum,
            avg: self.sum / self.count,
            max: self.max,
            min: self.min,
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
            sum: self.sum,
            avg: if self.count == 0 {
                0
            } else {
                self.sum / self.count
            },
            max: self.max,
            min: self.min,
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
