//! Maps objects and their ids to different broadcast channels
//!
//! <div class="warning">
//!
//! # TODO
//! - [ ] This component might be temporary. It serves as a router between different publishers and subscribers
//! - [ ] I need a way to remove unused channels (use LRU)
//! - [ ] Configure channel limits
//!
//! </div>

use dashmap::DashMap;
use tokio::sync::broadcast;

use crate::protocol::pubsub::SubscriptionResponse;

type InboxHashMap = DashMap<(String, String), broadcast::Sender<SubscriptionResponse>>;

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
        let sender = self.inboxes.entry((k1, k2)).or_insert_with(|| {
            let (sender, _) = broadcast::channel(1_000);
            sender
        });
        sender.subscribe()
    }

    pub fn publish(&self, k1: String, k2: String, message: SubscriptionResponse) {
        let maybe_sender = self.inboxes.get_mut(&(k1, k2));
        if let Some(sender) = maybe_sender {
            sender.send(message).ok();
        }
    }
}
