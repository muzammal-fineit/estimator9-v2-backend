use std::borrow::Cow;

use crate::shared::DomainError;

/// One thing a user may do, shaped `resource.verb` — `users.manage`,
/// `provision.approve`.
///
/// The convention is enforced rather than merely documented: a typo like
/// `usersmanage` would otherwise become a permission that silently matches
/// nothing, which fails open in the worst possible way.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PermissionName(Cow<'static, str>);

impl PermissionName {
    // The vocabulary the code checks against, as constants so a handler cannot
    // mistype one. `Cow` rather than `String` exists precisely so these can be
    // `const`: a typo becomes a compile error, while values read from the
    // database still go through `parse`.
    pub const USERS_READ: Self = Self(Cow::Borrowed("users.read"));
    pub const USERS_MANAGE: Self = Self(Cow::Borrowed("users.manage"));
    pub const ROLES_READ: Self = Self(Cow::Borrowed("roles.read"));
    pub const AUDIT_READ: Self = Self(Cow::Borrowed("audit.read"));

    pub fn parse(raw: impl Into<String>) -> Result<Self, DomainError> {
        let value = raw.into().trim().to_owned();

        let Some((resource, verb)) = value.split_once('.') else {
            return Err(DomainError::invalid(
                "permission_name",
                format!("{value:?} is not shaped resource.verb"),
            ));
        };

        let segment_ok = |s: &str| {
            !s.is_empty()
                && s.chars()
                    .all(|c| c.is_ascii_lowercase() || c == '_' || c.is_ascii_digit())
        };

        if segment_ok(resource) && segment_ok(verb) {
            Ok(Self(Cow::Owned(value)))
        } else {
            Err(DomainError::invalid(
                "permission_name",
                format!("{value:?} is not shaped resource.verb"),
            ))
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for PermissionName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// A permission and what it means, for the screen where an administrator
/// decides which role should hold it. The description is the only place that
/// explains `provision.approve` to someone who did not write it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Permission {
    pub name: PermissionName,
    pub description: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Each constant must survive `parse`, or a handler could check for a
    /// permission the seed can never grant.
    #[test]
    fn every_constant_is_a_name_the_parser_accepts() {
        for name in [
            PermissionName::USERS_READ,
            PermissionName::USERS_MANAGE,
            PermissionName::ROLES_READ,
            PermissionName::AUDIT_READ,
        ] {
            assert_eq!(PermissionName::parse(name.as_str()).ok(), Some(name));
        }
    }

    #[test]
    fn accepts_the_seeded_vocabulary() {
        for name in ["users.manage", "provision.approve", "mev.fit", "audit.read"] {
            assert!(PermissionName::parse(name).is_ok(), "rejected {name}");
        }
    }

    #[test]
    fn rejects_names_that_would_silently_match_nothing() {
        assert!(PermissionName::parse("usersmanage").is_err());
        assert!(PermissionName::parse("users.").is_err());
        assert!(PermissionName::parse(".manage").is_err());
        assert!(PermissionName::parse("Users.Manage").is_err());
    }
}
