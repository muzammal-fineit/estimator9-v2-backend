use crate::shared::DomainError;

/// A plaintext password that has passed policy, on its way to the hasher.
///
/// It exists so policy is enforced in one place and so the plaintext has a type
/// that refuses to print itself. It is deliberately short-lived: nothing stores
/// a `Password`, and no entity holds one — [`super::PasswordHash`] is what
/// persists.
#[derive(Clone, PartialEq, Eq)]
pub struct Password(String);

impl Password {
    /// NIST SP 800-63B favours length over composition rules, so there is no
    /// "must contain a symbol" here — those push people towards `Passw0rd!`
    /// and are not what stops an attack.
    pub const MIN_LENGTH: usize = 12;

    /// Argon2's cost is a function of input size as well as its parameters, so
    /// an unbounded password is a cheap way to make the server do expensive
    /// work. OWASP suggests capping well above any real password.
    pub const MAX_LENGTH: usize = 128;

    pub fn parse(raw: impl Into<String>) -> Result<Self, DomainError> {
        let value = raw.into();

        // Counting chars, not bytes: a 12-character password in a non-Latin
        // script would otherwise be judged by its UTF-8 length.
        let length = value.chars().count();

        if length < Self::MIN_LENGTH {
            return Err(DomainError::invalid(
                "password",
                format!("must be at least {} characters", Self::MIN_LENGTH),
            ));
        }

        if length > Self::MAX_LENGTH {
            return Err(DomainError::invalid(
                "password",
                format!("must be at most {} characters", Self::MAX_LENGTH),
            ));
        }

        Ok(Self(value))
    }

    /// Only the hasher should need this.
    pub fn expose(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Debug for Password {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Password(<redacted>)")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn never_renders_the_plaintext_in_debug_output() {
        let password = Password::parse("correct horse battery staple").unwrap();

        assert_eq!(format!("{password:?}"), "Password(<redacted>)");
    }

    #[test]
    fn enforces_length_at_both_ends() {
        assert!(Password::parse("short").is_err());
        assert!(Password::parse("a".repeat(Password::MIN_LENGTH)).is_ok());
        assert!(Password::parse("a".repeat(Password::MAX_LENGTH)).is_ok());
        assert!(Password::parse("a".repeat(Password::MAX_LENGTH + 1)).is_err());
    }

    #[test]
    fn measures_length_in_characters_not_bytes() {
        // 12 characters, 36 bytes in UTF-8.
        assert!(Password::parse("парольпароль").is_ok());
    }
}
