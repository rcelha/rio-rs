use rio_rs::prelude::*;

use crate::messages;
use crate::services;

type Noop = ();

make_registry! {
    services::MetricAggregator: [
        LifecycleMessage => (Noop, ServiceObjectLifeCycleError),
        messages::Ping => (messages::Pong, messages::MetricError),
        messages::Metric => (messages::MetricResponse, messages::MetricError),
        messages::GetMetric => (messages::MetricResponse, messages::MetricError),
        messages::Drop => (Noop, NoopError),
    ]
}
