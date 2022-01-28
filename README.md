# Rio

Distributed computing inspired by Orleans

This crate provides a framework for scalable, distributed and stateful services
based on message passing between objects

The naming adopted throughout the crate a based off of Orleans, this way it is
easier to understand the similarities and differences between both systems

## Application

Most of your application code will be written in forms of `Grains` and `Messages`

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
pub struct HelloWorldGrain {
    pub id: String,
}

#[async_trait]
impl Handler<HelloMessage> for HelloWorldGrain {
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

To run your application you need to spin up your servers, the `Silos`

```rust
use rio_rs::prelude::*;
use rio_rs::membership_provider::sql::{SqlMembersStorage};
use rio_rs::grain_placement_provider::sql::SqlGrainPlacementProvider;

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
# pub struct HelloWorldGrain {
#     pub id: String,
# }
#
# #[async_trait]
# impl Handler<HelloMessage> for HelloWorldGrain {
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

    // Configure types on the Silo's registry
    let mut registry = Registry::new();
    registry.add_static_fn::<HelloWorldGrain, String, _>(FromId::from_id);
    registry.add_handler::<HelloWorldGrain, HelloMessage>();

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

    // Configure the grain placement
    let pool = SqlMembersStorage::pool()
        .connect("sqlite::memory:")
        .await
        .expect("Grain placement database connection failure");
    let grain_placement_provider = SqlGrainPlacementProvider::new(pool);
    grain_placement_provider.migrate().await;

    // Create the server object
    let mut silo = Silo::new(
        addr.to_string(),
        registry,
        membership_provider,
        grain_placement_provider,
    );

    // Run the server
    // silo.serve().await;
}
```

## Client

Communicating with the cluster is just a matter of sending the serialized known messages via TCP.
The [`client`] module provides an easy way of achieving this:

```no_run
use rio_rs::prelude::*;
use rio_rs::membership_provider::sql::{SqlMembersStorage};

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
# pub struct HelloWorldGrain {
#     pub id: String,
# }
#
# #[async_trait]
# impl Handler<HelloMessage> for HelloWorldGrain {
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
            "HelloWorldGrain".to_string(),
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
- [ ] Harden networking (only happy path is implemented)
- [ ] Public API renaming (will we use Orleans' naming?)
- [ ] Increase public API test coverage
- [ ] 100% documentation of public API
- [ ] Pub/sub
- [ ] Placement strategies
- [ ] Naive object persistence
- [ ] Dockerized examples
- [ ] Supervision
- [ ] Regular actors (anonym actors)
