use argon2::Argon2;
use argon2::password_hash::phc::PasswordHash as PhcHash;
use argon2::password_hash::{Error as PhcError, PasswordHasher as _, PasswordVerifier};
use async_trait::async_trait;
use domain::identity::{HashingError, Password, PasswordHash, PasswordHasher};

/// Argon2id with the `argon2` crate's defaults, which track the OWASP
/// recommendation (19 MiB, 2 iterations, 1 lane).
///
/// The cost is the point: a hash fast enough to feel instant is fast enough to
/// brute force offline. Expect tens of milliseconds per call, and treat that as
/// the feature rather than something to tune away.
pub struct Argon2Hasher;

impl Argon2Hasher {
    pub fn new() -> Self {
        Self
    }
}

impl Default for Argon2Hasher {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl PasswordHasher for Argon2Hasher {
    async fn hash(&self, password: &Password) -> Result<PasswordHash, HashingError> {
        // Owned copy so the work can move to a blocking thread. Hashing on an
        // async worker would park that thread for the whole duration, starving
        // every other request the runtime had scheduled on it.
        let plaintext = password.expose().to_owned();

        let phc = blocking(move || {
            // The salt is generated internally from the OS RNG.
            Argon2::default()
                .hash_password(plaintext.as_bytes())
                .map(|hash| hash.to_string())
                .map_err(HashingError::backend)
        })
        .await?;

        PasswordHash::parse(phc).map_err(|_| HashingError::MalformedHash)
    }

    async fn verify(&self, password: &Password, hash: &PasswordHash) -> Result<bool, HashingError> {
        let plaintext = password.expose().to_owned();
        let stored = hash.as_str().to_owned();

        blocking(move || {
            let parsed = PhcHash::new(&stored).map_err(|_| HashingError::MalformedHash)?;

            // `verify_password` answers `PasswordInvalid` when the stored hash
            // has no salt or no output — the same answer as a wrong password.
            // A row like that would then reject every login forever with
            // nothing to say why, so it is caught here instead.
            if parsed.salt.is_none() || parsed.hash.is_none() {
                return Err(HashingError::MalformedHash);
            }

            match Argon2::default().verify_password(plaintext.as_bytes(), &parsed) {
                Ok(()) => Ok(true),

                // A mismatch is an expected answer, not a failure.
                Err(PhcError::PasswordInvalid) => Ok(false),

                // The stored hash is syntactically fine but unusable: a
                // different algorithm, bad base64, an unsupported version,
                // nonsense parameters. Every one of these is a problem with
                // that row — the account needs a password reset — and none of
                // them says anything is wrong with the hasher.
                Err(
                    PhcError::Algorithm
                    | PhcError::EncodingInvalid
                    | PhcError::OutputSize
                    | PhcError::ParamInvalid { .. }
                    | PhcError::ParamsInvalid
                    | PhcError::SaltInvalid
                    | PhcError::Version,
                ) => Err(HashingError::MalformedHash),

                // Crypto, Internal, OutOfMemory, RngFailure: the hasher itself
                // failed. Worth alerting on, unlike the arm above.
                Err(err) => Err(HashingError::backend(err)),
            }
        })
        .await
    }
}

/// Runs CPU-bound work on the blocking pool and flattens the join error.
async fn blocking<T, F>(work: F) -> Result<T, HashingError>
where
    F: FnOnce() -> Result<T, HashingError> + Send + 'static,
    T: Send + 'static,
{
    match tokio::task::spawn_blocking(work).await {
        Ok(result) => result,
        Err(join) => Err(HashingError::backend(join)),
    }
}

#[cfg(test)]
#[path = "argon2_hasher_tests.rs"]
mod tests;
