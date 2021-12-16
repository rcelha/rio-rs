use rio_rs::{IdentifiableType, Message};
use serde::{Deserialize, Serialize};

#[derive(Default, Debug, Deserialize, Serialize)]
pub struct Metric {
    pub tags: String,
    pub value: i32,
}
impl Message for Metric {}
impl IdentifiableType for Metric {
    fn user_defined_type_id() -> &'static str {
        "Metric"
    }
}

#[derive(Default, Debug, Deserialize, Serialize)]
pub struct MetricResponse {
    pub sum: i32,
    pub avg: i32,
    pub max: i32,
    pub min: i32,
}
