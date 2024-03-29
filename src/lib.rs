#![deny(rustdoc::broken_intra_doc_links)]
// #![warn(missing_docs)]
#![deny(rustdoc::missing_crate_level_docs)]
#![doc = include_str!("../README.md")]

pub mod app_data;
pub mod client;
pub mod cluster;
pub mod errors;
pub mod object_placement;
pub mod protocol;
pub mod registry;
pub mod server;
pub mod service;
pub mod service_object;
pub mod state;
pub mod tap_err;

pub use service_object::*;

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
    pub use super::protocol::{ClientError, ResponseError};

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
