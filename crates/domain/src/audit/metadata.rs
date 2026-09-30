use std::collections::BTreeMap;

/// Extra detail for an audit entry, as a flat string map.
///
/// Flat and stringly-typed on purpose: the adapter stores it as `jsonb`, and a
/// flat map stays queryable in Postgres without the domain growing a
/// dependency on a serialisation crate. `BTreeMap` keeps key order stable, so
/// two equivalent entries serialise identically.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AuditMetadata(BTreeMap<String, String>);

impl AuditMetadata {
    /// Key fragments that mean the value must never be stored, whatever the
    /// caller intended. The audit trail is the one log guaranteed to be kept
    /// forever, which makes it the worst possible place for a credential.
    const FORBIDDEN: [&'static str; 6] = [
        "password",
        "secret",
        "token",
        "hash",
        "credential",
        "authorization",
    ];

    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a field, redacting the value if the key suggests it is sensitive.
    ///
    /// Redacting rather than rejecting is deliberate: a dropped field leaves no
    /// trace of the mistake, while `<redacted>` in a row tells whoever reads it
    /// that something was attempted and stopped.
    pub fn insert(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        let key = key.into();
        let lowered = key.to_ascii_lowercase();

        let value = if Self::FORBIDDEN.iter().any(|f| lowered.contains(f)) {
            "<redacted>".to_owned()
        } else {
            value.into()
        };

        self.0.insert(key, value);
        self
    }

    pub fn iter(&self) -> impl Iterator<Item = (&String, &String)> {
        self.0.iter()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_ordinary_fields() {
        let meta = AuditMetadata::new()
            .insert("reason", "wrong_password")
            .insert("attempted_email", "someone@example.com");

        let pairs: Vec<_> = meta.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();

        assert_eq!(
            pairs,
            [
                ("attempted_email", "someone@example.com"),
                ("reason", "wrong_password"),
            ]
        );
    }

    #[test]
    fn redacts_anything_whose_key_looks_like_a_credential() {
        let meta = AuditMetadata::new()
            .insert("password", "hunter2")
            .insert("New_Password", "hunter2")
            .insert("access_token", "eyJhbGciOi")
            .insert("password_hash", "$argon2id$v=19$")
            .insert("Authorization", "Bearer eyJ");

        for (key, value) in meta.iter() {
            assert_eq!(value, "<redacted>", "{key} leaked its value");
        }
    }

    #[test]
    fn no_credential_material_survives_into_the_rendered_map() {
        let meta = AuditMetadata::new()
            .insert("reason", "wrong_password")
            .insert("password", "hunter2");

        let rendered: String = meta.iter().map(|(k, v)| format!("{k}={v};")).collect();

        assert!(!rendered.contains("hunter2"));
        assert!(rendered.contains("reason=wrong_password"));
    }
}
