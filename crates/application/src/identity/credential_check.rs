use std::sync::Arc;

use chrono::{DateTime, Utc};
use domain::audit::LoginFailure;
use domain::identity::{LoginControls, Password, PasswordHasher, User, UserRepository};

use super::{Credentials, Rejected};

/// Decides whether a set of credentials is good, and keeps the brute-force
/// counters up to date.
///
/// Separate from [`super::Authenticate`], which decides what to *do* about the
/// answer — start a session, write the trail, shape a response. This type only
/// answers the question, which is what makes the security rules below readable
/// in one screen.
pub struct CredentialCheck {
    users: Arc<dyn UserRepository>,
    hasher: Arc<dyn PasswordHasher>,
}

impl CredentialCheck {
    pub fn new(users: Arc<dyn UserRepository>, hasher: Arc<dyn PasswordHasher>) -> Self {
        Self { users, hasher }
    }

    pub async fn execute(
        &self,
        credentials: &Credentials,
        now: DateTime<Utc>,
    ) -> Result<User, Rejected> {
        let Some(user) = self.users.find_by_email(&credentials.email).await? else {
            // Hash anyway, then discard it. Returning early here would make an
            // unknown address measurably faster to reject than a known one,
            // which is enough to enumerate the user table.
            self.burn_time(&credentials.password).await;

            return Err(LoginFailure::UnknownAccount.into());
        };

        let locked_until = user
            .login_controls()
            .is_locked_at(now)
            .then(|| user.login_controls().locked_until())
            .flatten();

        // The password is verified before the lock is consulted, so a locked
        // account can only be revealed to someone who already knows the
        // password. That costs an Argon2 hash on every attempt against a
        // locked account, affordable only because login is rate limited to a
        // handful of attempts per minute per address.
        let matches = match self
            .hasher
            .verify(&credentials.password, user.password_hash())
            .await
        {
            Ok(matches) => matches,
            // An unusable stored hash is a data problem with one row. The
            // caller sees an ordinary failed login; the trail says otherwise.
            Err(_) => return Err(LoginFailure::CorruptHash.into()),
        };

        if !matches {
            // The counter is not advanced while the account is already locked.
            // Otherwise anyone could hold a known account shut indefinitely by
            // guessing wrongly once every few minutes — turning a brute-force
            // defence into a denial-of-service tool.
            if locked_until.is_none() {
                let controls = user.login_controls().after_failure(now);
                self.users.save_login_controls(user.id(), &controls).await?;
            }

            return Err(LoginFailure::WrongPassword.into());
        }

        // Past this line the caller has proven they know the password, so
        // telling them the account is locked reveals nothing they could not
        // already confirm.
        if let Some(until) = locked_until {
            return Err(Rejected::Locked { until });
        }

        // Still generic: a disabled account must not be distinguishable from a
        // wrong password, because unlike a lockout it is not self-healing and
        // knowing would tell an attacker to move on to a live account.
        if !user.is_active() {
            return Err(LoginFailure::AccountInactive.into());
        }

        self.users
            .save_login_controls(user.id(), &LoginControls::clear())
            .await?;

        Ok(user)
    }

    /// Spends roughly what a real verification spends, so timing does not
    /// betray that the address is unknown.
    async fn burn_time(&self, password: &Password) {
        let _ = self.hasher.hash(password).await;
    }
}
