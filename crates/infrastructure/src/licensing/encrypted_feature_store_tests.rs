//! Tests for [`super::EncryptedFeatureStore`].

use domain::licensing::feature::lookup;

use super::*;

fn store(name: &str) -> (EncryptedFeatureStore, PathBuf) {
    let path = std::env::temp_dir().join(format!("estimator9-features-{name}.bin"));
    let _ = std::fs::remove_file(&path);

    (EncryptedFeatureStore::new(path.clone()), path)
}

fn mev() -> domain::licensing::FeatureName {
    lookup("mev.fitting").map(|f| f.name).unwrap()
}

#[tokio::test]
async fn a_missing_file_means_a_fresh_installation() {
    let (store, _) = store("missing");

    assert_eq!(store.load().await.unwrap(), FeatureSet::defaults());
}

#[tokio::test]
async fn what_was_saved_is_what_loads() {
    let (store, _) = store("roundtrip");

    let mut features = FeatureSet::defaults();
    features.set(mev(), true);

    store.save(&features).await.unwrap();

    assert_eq!(store.load().await.unwrap(), features);
}

/// The property the design rests on: an edited file is refused rather than
/// decrypting to something plausible, so tampering breaks the installation
/// visibly instead of quietly granting features.
#[tokio::test]
async fn an_edited_file_is_rejected_rather_than_honoured() {
    let (store, path) = store("tampered");

    store.save(&FeatureSet::defaults()).await.unwrap();

    let mut raw = std::fs::read(&path).unwrap();
    let last = raw.len() - 1;
    raw[last] ^= 0xff;
    std::fs::write(&path, raw).unwrap();

    assert!(matches!(store.load().await, Err(StoreError::Tampered)));
}

#[tokio::test]
async fn a_truncated_file_is_rejected() {
    let (store, path) = store("truncated");

    store.save(&FeatureSet::defaults()).await.unwrap();
    std::fs::write(&path, b"short").unwrap();

    assert!(matches!(store.load().await, Err(StoreError::Tampered)));
}

/// Plaintext must not be recoverable by reading the file, even though the key
/// is recoverable from the binary — the two are different bars.
#[tokio::test]
async fn the_feature_names_are_not_readable_in_the_file() {
    let (store, path) = store("opaque");

    let mut features = FeatureSet::defaults();
    features.set(mev(), true);
    store.save(&features).await.unwrap();

    let raw = std::fs::read(&path).unwrap();

    assert!(!String::from_utf8_lossy(&raw).contains("mev.fitting"));
}
