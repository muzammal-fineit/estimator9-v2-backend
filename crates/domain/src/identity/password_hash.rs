use crate::shared::DomainError;

/// A hashed password — never the plaintext, which has no domain type at all
/// and exists only as a `&str` travelling from the request into the hasher.
///
/// `Debug` is hand-written to print a placeholder. A derived one would leak the
/// hash into every `tracing::error!(?user)` and every audit entry that ever
/// carries a `User`, which is exactly the accident this type exists to prevent.
#[derive(Clone, PartialEq, Eq)]
pub struct PasswordHash(String);

impl PasswordHash {
    /// Wrap a hash produced by a `PasswordHasher`, or read back from storage.
    ///
    /// Only emptiness is rejected for now. The check tightens to PHC format
    /// (`$argon2id$...`) in phase 2, once every stored row holds a real Argon2
    /// hash — doing it here would make the existing smoke-test rows unreadable
    /// and take `/users` down with a 500.
    pub fn parse(raw: impl Into<String>) -> Result<Self, DomainError> {
        let value = raw.into();

        if value.trim().is_empty() {
            return Err(DomainError::invalid("password_hash", "must not be blank"));
        }

        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Debug for PasswordHash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("PasswordHash(<redacted>)")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn never_renders_the_hash_in_debug_output() {
        let hash = PasswordHash::parse("$argon2id$v=19$m=19456,t=2,p=1$abc$def").unwrap();

        let rendered = format!("{hash:?}");

        assert_eq!(rendered, "PasswordHash(<redacted>)");
        assert!(!rendered.contains("argon2"));
        assert!(!rendered.contains("def"));
    }

    #[test]
    fn rejects_a_blank_hash() {
        assert!(PasswordHash::parse("   ").is_err());
        assert!(PasswordHash::parse("").is_err());
    }
}
