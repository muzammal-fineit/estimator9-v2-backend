use std::path::PathBuf;

use async_trait::async_trait;
use chacha20poly1305::aead::{Aead, KeyInit};
use chacha20poly1305::{Key, XChaCha20Poly1305, XNonce};
use domain::licensing::{FeatureSet, FeatureStore, StoreError};

/// The key is baked in at build time, never read from the environment at run
/// time — an installation must not be able to supply its own.
///
/// Release builds set `ESTIMATOR9_FEATURE_KEY` (32 bytes, hex) so the value is
/// not in version control and can differ per release. The fallback exists only
/// so a development checkout builds; it protects nothing, and DEPLOYMENT.md
/// says so.
const KEY_HEX: &str = match option_env!("ESTIMATOR9_FEATURE_KEY") {
    Some(key) => key,
    None => "6465762d6f6e6c792d6b65792d6e6f742d666f722d7265616c2d7573652d3031",
};

const NONCE_BYTES: usize = 24;

/// Keeps the enabled feature set in an encrypted file.
///
/// XChaCha20-Poly1305 is authenticated, which is the property that matters
/// here: the point is not to keep the list secret so much as to know when the
/// stored state has been edited. A modified file fails its tag check and
/// becomes [`StoreError::Tampered`] rather than decrypting to something
/// plausible.
///
/// This is tamper-evidence, not tamper-proofing. The key ships in the binary,
/// so someone holding the binary can extract it. See the module docs on
/// `domain::licensing`.
pub struct EncryptedFeatureStore {
    path: PathBuf,
}

impl EncryptedFeatureStore {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    fn cipher() -> Result<XChaCha20Poly1305, StoreError> {
        let bytes = hex::decode(KEY_HEX).map_err(StoreError::io)?;

        let key = Key::try_from(bytes.as_slice())
            .map_err(|_| StoreError::Io("ESTIMATOR9_FEATURE_KEY must be 32 bytes of hex".into()))?;

        Ok(XChaCha20Poly1305::new(&key))
    }
}

#[async_trait]
impl FeatureStore for EncryptedFeatureStore {
    async fn load(&self) -> Result<FeatureSet, StoreError> {
        let raw = match tokio::fs::read(&self.path).await {
            Ok(raw) => raw,

            // Nothing stored yet: a fresh installation, which gets the
            // defaults. Distinct from a file that exists and will not open.
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                return Ok(FeatureSet::defaults());
            }

            Err(err) => return Err(StoreError::io(err)),
        };

        if raw.len() <= NONCE_BYTES {
            return Err(StoreError::Tampered);
        }

        let (nonce, ciphertext) = raw.split_at(NONCE_BYTES);
        let nonce = XNonce::try_from(nonce).map_err(|_| StoreError::Tampered)?;

        let plaintext = Self::cipher()?
            .decrypt(&nonce, ciphertext)
            .map_err(|_| StoreError::Tampered)?;

        let names = String::from_utf8(plaintext).map_err(|_| StoreError::Tampered)?;

        // Unknown names are dropped by `rehydrate`, so even a correctly
        // encrypted file naming features this binary has never heard of grants
        // nothing.
        Ok(FeatureSet::rehydrate(
            names.split('\n').filter(|line| !line.is_empty()),
        ))
    }

    async fn save(&self, features: &FeatureSet) -> Result<(), StoreError> {
        let plaintext = features.names().collect::<Vec<_>>().join("\n");

        let mut nonce = [0u8; NONCE_BYTES];
        getrandom::fill(&mut nonce).map_err(StoreError::io)?;

        let ciphertext = Self::cipher()?
            .encrypt(&XNonce::from(nonce), plaintext.as_bytes())
            .map_err(|_| StoreError::Tampered)?;

        if let Some(parent) = self.path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(StoreError::io)?;
        }

        let mut out = nonce.to_vec();
        out.extend_from_slice(&ciphertext);

        // Written to a temporary file and renamed, so a crash mid-write cannot
        // leave a half-file that reads as tampering.
        let temporary = self.path.with_extension("tmp");
        tokio::fs::write(&temporary, &out)
            .await
            .map_err(StoreError::io)?;
        tokio::fs::rename(&temporary, &self.path)
            .await
            .map_err(StoreError::io)?;

        Ok(())
    }
}

#[cfg(test)]
#[path = "encrypted_feature_store_tests.rs"]
mod tests;
