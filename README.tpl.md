# {{crate}}

{{readme}}

---

How it is:

```rust
#[derive(Debug, Default, TypeName, WithId, ManagedState)]
pub struct MetricAggregator {
    pub id: String,
    #[managed_state(provider = SqlState)]
    pub metric_stats: Option<MetricStats>,
}


#[async_trait]
impl Handler<messages::Metric> for MetricAggregator {
    type Returns = messages::MetricResponse;
    async fn handle(
        &mut self,
        message: messages::Metric,
        app_data: Arc<AppData>,
    ) -> Result<Self::Returns, HandlerError> {
        let state_saver = app_data.get::<SqlState>();
        self.save_state(state_saver).await?;
        Ok(messages::MetricResponse {
           sum: 1,
           avg: 1,
           max: 1,
           min: 1
        })
    }
}
```

How I want it to be:

```rust
#[derive(Debug, Default, TypeName, WithId, ManagedState)]
pub struct MetricAggregator {
    pub id: String,
    #[managed_state]
    pub metric_stats: MetricStats,
}


#[async_trait]
impl Handler<messages::Metric> for MetricAggregator {
    type Returns = messages::MetricResponse;
    async fn handle(
        &mut self,
        message: messages::Metric,
        app_data: Arc<AppData>,
    ) -> Result<Self::Returns, HandlerError> {
        self.save_state<MetricStats>().await?;
        Ok(messages::MetricResponse {
           sum: 1,
           avg: 1,
           max: 1,
           min: 1
        })
    }
}

// Server
/// ...
server.configure_state<MetricAggregator, MetricStats, SqlState>(SqlStateConfig::sqlite("sqlite:///tmp/test.sqlite3"));
server.run().await?;
/// ...
```

If you want to have more than one attribute persisted with the same type?

```rust
type OldMetricStats = MetricStats; // I don't know if this would work
// or
pub struct OldMetricStats(pub MetricStats);
impl Deref for OldMetricStats {
  type Target = MetricStats;
  fn deref(&self) -> &Self::Target {
    &self.0
  }
}
// or
custom_derive! {
    #[derive(NewtypeFrom, NewtypeDeref, NewtypeDerefMut)]
    pub struct OldMetricStats(MetricStats);
}

#[derive(Debug, Default, TypeName, WithId, ManagedState)]
pub struct MetricAggregator {
    pub id: String,
    #[managed_state]
    pub metric_stats: MetricStats,
    #[managed_state]
    pub old_metric_stats: OldMetricStats,
}


#[async_trait]
impl Handler<messages::Metric> for MetricAggregator {
    type Returns = messages::MetricResponse;
    async fn handle(
        &mut self,
        message: messages::Metric,
        app_data: Arc<AppData>,
    ) -> Result<Self::Returns, HandlerError> {
        self.save_state<MetricStats>().await?;
        self.save_state<OldMetricStats>().await?;
        Ok(messages::MetricResponse {
           sum: 1,
           avg: 1,
           max: 1,
           min: 1
        })
    }
}

// Server
/// ...
server.configure_state<MetricAggregator, MetricStats, SqlState>(SqlStateConfig::sqlite("sqlite:///tmp/test.sqlite3"));
server.configure_state<MetricAggregator, OldMetricStats, LocalState>(LocalStateConfig::default());
server.run().await?;
/// ...
`


---

## Roadmap

There are a few things that must be done before v0.1.0:

- [x] Naive server/client protocol
- [x] Basic cluster support
- [x] Basic placement support
- [x] Object self shutdown
- [x] Naive object persistence
- [x] Public API renaming
- [x] Reduce Boxed objects
- [x] Create a Server builder
- [x] Remove need to use `add_static_fn(FromId::from_id)` -> Removed in favour of `Registry::add_type`
- [x] Support service background task
- [x] Pub/sub
- [x] Examples covering most use cases
  - [x] Background async task on a service
  - [x] Background blocking task on a service (_see_ [black-jack](./examples/black-jack))
  - [x] Pub/sub (_see_ [black-jack](./examples/black-jack))
- [ ] Re-organize workspace
- [ ] Allow `ServiceObject` trait without state persistence
- [ ] Feature: Create server from config
- [ ] Bypass clustering for self messages
- [ ] Bypass networking for local messages
- [ ] Move all the client to user tower
- [ ] Remove the need to pass the StateSaver to `ObjectStateManager::save_state`
- [ ] Error and panic handling on life cycle hooks (probably kill the object)
- [ ] Handle panics on messages handling
- [ ] Include registry configuration in Server builder
- [ ] Create a getting started tutorial
  - [ ] Cargo init
  - [ ] Add deps (rio-rs, tokio, async_trait, serde, sqlx - optional)
  - [ ] Write a server
  - [ ] Write a client
  - [ ] Add service and messages
  - [ ] Cargo run --bin server
  - [ ] Cargo run --bin client
  - [ ] Life cycle
  - [ ] Life cycle depends on app_data(StateLoader + StateSaver)
  - [ ] Cargo test?
- [ ] Make all sql statements compatible with sqlite, mysql and pgsql
- [ ] Add more extensive tests to client/server integration
- [ ] Increase public API test coverage
- [ ] Client/server keep alive
- [ ] Reduce static lifetimes
- [ ] 100% documentation of public API
- [ ] Placement strategies (nodes work with different sets of trait objects)
- [ ] Dockerized examples
- [ ] Add pgsql jsonb support
- [ ] Add all SQL storage behind a feature flag (sqlite, mysql, pgsql, etc)
- [ ] Supervision
- [ ] Ephemeral objects (aka regular - local - actors)
- [ ] Remove magic numbers
- [ ] Object TTL
- [ ] Matrix test with different backends
- [ ] Replace prints with logging
- [?] Support 'typed' message/response on client
- [ ] Support ephemeral port
- [ ] Remove the need for an `Option<T>` value for `managed_state` attributes (as long as it has a 'Default')
- [ ] Code of conduct
```
