//! Tests for [`super::Argon2Hasher`], in their own file so the adapter stays
//! under the 150-line limit — the escape hatch described in
//! docs/ARCHITECTURE.md under "One thing per file".

use super::*;

#[tokio::test]
async fn a_hash_verifies_against_its_own_password_and_nothing_else() {
    let hasher = Argon2Hasher::new();
    let password = Password::parse("correct horse battery staple").unwrap();
    let other = Password::parse("incorrect horse battery staple").unwrap();

    let hash = hasher.hash(&password).await.unwrap();

    assert!(hasher.verify(&password, &hash).await.unwrap());
    assert!(!hasher.verify(&other, &hash).await.unwrap());
}

#[tokio::test]
async fn the_same_password_hashes_differently_every_time() {
    let hasher = Argon2Hasher::new();
    let password = Password::parse("correct horse battery staple").unwrap();

    let first = hasher.hash(&password).await.unwrap();
    let second = hasher.hash(&password).await.unwrap();

    // Different salts, so identical passwords are not identifiable by
    // comparing stored hashes.
    assert_ne!(first.as_str(), second.as_str());
    assert!(first.as_str().starts_with("$argon2id$"));
}

/// Every route to an unusable stored hash must look the same to the caller,
/// and none of them may be mistaken for a wrong password.
///
/// The first case is the dangerous one: it is syntactically valid PHC, so
/// `verify_password` answers `PasswordInvalid` — identical to a wrong
/// password. Left unhandled, such a row rejects every login forever and says
/// nothing about why.
#[tokio::test]
async fn a_corrupt_stored_hash_is_an_error_not_a_silent_false() {
    let hasher = Argon2Hasher::new();
    let password = Password::parse("correct horse battery staple").unwrap();

    for stored in [
        "$not-a-real-phc-string",                   // valid PHC, no salt or output
        "plaintext-somebody-stored-here",           // not PHC at all
        "$argon2id$v=19$m=19456,t=2,p=1$####$####", // argon2, invalid base64
    ] {
        let broken = PasswordHash::parse(stored).unwrap();

        assert!(
            matches!(
                hasher.verify(&password, &broken).await,
                Err(HashingError::MalformedHash)
            ),
            "{stored:?} should report a malformed hash"
        );
    }
}
