# rio-rs

![GitHub Actions Workflow Status](https://img.shields.io/github/actions/workflow/status/rcelha/rio-rs/.github%2Fworkflows%2Frust.yaml?style=for-the-badge)
[![Crates.io Version](https://img.shields.io/crates/v/rio-rs?style=for-the-badge&link=https%3A%2F%2Fcrates.io%2Fcrates%2Frio-rs)](https://crates.io/crates/rio-rs)
[![docs.rs](https://img.shields.io/docsrs/rio-rs?style=for-the-badge&link=https%3A%2F%2Fdocs.rs%2Frio-rs%2Flatest%2Frio_rs%2F)](https://docs.rs/rio-rs/latest/rio_rs/)

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
use rio_rs::cluster::storage::sqlite::SqliteMembershipStorage;
use rio_rs::object_placement::sqlite::SqliteObjectPlacement;


#[tokio::main]
async fn main() {
    let addr = "0.0.0.0:0";

    // Configure types on the server's registry
    let mut registry = Registry::new();
    registry.add_type::<HelloWorldService>();
    registry.add_handler::<HelloWorldService, HelloMessage>();

    // Configure the Cluster Membership provider
    let pool = SqliteMembershipStorage::pool()
        .connect("sqlite::memory:")
        .await
        .expect("Membership database connection failure");
    let members_storage = SqliteMembershipStorage::new(pool);

    let membership_provider = PeerToPeerClusterProvider::builder()
        .members_storage(members_storage)
        .build();

    // Configure the object placement
    let pool = SqliteMembershipStorage::pool()
        .connect("sqlite::memory:")
        .await
        .expect("Object placement database connection failure");
    let object_placement_provider = SqliteObjectPlacement::new(pool);

    // Create the server object
    let mut server = Server::builder()
        .address(addr.to_string())
        .registry(registry)
        .app_data(AppData::new())
        .cluster_provider(membership_provider)
        .object_placement_provider(object_placement_provider)
        .build();
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
use rio_rs::cluster::storage::sqlite::SqliteMembershipStorage;


#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Member storage configuration (Rendezvous)
    let pool = SqliteMembershipStorage::pool()
        .connect("sqlite::memory:")
        .await?;
    let members_storage = SqliteMembershipStorage::new(pool);
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

## Features

Here are some of the features that are fully implemented:

### Clustering

Clustering is divided in two parts: `Membership Protocol` and `Membership Storage`.

The Membership Storage is responsible for the rendezvous of the cluster, it manages which nodes are members of the
clusters, and how to store the nodes' state in the cluster. Both server and client need to have access to the Membership Storage.

The Membership Protocol is a server that run in each node of the cluster, it is reponsible for testing
the nodes to define which nodes are alive and which are dead.
The Memebership Protocols utilize the Membership Storage to store the state of the nodes in the cluster.

Currently, we only have a `PeerToPeerClusterProvider`, which is a simple implementation of the cluster membership protocol that uses a gossip protocol to keep track of the nodes in the cluster.

As for Storages, we have a few:

- LocalStorage: A simple in-memory storage, built just for testing
- HttpMembershipStorage: A read-only storage that uses HTTP API to expose information of the cluster, it is useful to use this on the client side, but it should never be used on the server side, since it is read-only and the server needs to update the state of the cluster.
- PostgresMembershipStorage
- RedisMembershipStorage
- SqliteMembershipStorage

### Object Placement

Object Placement maps each object's location in the cluster. Only the server has access to the Object Placement, and it is used by the server to know where to send the requests for each object.

- LocalObjectPlacement: Simple in-memory object placement, built just for testing
- PostgresObjectPlacement
- RedisObjectPlacement
- SqliteObjectPlacement

### Object Persistence (Managed State)

Rio offers a way to manage the state of your objects in a persistent storage.
You can simply drop the `ManagedState` derive on your struct, and it will automatically implement necessary
traits to serialize and deserialize your struct, and to save and load it from a persistence backend.
Alternatively, you can implement the persistence traits manually, if you need more control over how your state is saved and loaded.

Here are the built-in persistence backends:

- LocalState: A simple in-memory state, built just for testing
- PostgresState
- RedisState
- SqliteState
