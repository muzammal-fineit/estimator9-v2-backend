use chrono::{DateTime, Duration, Utc};

/// Account-level brute-force state.
///
/// Per-IP rate limiting cannot see an attack spread across many addresses at
/// one account; this counter is keyed by the account instead. Both are needed,
/// and neither replaces the other.
///
/// Every method takes `now` rather than reading the clock, so the rules are
/// testable without waiting and without a clock port.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoginControls {
    failed_attempts: u32,
    locked_until: Option<DateTime<Utc>>,
}

impl LoginControls {
    pub const MAX_ATTEMPTS: u32 = 5;
    const LOCKOUT_MINUTES: i64 = 15;

    pub fn rehydrate(failed_attempts: u32, locked_until: Option<DateTime<Utc>>) -> Self {
        Self {
            failed_attempts,
            locked_until,
        }
    }

    /// A clean slate — a new account, or one that has just authenticated.
    pub fn clear() -> Self {
        Self {
            failed_attempts: 0,
            locked_until: None,
        }
    }

    /// The lock expires on its own. Nothing has to unlock the account, which
    /// means a stuck record cannot leave someone permanently shut out.
    pub fn is_locked_at(&self, now: DateTime<Utc>) -> bool {
        self.locked_until.is_some_and(|until| until > now)
    }

    /// The state after a rejected password.
    ///
    /// The counter is not reset when the lock expires: attempt six after a
    /// lapsed lock locks the account again immediately. Otherwise waiting out
    /// the window would buy five fresh guesses every fifteen minutes forever.
    pub fn after_failure(&self, now: DateTime<Utc>) -> Self {
        let failed_attempts = self.failed_attempts.saturating_add(1);

        let locked_until = if failed_attempts >= Self::MAX_ATTEMPTS {
            Some(now + Duration::minutes(Self::LOCKOUT_MINUTES))
        } else {
            self.locked_until
        };

        Self {
            failed_attempts,
            locked_until,
        }
    }

    pub fn failed_attempts(&self) -> u32 {
        self.failed_attempts
    }

    pub fn locked_until(&self) -> Option<DateTime<Utc>> {
        self.locked_until
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn now() -> DateTime<Utc> {
        DateTime::from_timestamp(1_700_000_000, 0).unwrap_or_default()
    }

    #[test]
    fn a_fresh_account_is_not_locked() {
        assert!(!LoginControls::clear().is_locked_at(now()));
    }

    #[test]
    fn locks_only_once_the_threshold_is_reached() {
        let mut controls = LoginControls::clear();

        for _ in 1..LoginControls::MAX_ATTEMPTS {
            controls = controls.after_failure(now());
            assert!(!controls.is_locked_at(now()), "locked too early");
        }

        controls = controls.after_failure(now());
        assert!(controls.is_locked_at(now()));
    }

    #[test]
    fn the_lock_lapses_without_anything_unlocking_it() {
        let locked = LoginControls::rehydrate(5, Some(now()));

        assert!(!locked.is_locked_at(now() + Duration::seconds(1)));
    }

    #[test]
    fn one_failure_after_a_lapsed_lock_locks_again_immediately() {
        let lapsed = LoginControls::rehydrate(LoginControls::MAX_ATTEMPTS, Some(now()));
        let later = now() + Duration::hours(1);

        assert!(!lapsed.is_locked_at(later));
        assert!(lapsed.after_failure(later).is_locked_at(later));
    }

    #[test]
    fn a_successful_login_clears_the_counter() {
        assert_eq!(LoginControls::clear().failed_attempts(), 0);
        assert!(LoginControls::clear().locked_until().is_none());
    }
}
