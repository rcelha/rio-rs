use super::*;
use async_trait::async_trait;
use rio_rs::{Handler, IdentifiableType};

#[derive(Debug, Default)]
pub struct MetricAggregator {
    pub sum: i32,
    pub count: i32,
    pub max: i32,
    pub min: i32,
}

impl IdentifiableType for MetricAggregator {
    fn user_defined_type_id() -> &'static str {
        "MetricAggregator"
    }
}

#[async_trait]
impl Handler<messages::Metric> for MetricAggregator {
    type Returns = messages::MetricResponse;
    async fn handle(
        &mut self,
        message: messages::Metric,
    ) -> Result<Self::Returns, rio_rs::HandlerError> {
        // TODO propagate to message.tags
        self.count += 1;
        self.sum += message.value;
        self.min = i32::min(self.min, message.value);
        self.max = i32::max(self.max, message.value);
        Ok(messages::MetricResponse {
            sum: self.sum,
            avg: 0,
            max: self.max,
            min: self.min,
        })
    }
}
