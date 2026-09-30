use async_trait::async_trait;

use super::{Email, PasswordHash, PasswordResetId, RoleName, UserId};
use crate::audit::AuditEntry;
use crate::shared::RepositoryError;

/// Writes that change a user, each recording its own audit entry **in the same
/// transaction as the change**.
///
/// Named for what it does, not who may call it: an administrator changing
/// someone else's roles and a user changing their own password both land here,
/// and the rules about which is allowed live in the use cases.
///
/// Why the entry is a parameter rather than a separate call: a role grant that
/// exists with no record of who made it is the failure this design is meant to
/// rule out. Passing the entry in makes the two writes one operation that
/// either lands or does not, and puts the transaction in the adapter — which
/// is the layer whose job transactions are.
///
/// The alternative, a unit-of-work handing transaction-bound repositories up
/// to the use case, buys generality this codebase has no use for yet and costs
/// a great deal of lifetime plumbing in return. If a future operation needs to
/// span several aggregates, that is the point to reach for one.
#[async_trait]
pub trait UserWriteRepository: Send + Sync + 'static {
    /// Creates an account with exactly `roles`.
    ///
    /// A duplicate address is [`RepositoryError::AlreadyExists`] — the
    /// `users_email_lower_key` index decides, so two requests racing for the
    /// same address cannot both win.
    async fn create(
        &self,
        email: &Email,
        name: &str,
        password: &PasswordHash,
        roles: &[RoleName],
        entry: AuditEntry,
    ) -> Result<UserId, RepositoryError>;

    /// Changes a user's name and/or address. `None` leaves a field alone.
    ///
    /// A duplicate address is [`RepositoryError::AlreadyExists`], decided by
    /// the same index that governs creation.
    async fn update(
        &self,
        user: UserId,
        name: Option<&str>,
        email: Option<&Email>,
        entry: AuditEntry,
    ) -> Result<(), RepositoryError>;

    /// Replaces the password and ends every session the account has —
    /// including the one making the request.
    ///
    /// All of it in one transaction: a password changed because it may be
    /// known to someone else is worthless if the sessions opened with the old
    /// one keep working. The caller issues a fresh session afterwards, so the
    /// person who asked stays signed in and everyone else does not.
    async fn change_password(
        &self,
        user: UserId,
        password: &PasswordHash,
        entry: AuditEntry,
    ) -> Result<(), RepositoryError>;

    /// Redeems a reset token and sets the password, ending every session.
    ///
    /// Consuming the token is guarded so two requests racing with the same
    /// link cannot both succeed.
    async fn reset_password(
        &self,
        user: UserId,
        token: PasswordResetId,
        password: &PasswordHash,
        entry: AuditEntry,
    ) -> Result<(), RepositoryError>;

    /// Deactivates an account and ends everything it can still do: live
    /// sessions closed, refresh tokens revoked, all in the same transaction as
    /// the flag itself.
    ///
    /// Access tokens already issued remain valid until they expire — that is
    /// the bound the short TTL exists to set, and no write here can shorten it.
    async fn deactivate(&self, user: UserId, entry: AuditEntry) -> Result<(), RepositoryError>;

    /// Replaces the user's roles with exactly `roles`.
    ///
    /// Unknown role names are a [`RepositoryError::NotFound`] and change
    /// nothing — a partial application would leave the user with whichever
    /// roles happened to resolve first.
    async fn set_roles(
        &self,
        user: UserId,
        roles: &[RoleName],
        entry: AuditEntry,
    ) -> Result<(), RepositoryError>;
}
