# {{crate}}

{{readme}}

---

## Roadmap

There are a few things that must be done before v0.1.0:

### Version 0.1.0

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
- [x] Re-organize workspace
- [x] Support ephemeral port
- [x] Remove the need for an `Option<T>` value for `managed_state` attributes (as long as it has a 'Default')
- [ ] Support 'typed' message/response on client (TODO define what this means)
- [ ] `ServiceObject::send` shouldn't need a type for the member storage
- [x] Handle panics on messages handling
- [x] Error and panic handling on life cycle hooks (probably kill the object)
- [x] Create a test or example to show re-allocation when servers dies
- [x] Sqlite support for sql backends
- [x] PostgreSQL support for sql backends
- [x] Redis support for members storage
- [x] Redis support for state backend (loader and saver)
- [x] Redis support for object placement

### Version 0.2.0

- [ ] Remove the need for two types of concurrent hashmap (papaya and dashmap)
- [ ] Client doesn't need to have a access to the cluster backend if we implement an HTTP API
- [ ] Allow `ServiceObject` trait without state persistence
- [ ] Create server from config
- [ ] Bypass clustering for self messages
- [ ] Bypass networking for local messages
- [ ] Move all the client to user tower
- [ ] Remove the need to pass the StateSaver to `ObjectStateManager::save_state`
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
- [ ] MySQL support for sql backends
- [ ] Add more extensive tests to client/server integration
- [ ] Increase public API test coverage
- [ ] Client/server keep alive
- [ ] Reduce static lifetimes
- [ ] 100% documentation of public API
- [ ] Placement strategies (nodes work with different sets of trait objects)
- [~] Dockerized examples
- [ ] Add pgsql jsonb support
- [ ] Add all SQL storage behind a feature flag (sqlite, mysql, pgsql, etc)
- [ ] Supervision
- [ ] Ephemeral objects (aka regular - local - actors)
- [ ] Remove magic numbers
- [ ] Object TTL
- [ ] Matrix test with different backends
- [x] Replace prints with logging
- [ ] Code of conduct
- [ ] Metrics and Tracing
- [ ] Deny allocations based on system resources
