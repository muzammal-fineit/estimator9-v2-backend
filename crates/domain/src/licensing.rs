//! Licensing context: which software features this installation may use.
//!
//! Deliberately *not* modelled in the database. The client owns their Postgres,
//! so a flag stored there is a flag they control — which is the opposite of the
//! point. The catalogue is compiled into the binary and the state lives in an
//! encrypted file only the binary can read.
//!
//! What that buys, precisely: tamper-*evidence*, not tamper-*proofing*. The key
//! is in the binary, so someone holding the binary can in principle extract it.
//! The cost rises from "edit a table" to "reverse-engineer an executable", and
//! every change made through the supported path is audited.

pub mod feature;
pub mod feature_store;

pub use feature::{Feature, FeatureName, FeatureSet};
pub use feature_store::{FeatureStore, StoreError};
