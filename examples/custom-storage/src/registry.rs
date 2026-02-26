use rio_rs::prelude::*;

use crate::messages;
use crate::services;

type Noop = ();

make_registry! {
    services::Room: [
        LifecycleMessage => (Noop, ServiceObjectLifeCycleError),
        messages::Ping => (messages::Pong, NoopError),
    ]
}
