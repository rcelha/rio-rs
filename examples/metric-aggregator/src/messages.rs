use rio_rs::prelude::*;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(TypeName, Message, Debug, Deserialize, Serialize)]
pub struct Metric {
    pub tags: String,
    pub value: i32,
}

#[derive(TypeName, Message, Debug, Deserialize, Serialize)]
pub struct GetMetric {}

#[derive(TypeName, Message, Debug, Deserialize, Serialize)]
pub struct Drop {}

#[derive(TypeName, Message, Debug, Deserialize, Serialize)]
pub struct Ping {
    pub ping_id: String,
}

#[derive(TypeName, Message, Debug, Deserialize, Serialize)]
pub struct Pong {
    pub ping_id: String,
}

#[derive(TypeName, Message, Debug, Deserialize, Serialize)]
pub struct MetricResponse {
    pub sum: i32,
    pub avg: i32,
    pub max: i32,
    pub min: i32,
}

#[derive(Debug, Serialize, Deserialize, Error, Clone)]
pub enum MetricError {
    #[error("Error saving metrics")]
    SaveError,
}
