use async_trait::async_trait;

use super::FeatureSet;

/// Where the enabled set is kept between restarts.
///
/// The port says nothing about encryption or files — that is the adapter's
/// business, and swapping it for a vendor-signed licence later changes nothing
/// above this line.
#[async_trait]
pub trait FeatureStore: Send + Sync + 'static {
    /// Loads the set, falling back to [`FeatureSet::defaults`] when there is
    /// nothing stored yet.
    ///
    /// A file that exists but cannot be read is a [`StoreError`], never a
    /// silent fallback: quietly reverting to defaults would turn tampering
    /// into a way to *change* behaviour rather than to break it.
    async fn load(&self) -> Result<FeatureSet, StoreError>;

    async fn save(&self, features: &FeatureSet) -> Result<(), StoreError>;
}

#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    /// The stored state failed its integrity check — edited, truncated, or
    /// written by a binary with a different key. Worth an alert: on a client
    /// installation it means somebody tried.
    #[error("the stored feature state has been tampered with or is unreadable")]
    Tampered,

    #[error("the feature state could not be read or written")]
    Io(#[source] Box<dyn std::error::Error + Send + Sync>),
}

impl StoreError {
    pub fn io(source: impl std::error::Error + Send + Sync + 'static) -> Self {
        Self::Io(Box::new(source))
    }
}
