mod builder;
mod client;
mod pool;

pub const DEFAULT_TIMEOUT_MILLIS: u64 = 500;

pub use builder::ClientBuilder;
pub use client::Client;
pub use pool::ClientConnectionManager;
