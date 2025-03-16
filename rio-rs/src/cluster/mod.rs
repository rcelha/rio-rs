//! This module holds all the modules responsible for clustering support

/// Controls which cluster members' are healthy
pub mod membership_protocol;

/// Stores a list of running servers
///
/// It serves Rendevouz and Cluster Membership APIs
pub mod storage;
