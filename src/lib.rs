#![deny(rustdoc::broken_intra_doc_links)]
// #![warn(missing_docs)]
#![deny(rustdoc::missing_crate_level_docs)]
#![doc = include_str!("../README.md")]

pub mod app_data;
pub mod client;
pub mod cluster_provider;
pub mod errors;
pub mod grain;
pub mod grain_placement_provider;
pub mod membership_provider;
pub mod protocol;
pub mod registry;
pub mod server;
pub mod service;
pub mod state_provider;

pub use grain::*;

pub mod derive {
    pub use rio_macros::FromId;
    pub use rio_macros::ManagedState;
    pub use rio_macros::Message;
    pub use rio_macros::TypeName;
}

pub mod prelude {
    pub use super::app_data::AppData;
    pub use super::client::ClientBuilder;
    pub use super::cluster_provider::peer_to_peer::{
        PeerToPeerClusterConfig, PeerToPeerClusterProvider,
    };
    pub use super::cluster_provider::ClusterProvider;
    pub use super::derive::{FromId, ManagedState, Message, TypeName};
    pub use super::errors::{ClientBuilderError, ClientError, GrainLifeCycleError, HandlerError};
    pub use super::membership_provider::MembersStorage;
    pub use super::registry::{Handler, Registry};

    pub use super::server::Server;
    pub use super::service::Service;
    // pub use super::silo::Silo;
    pub use super::state_provider::ObjectStateManager;
    pub use super::FromId;
    pub use super::Grain;
    pub use super::GrainId;
    pub use super::GrainStateLoad;
    pub use super::LifecycleMessage;
}
