# Rio

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

#[derive(TypeName, FromId, Default)]
pub struct HelloWorldService {
    pub id: String,
}

#[async_trait]
impl Handler<HelloMessage> for HelloWorldService {
    type Returns = HelloResponse;
    async fn handle(
        &mut self,
        message: HelloMessage,
        app_data: Arc<AppData>,
    ) -> Result<Self::Returns, HandlerError> {
        println!("Hello world");
        Ok(HelloResponse {})
    }
}
```

## Running Server

To run your application you need to spin up your servers, the `Server`

<!-- TODO: Include example of other databases -->

```rust
use rio_rs::prelude::*;
use rio_rs::cluster::storage::sql::{SqlMembersStorage};
use rio_rs::object_placement::sql::SqlObjectPlacementProvider;

# // Copied from the snippet above
# use async_trait::async_trait;
# use serde::{Deserialize, Serialize};
# use std::sync::Arc;
#
# #[derive(TypeName, Message, Deserialize, Serialize)]
# pub struct HelloMessage {
#     pub name: String
# }
#
# #[derive(TypeName, Message, Deserialize, Serialize)]
# pub struct HelloResponse {}
#
# #[derive(TypeName, FromId, Default)]
# pub struct HelloWorldService {
#     pub id: String,
# }
#
# #[async_trait]
# impl Handler<HelloMessage> for HelloWorldService{
#     type Returns = HelloResponse;
#     async fn handle(
#         &mut self,
#         message: HelloMessage,
#         app_data: Arc<AppData>,
#     ) -> Result<Self::Returns, HandlerError> {
#         println!("Hello world");
#         Ok(HelloResponse {})
#     }
# }

#[tokio::main]
async fn main() {
    let addr = "0.0.0.0:5000";

    // Configure types on the server's registry
    let mut registry = Registry::new();
    registry.add_static_fn::<HelloWorldService, String, _>(FromId::from_id);
    registry.add_handler::<HelloWorldService, HelloMessage>();

    // Configure the Cluster Membership provider
    let pool = SqlMembersStorage::pool()
        .connect("sqlite::memory:")
        .await
        .expect("Membership database connection failure");
    let members_storage = SqlMembersStorage::new(pool);
    members_storage.migrate().await;

    let membership_provider_config = PeerToPeerClusterConfig::default();
    let membership_provider =
        PeerToPeerClusterProvider::new(members_storage, membership_provider_config);

    // Configure the object placement
    let pool = SqlMembersStorage::pool()
        .connect("sqlite::memory:")
        .await
        .expect("Object placement database connection failure");
    let object_placement_provider = SqlObjectPlacementProvider::new(pool);
    object_placement_provider.migrate().await;

    // Create the server object
    let mut server = Server::new(
        addr.to_string(),
        registry,
        membership_provider,
        object_placement_provider,
    );

    // Run the server
    // server.run().await;
}
```

## Client

Communicating with the cluster is just a matter of sending the serialized known messages via TCP.
The [`client`] module provides an easy way of achieving this:

```no_run
use rio_rs::prelude::*;
use rio_rs::cluster::storage::sql::{SqlMembersStorage};

# // Copied from the snippet above
# use async_trait::async_trait;
# use serde::{Deserialize, Serialize};
# use std::sync::Arc;
#
# #[derive(TypeName, Message, Deserialize, Serialize)]
# pub struct HelloMessage {
#     pub name: String
# }
#
# #[derive(TypeName, Message, Deserialize, Serialize)]
# pub struct HelloResponse {}
#
# #[derive(TypeName, FromId, Default)]
# pub struct HelloWorldService {
#     pub id: String,
# }
#
# #[async_trait]
# impl Handler<HelloMessage> for HelloWorldService {
#     type Returns = HelloResponse;
#     async fn handle(
#         &mut self,
#         message: HelloMessage,
#         app_data: Arc<AppData>,
#     ) -> Result<Self::Returns, HandlerError> {
#         println!("Hello world");
#         Ok(HelloResponse {})
#     }
# }

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Member storage configuration (Rendezvous)
    let pool = SqlMembersStorage::pool()
        .connect("sqlite::memory:")
        .await?;
    let members_storage = SqlMembersStorage::new(pool);
    # members_storage.migrate().await;

    // Create the client
    let mut client = ClientBuilder::new()
        .members_storage(members_storage)
        .build()?;

    let payload = HelloMessage { name: "Client".to_string() };
    let response: HelloResponse = client
        .send(
            "HelloWorldService".to_string(),
            "any-string-id".to_string(),
            &payload,
        ).await?;

    // response is a `HelloResponse {}`
    Ok(())
}
```

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
- [ ] Include registry configuration in Server builder
- [ ] Remove need to use `add_static_fn(FromId::from_id)`
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
- [ ] Pub/sub
- [ ] Placement strategies
- [ ] Dockerized examples
- [ ] Add psql jsonb support
- [ ] Add all SQL storage behind a feature flag (sqlite, mysql, pgsql, etc)
- [ ] Supervision
- [ ] Ephemeral objects (aka regular actors)
- [ ] Code of conduct
- [ ] Remove magic numbers
- [ ] Object TTL
- [x] Support service background task
- [ ] Matrix test with different backends
- [ ] Support 'typed' message/response on client
- [ ] Support ephemeral port
- [ ] Examples covering most use cases
  - [ ] Background async task on a service
  - [ ] Background blocking task on a service
