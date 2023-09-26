# {{crate}}

{{readme}}

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
- [x] Remove need to use `add_static_fn(FromId::from_id)`
  - Removed in favour of `Registry::add_type`
- [x] Support service background task
- [ ] Pub/sub
- [-] Examples covering most use cases
  - [ ] Background async task on a service
  - [x] Background blocking task on a service (_see_ [examples/black-jack]())
  - [ ] Pub/sub
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
- [ ] Client/server keep alive
- [ ] Reduce static lifetimes
- [ ] Increase public API test coverage
- [ ] 100% documentation of public API
- [ ] Placement strategies (nodes work with different sets of trait objects)
- [ ] Dockerized examples
- [ ] Add pgsql jsonb support
- [ ] Add all SQL storage behind a feature flag (sqlite, mysql, pgsql, etc)
- [ ] Supervision
- [ ] Ephemeral objects (aka regular actors)
- [ ] Code of conduct
- [ ] Remove magic numbers
- [ ] Object TTL
- [ ] Matrix test with different backends
- [?] Support 'typed' message/response on client
- [ ] Support ephemeral port
- [ ] Remove the need for an `Option<T>` value for `managed_state` attributes (as long as it has a 'Default')
