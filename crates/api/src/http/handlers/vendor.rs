//! The vendor panel.
//!
//! Every endpoint here is restricted to the vendor account and unreachable by
//! any client role, however privileged. That is the point: these control what
//! the installation is licensed to do, which is not the client's to decide.

pub mod list_features;
pub mod set_feature;

pub use list_features::*;
pub use set_feature::*;
