use crate::shared::DomainError;

/// A role's identifier, as stored in `roles.name`.
///
/// Lowercase ASCII so that comparisons — including the superadmin check the
/// whole guard rests on — can never turn on casing.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RoleName(String);

impl RoleName {
    /// The role that bypasses permission checks entirely. Held in one place so
    /// no guard, seed, or test can drift to a different spelling.
    pub const SUPERADMIN: &'static str = "superadmin";

    pub fn parse(raw: impl Into<String>) -> Result<Self, DomainError> {
        let value = raw.into().trim().to_owned();

        let well_formed = !value.is_empty()
            && value
                .chars()
                .all(|c| c.is_ascii_lowercase() || c == '_' || c.is_ascii_digit());

        if well_formed {
            Ok(Self(value))
        } else {
            Err(DomainError::invalid(
                "role_name",
                format!("{value:?} is not lowercase ascii"),
            ))
        }
    }

    pub fn superadmin() -> Self {
        Self(Self::SUPERADMIN.to_owned())
    }

    pub fn is_superadmin(&self) -> bool {
        self.0 == Self::SUPERADMIN
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for RoleName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognises_the_superadmin_role() {
        assert!(RoleName::parse("superadmin").unwrap().is_superadmin());
        assert!(RoleName::superadmin().is_superadmin());
        assert!(!RoleName::parse("admin").unwrap().is_superadmin());
    }

    #[test]
    fn rejects_casing_that_could_dodge_the_superadmin_check() {
        assert!(RoleName::parse("SuperAdmin").is_err());
        assert!(RoleName::parse("SUPERADMIN").is_err());
    }

    #[test]
    fn rejects_empty_and_punctuated_names() {
        assert!(RoleName::parse("  ").is_err());
        assert!(RoleName::parse("super-admin").is_err());
    }
}
