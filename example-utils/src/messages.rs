use rio_rs::Message;
use serde::{Deserialize, Serialize};

#[derive(Default, Debug, Deserialize, Serialize)]
pub struct Metric {
    pub tags: String,
    pub value: i32,
}
impl Message for Metric {}

#[derive(Default, Debug, Deserialize, Serialize)]
pub struct MetricResponse {
    pub sum: i32,
    pub avg: i32,
    pub max: i32,
    pub min: i32,
}
