//! Maps objects and their ids to different broadcast channels
//!
//! <div class="warning">
//!
//! # TODO
//! - [ ] This component might be temporary. It serves as a router between
//!       different publishers and subscribers
//! - [ ] I need a way to remove unused channels (use LRU)
//! - [ ] Configure channel limits
//!
//! </div>
use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use tokio::sync::broadcast;

use crate::protocol::pubsub::SubscriptionResponse;

type InboxHashMap = Arc<RwLock<HashMap<(String, String), broadcast::Sender<SubscriptionResponse>>>>;

#[derive(Debug, Default, Clone)]
pub struct MessageRouter {
    inboxes: InboxHashMap,
}

impl MessageRouter {
    pub fn create_subscription(
        &self,
        k1: String,
        k2: String,
    ) -> broadcast::Receiver<SubscriptionResponse> {
        let mut inboxes_guard = self.inboxes.write().expect("Unrecoverable error");
        let sender = inboxes_guard.entry((k1, k2)).or_insert_with(|| {
            let (sender, _) = broadcast::channel(1_000);
            sender
        });
        sender.subscribe()
    }

    pub fn publish(&self, k1: String, k2: String, message: SubscriptionResponse) {
        let mut inboxes_guard = self.inboxes.write().expect("Unrecoverable error");
        let sender = inboxes_guard.entry((k1, k2)).or_insert_with(|| {
            let (sender, _) = broadcast::channel(1_000);
            sender
        });
        sender.send(message).ok();
    }
}
