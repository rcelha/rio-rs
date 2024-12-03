//! Distributed stateful services inspired by Orleans
//!
//! This crate provides a framework for scalable, distributed and stateful services
//! based on message passing between objects
//!
//! # Application
//!
//! Most of your application code will be written in forms of `ServiceObjects` and `Messages`
//!
//! ```rust
//! use async_trait::async_trait;
//! use rio_rs::prelude::*;
//! use serde::{Deserialize, Serialize};
//! use std::sync::Arc;
//!
//! #[derive(TypeName, Message, Deserialize, Serialize)]
//! pub struct HelloMessage {
//!     pub name: String
//! }
//!
//! #[derive(TypeName, Message, Deserialize, Serialize)]
//! pub struct HelloResponse {}
//!
//! #[derive(TypeName, WithId, Default)]
//! pub struct HelloWorldService {
//!     pub id: String,
//! }
//!
//! #[async_trait]
//! impl Handler<HelloMessage> for HelloWorldService {
//!     type Returns = HelloResponse;
//!     async fn handle(
//!         &mut self,
//!         message: HelloMessage,
//!         app_data: Arc<AppData>,
//!     ) -> Result<Self::Returns, HandlerError> {
//!         println!("Hello world");
//!         Ok(HelloResponse {})
//!     }
//! }
//!
//! ```
//!
//! # Running Server
//!
//! To run your application you need to spin up your servers, the `Server`
//!
//! TODO: Include example of other databases
//!
//! ```rust
//! use rio_rs::prelude::*;
//! use rio_rs::cluster::storage::sql::{SqlMembersStorage};
//! use rio_rs::object_placement::sql::SqlObjectPlacementProvider;
//!
//! # // Copied from the snippet above
//! # use async_trait::async_trait;
//! # use serde::{Deserialize, Serialize};
//! # use std::sync::Arc;
//! # #[derive(TypeName, Message, Deserialize, Serialize)]
//! # pub struct HelloMessage {
//! #     pub name: String
//! # }
//! # #[derive(TypeName, Message, Deserialize, Serialize)]
//! # pub struct HelloResponse {}
//! # #[derive(TypeName, WithId, Default)]
//! # pub struct HelloWorldService {
//! #     pub id: String,
//! # }
//! # #[async_trait]
//! # impl Handler<HelloMessage> for HelloWorldService{
//! #     type Returns = HelloResponse;
//! #     async fn handle(
//! #         &mut self,
//! #         message: HelloMessage,
//! #         app_data: Arc<AppData>,
//! #     ) -> Result<Self::Returns, HandlerError> {
//! #         println!("Hello world");
//! #         Ok(HelloResponse {})
//! #     }
//! # }
//!
//! #[tokio::main]
//! async fn main() {
//!     let addr = "0.0.0.0:0";
//!
//!     // Configure types on the server's registry
//!     let mut registry = Registry::new();
//!     registry.add_type::<HelloWorldService>();
//!     registry.add_handler::<HelloWorldService, HelloMessage>();
//!
//!     // Configure the Cluster Membership provider
//!     let pool = SqlMembersStorage::pool()
//!         .connect("sqlite::memory:")
//!         .await
//!         .expect("Membership database connection failure");
//!     let members_storage = SqlMembersStorage::new(pool);
//!
//!     let membership_provider_config = PeerToPeerClusterConfig::default();
//!     let membership_provider =
//!         PeerToPeerClusterProvider::new(members_storage, membership_provider_config);
//!
//!     // Configure the object placement
//!     let pool = SqlMembersStorage::pool()
//!         .connect("sqlite::memory:")
//!         .await
//!         .expect("Object placement database connection failure");
//!     let object_placement_provider = SqlObjectPlacementProvider::new(pool);
//!
//!     // Create the server object
//!     let mut server = Server::new(
//!         addr.to_string(),
//!         registry,
//!         membership_provider,
//!         object_placement_provider,
//!     );
//!     server.prepare().await;
//!     let listener = server.bind().await.expect("Bind");
//!     // Run the server
//!     // server.run(listener).await;
//! }
//! ```
//!
//! # Client
//!
//! Communicating with the cluster is just a matter of sending the serialized known messages via TCP.
//! The [`client`] module provides an easy way of achieving this:
//!
//! ```no_run
//! use rio_rs::prelude::*;
//! use rio_rs::cluster::storage::sql::{SqlMembersStorage};
//!
//! # // Copied from the snippet above
//! # use async_trait::async_trait;
//! # use serde::{Deserialize, Serialize};
//! # use std::sync::Arc;
//! # #[derive(TypeName, Message, Deserialize, Serialize)]
//! # pub struct HelloMessage {
//! #     pub name: String
//! # }
//! # #[derive(TypeName, Message, Deserialize, Serialize)]
//! # pub struct HelloResponse {}
//! # #[derive(TypeName, WithId, Default)]
//! # pub struct HelloWorldService {
//! #     pub id: String,
//! # }
//! # #[async_trait]
//! # impl Handler<HelloMessage> for HelloWorldService {
//! #     type Returns = HelloResponse;
//! #     async fn handle(
//! #         &mut self,
//! #         message: HelloMessage,
//! #         app_data: Arc<AppData>,
//! #     ) -> Result<Self::Returns, HandlerError> {
//! #         println!("Hello world");
//! #         Ok(HelloResponse {})
//! #     }
//! # }
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Member storage configuration (Rendezvous)
//!     let pool = SqlMembersStorage::pool()
//!         .connect("sqlite::memory:")
//!         .await?;
//!     let members_storage = SqlMembersStorage::new(pool);
//!     # members_storage.prepare().await;
//!
//!     // Create the client
//!     let mut client = ClientBuilder::new()
//!         .members_storage(members_storage)
//!         .build()?;
//!
//!     let payload = HelloMessage { name: "Client".to_string() };
//!     let response: HelloResponse = client
//!         .send(
//!             "HelloWorldService".to_string(),
//!             "any-string-id".to_string(),
//!             &payload,
//!         ).await?;
//!
//!     // response is a `HelloResponse {}`
//!     Ok(())
//! }
//! ```

#![deny(rustdoc::broken_intra_doc_links)]
// #![warn(missing_docs)]
#![deny(rustdoc::missing_crate_level_docs)]

pub mod app_data;
pub mod client;
pub mod cluster;
pub mod errors;
pub mod message_router;
pub mod object_placement;
pub mod protocol;
pub mod registry;
pub mod server;
pub mod service;
pub mod service_object;
pub mod sql_migration;
pub mod state;

pub use service_object::*;

/// Re-exports of [rio_macros]
pub mod derive {
    pub use rio_macros::ManagedState;
    pub use rio_macros::Message;
    pub use rio_macros::TypeName;
    pub use rio_macros::WithId;
}

pub mod prelude {
    pub use super::app_data::AppData;
    pub use super::client::ClientBuilder;
    pub use super::cluster::membership_protocol::peer_to_peer::{
        PeerToPeerClusterConfig, PeerToPeerClusterProvider,
    };
    pub use super::cluster::membership_protocol::ClusterProvider;
    pub use super::cluster::storage::MembersStorage;
    pub use super::derive::{ManagedState, Message, TypeName, WithId};
    pub use super::errors::{ClientBuilderError, HandlerError, ServiceObjectLifeCycleError};
    pub use super::protocol::{ClientError, RequestError, ResponseError};

    pub use super::registry::{Handler, Registry};

    pub use super::server::Server;
    pub use super::server::ServerBuilder;
    pub use super::state::ObjectStateManager;
    pub use super::LifecycleMessage;
    pub use super::ObjectId;
    pub use super::ServiceObject;
    pub use super::ServiceObjectStateLoad;
    pub use super::WithId;
}
