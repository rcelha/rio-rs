# {{crate}}

![GitHub Actions Workflow Status](https://img.shields.io/github/actions/workflow/status/rcelha/rio-rs/.github%2Fworkflows%2Frust.yaml?style=for-the-badge)
![Crates.io Version](https://img.shields.io/crates/v/rio-rs?style=for-the-badge&link=https%3A%2F%2Fcrates.io%2Fcrates%2Frio-rs)
![docs.rs](https://img.shields.io/docsrs/rio-rs?style=for-the-badge&link=https%3A%2F%2Fdocs.rs%2Frio-rs%2Flatest%2Frio_rs%2F)

---

{{readme}}

---

## Roadmap

There are a few things that must be done before v0.1.0:

### Next

- [x] Do some renaming around:
  - rename MembersStorage to MembershipStorage (rio_rs::cluster::storage)
  - ObjectPlacement to ObjectPlacementItem (rio_rs::object_placement)
  - ObjectPlacementProvider to ObjectPlacement (rio_rs::object_placement)
- [ ] MDNs
- [ ] Client bindings for other languages
- [ ] Remove the need for two types of concurrent hashmap (papaya and dashmap)
- [ ] Guest languages support - Currently possible with WASM + tons of boiler-plate
- [ ] Create server from config
- [ ] Bypass clustering for self messages
  - _possibly a no-issue_
- [ ] Bypass networking for local messages
  - _partialy done?_
- [ ] Move all the client to user tower
- [ ] Remove the need to pass the StateSaver to `ObjectStateManager::save_state`
  - Might not be feasible, there are a few workarounds for testing that I might write some examples
    and call it a day
- [ ] Include registry configuration in Server builder
- [ ] Create a getting started tutorial
  - Cargo init
  - Add deps (rio-rs, tokio, async_trait, serde, sqlx - optional)
  - Write a server
  - Write a client
  - Add service and messages
  - Cargo run --bin server
  - Cargo run --bin client
  - Life cycle
  - Life cycle depends on app_data(StateLoader + StateSaver)
  - Cargo test?
- [ ] MySQL support for sql backends
- [ ] Add pgsql jsonb support
- [ ] Client/server keep alive
- [ ] Topology - nodes work with different sets of service types
- [ ] Placement strategies - sets where to place objects
- [ ] Supervision
- [ ] Ephemeral objects (aka regular - local - actors)
- [ ] Remove magic numbers
- [ ] Object TTL
- [ ] Code of conduct
- [ ] Metrics
- [ ] Tracing
- [ ] Deny allocations based on system resources
- [ ] Dockerized examples
- [ ] Reduce static lifetimes
  - Might not be feasible


### Version 0.2.3

- [x] Improve error message for ManagedState macro when the struct doesn't implement ServiceObject
- [x] Improve error message for ManagedState when the storage is not in the context
- [x] Improve error message for when the services are not added to the registry (server)
- [x] Client doesn't need to have a access to the cluster backend if we implement an HTTP API

### Version 0.2.0

- [x] Support 'typed' message/response on client (TODO define what this means)
- [x] Ability to hook up own custom errors on message handlers
- [x] Allow `ServiceObject` trait without state persistence
- [x] Add more extensive tests to client/server integration
- [x] Increase public API test coverage
- [x] Add all SQL storage behind a feature flag (sqlite, mysql, pgsql, etc)
- [x] Matrix test with different backends
- [x] Replace prints with logging

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
- [x] `ServiceObject::send` shouldn't need a type for the member storage
- [x] Handle panics on messages handling
- [x] Error and panic handling on life cycle hooks (probably kill the object)
- [x] Create a test or example to show re-allocation when servers dies
- [x] Sqlite support for sql backends
- [x] PostgreSQL support for sql backends
- [x] Redis support for members storage
- [x] Redis support for state backend (loader and saver)
- [x] Redis support for object placement
