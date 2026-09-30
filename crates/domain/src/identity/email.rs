use crate::shared::DomainError;

/// A syntactically valid email address.
///
/// Case is preserved as the user typed it. Uniqueness is case-insensitive and
/// is enforced by the `users_email_lower_key` index, not here — a value object
/// can only judge one value, never the set.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Email(String);

impl Email {
    pub fn parse(raw: impl Into<String>) -> Result<Self, DomainError> {
        let value = raw.into().trim().to_owned();

        match value.split_once('@') {
            Some((local, host)) if !local.is_empty() && host.contains('.') => Ok(Self(value)),
            _ => Err(DomainError::invalid("email", format!("{value:?}"))),
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for Email {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_a_normal_address_and_keeps_its_case() {
        let email = Email::parse("  Bank.User@x.com ").unwrap();
        assert_eq!(email.as_str(), "Bank.User@x.com");
    }

    #[test]
    fn rejects_addresses_without_a_local_part_or_dotted_host() {
        assert!(Email::parse("@x.com").is_err());
        assert!(Email::parse("user@localhost").is_err());
        assert!(Email::parse("user").is_err());
    }
}
