//! Trait object registry
//!
//! Provides storage for objects and maps their callables to handle registered message types

use crate::app_data::AppData;

use async_trait::async_trait;
use serde::{de::DeserializeOwned, Serialize};
use std::sync::Arc;

#[async_trait]
pub trait Handler<M>
where
    Self: Send + Sync,
    M: Message + Send + Sync,
{
    type Returns: Serialize + Sync + Send;
    type Error: Serialize;

    async fn handle(
        &mut self,
        message: M,
        context: Arc<AppData>,
    ) -> Result<Self::Returns, Self::Error>;
}

pub trait Message: Serialize + DeserializeOwned {}
