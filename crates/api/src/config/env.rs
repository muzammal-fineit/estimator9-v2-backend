//! Typed readers for environment variables.
//!
//! Every reader is a thin wrapper over a pure function that takes the raw
//! `Option<&str>`. The parsing rules are therefore testable without touching
//! the process environment — which in edition 2024 would mean `unsafe`, and
//! would race against every other test in the binary.

use std::str::FromStr;

use super::ConfigError;

/// Required, and treats whitespace as absent: `PORT=` in an env file is a
/// mistake, not a value.
pub fn required(name: &'static str) -> Result<String, ConfigError> {
    match std::env::var(name) {
        Ok(value) if !value.trim().is_empty() => Ok(value),
        _ => Err(ConfigError::Missing { name }),
    }
}

pub fn optional(name: &'static str, default: &str) -> String {
    match std::env::var(name) {
        Ok(value) if !value.trim().is_empty() => value,
        _ => default.to_owned(),
    }
}

/// Parses to `T`, falling back to `default` when unset. A value that is present
/// but unparseable is an error rather than a silent fallback — a typo in
/// `PORT` should stop the process, not quietly serve on 3000.
pub fn parsed<T>(name: &'static str, default: T, reason: &'static str) -> Result<T, ConfigError>
where
    T: FromStr,
{
    parse_or(std::env::var(name).ok().as_deref(), default, name, reason)
}

pub fn flag(name: &'static str, default: bool) -> bool {
    parse_flag(std::env::var(name).ok().as_deref(), default)
}

fn parse_or<T>(
    raw: Option<&str>,
    default: T,
    name: &'static str,
    reason: &'static str,
) -> Result<T, ConfigError>
where
    T: FromStr,
{
    match raw.map(str::trim) {
        None | Some("") => Ok(default),
        Some(value) => value
            .parse()
            .map_err(|_| ConfigError::invalid(name, reason)),
    }
}

/// Anything that is not a recognised false value counts as true, so a typo
/// leaves a feature on rather than silently off.
fn parse_flag(raw: Option<&str>, default: bool) -> bool {
    match raw.map(str::trim) {
        None | Some("") => default,
        Some(value) => !matches!(
            value.to_ascii_lowercase().as_str(),
            "false" | "0" | "no" | "off"
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_and_blank_values_fall_back_to_the_default() {
        assert_eq!(parse_or(None, 3000u16, "PORT", "").unwrap(), 3000);
        assert_eq!(parse_or(Some("  "), 3000u16, "PORT", "").unwrap(), 3000);
        assert!(parse_flag(None, true));
        assert!(!parse_flag(Some(""), false));
    }

    #[test]
    fn a_present_but_unparseable_value_is_an_error_not_a_fallback() {
        assert!(parse_or(Some("eighty"), 3000u16, "PORT", "expected a port").is_err());
        assert!(parse_or(Some("70000"), 3000u16, "PORT", "expected a port").is_err());
    }

    #[test]
    fn only_the_recognised_words_turn_a_flag_off() {
        for off in ["false", "FALSE", "0", "no", "Off"] {
            assert!(!parse_flag(Some(off), true), "{off} should disable");
        }

        for on in ["true", "1", "yes", "tru", "anything"] {
            assert!(parse_flag(Some(on), false), "{on} should enable");
        }
    }

    #[test]
    fn an_error_never_contains_the_offending_value() {
        let err = parse_or(Some("hunter2"), 0u16, "SECRET_PORT", "expected a port").unwrap_err();

        assert!(!err.to_string().contains("hunter2"));
    }
}
