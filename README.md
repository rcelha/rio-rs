# rio-rs

![GitHub Actions Workflow Status](https://img.shields.io/github/actions/workflow/status/rcelha/rio-rs/.github%2Fworkflows%2Frust.yaml?style=for-the-badge)
![Crates.io Version](https://img.shields.io/crates/v/rio-rs?style=for-the-badge&link=https%3A%2F%2Fcrates.io%2Fcrates%2Frio-rs)
![docs.rs](https://img.shields.io/docsrs/rio-rs?style=for-the-badge&link=https%3A%2F%2Fdocs.rs%2Frio-rs%2Flatest%2Frio_rs%2F)

---

Distributed stateful services inspired by Orleans

This crate provides a framework for scalable, distributed and stateful services
based on message passing between objects

## Application

Most of your application code will be written in forms of `ServiceObjects` and `Messages`

```rust
use async_trait::async_trait;
use rio_rs::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(TypeName, Message, Deserialize, Serialize)]
pub struct HelloMessage {
    pub name: String
}

#[derive(TypeName, Message, Deserialize, Serialize)]
pub struct HelloResponse {}

#[derive(TypeName, WithId, Default)]
pub struct HelloWorldService {
    pub id: String,
}

#[async_trait]
impl Handler<HelloMessage> for HelloWorldService {
    type Returns = HelloResponse;
    type Error = NoopError;
    async fn handle(
        &mut self,
        message: HelloMessage,
        app_data: Arc<AppData>,
    ) -> Result<Self::Returns, Self::Error> {
        println!("Hello world");
        Ok(HelloResponse {})
    }
}

```

## Running Server

To run your application you need to spin up your servers, the `Server`

TODO: Include example of other databases

```rust
use rio_rs::prelude::*;
use rio_rs::cluster::storage::sqlite::SqliteMembersStorage;
use rio_rs::object_placement::sqlite::SqliteObjectPlacementProvider;


#[tokio::main]
async fn main() {
    let addr = "0.0.0.0:0";

    // Configure types on the server's registry
    let mut registry = Registry::new();
    registry.add_type::<HelloWorldService>();
    registry.add_handler::<HelloWorldService, HelloMessage>();

    // Configure the Cluster Membership provider
    let pool = SqliteMembersStorage::pool()
        .connect("sqlite::memory:")
        .await
        .expect("Membership database connection failure");
    let members_storage = SqliteMembersStorage::new(pool);

    let membership_provider_config = PeerToPeerClusterConfig::default();
    let membership_provider =
        PeerToPeerClusterProvider::new(members_storage, membership_provider_config);

    // Configure the object placement
    let pool = SqliteMembersStorage::pool()
        .connect("sqlite::memory:")
        .await
        .expect("Object placement database connection failure");
    let object_placement_provider = SqliteObjectPlacementProvider::new(pool);

    // Create the server object
    let mut server = Server::new(
        addr.to_string(),
        registry,
        membership_provider,
        object_placement_provider,
    );
    server.prepare().await;
    let listener = server.bind().await.expect("Bind");
    // Run the server
    // server.run(listener).await;
}
```

## Client

Communicating with the cluster is just a matter of sending the serialized known messages via TCP.
The [`client`] module provides an easy way of achieving this:

```rust
use rio_rs::prelude::*;
use rio_rs::cluster::storage::sqlite::SqliteMembersStorage;


#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Member storage configuration (Rendezvous)
    let pool = SqliteMembersStorage::pool()
        .connect("sqlite::memory:")
        .await?;
    let members_storage = SqliteMembersStorage::new(pool);
    # members_storage.prepare().await;

    // Create the client
    let mut client = ClientBuilder::new()
        .members_storage(members_storage)
        .build()?;

    let payload = HelloMessage { name: "Client".to_string() };
    let response: HelloResponse = client
        .send::<HelloResponse, NoopError>(
            "HelloWorldService".to_string(),
            "any-string-id".to_string(),
            &payload,
        ).await?;

    // response is a `HelloResponse {}`
    Ok(())
}
```

---

## Roadmap

There are a few things that must be done before v0.1.0:

### Next

- [x] Improve error message for ManagedState macro when the struct doesn't implement ServiceObject
- [x] Improve error message for ManagedState when the storage is not in the context
- [x] Improve error message for when the services are not added to the registry (server)
- [ ] Python client bindings
- [ ] Guest languages support
- [ ] Remove the need for two types of concurrent hashmap (papaya and dashmap)
- [x] Client doesn't need to have a access to the cluster backend if we implement an HTTP API
- [ ] Create server from config
- [ ] Bypass clustering for self messages
- [~] Bypass networking for local messages
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
- [ ] Add pgsql jsonb support
- [ ] Client/server keep alive
- [ ] Placement strategies (nodes work with different sets of trait objects)
- [ ] Supervision
- [ ] Ephemeral objects (aka regular - local - actors)
- [ ] Remove magic numbers
- [ ] Object TTL
- [ ] Code of conduct
- [ ] Metrics and Tracing
- [ ] Deny allocations based on system resources
- [ ] Dockerized examples
- [?] Reduce static lifetimes



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
