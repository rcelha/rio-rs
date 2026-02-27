use rio_rs::prelude::*;

use crate::messages;
use crate::services;

type Noop = ();

make_registry! {
    services::PresenceService: [
        LifecycleMessage => (Noop, ServiceObjectLifeCycleError),
        messages::Ping => (Noop, NoopError),
    ]
}
