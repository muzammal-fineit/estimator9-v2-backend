/// What happened, from a closed vocabulary.
///
/// Deliberately wraps `&'static str` rather than `String`: every action is
/// known at compile time, so callers can only use a constant defined here. A
/// typo becomes a compile error instead of an audit row that no query will ever
/// match, and the whole vocabulary is readable in one screen — which is the
/// first thing an auditor asks for.
///
/// Naming is `context.subject.verb`, past tense.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AuditAction(&'static str);

impl AuditAction {
    // Authentication.
    pub const LOGIN_SUCCEEDED: Self = Self("auth.login.succeeded");
    pub const LOGIN_FAILED: Self = Self("auth.login.failed");

    pub const TOKEN_REFRESHED: Self = Self("auth.token.refreshed");
    pub const LOGOUT: Self = Self("auth.logout");
    pub const PASSWORD_CHANGED: Self = Self("auth.password.changed");
    pub const PASSWORD_RESET_REQUESTED: Self = Self("auth.password.reset_requested");
    pub const PASSWORD_RESET_COMPLETED: Self = Self("auth.password.reset_completed");

    /// An already-exchanged refresh token was presented again. Either a stolen
    /// token is being replayed or the rightful holder arrived after a thief —
    /// indistinguishable, and both mean the chain is compromised. The single
    /// most alert-worthy row in the table.
    pub const REFRESH_REUSED: Self = Self("auth.refresh.reused");

    /// User administration.
    pub const USER_CREATED: Self = Self("user.created");
    pub const USER_DEACTIVATED: Self = Self("user.deactivated");
    pub const USER_UPDATED: Self = Self("user.updated");
    pub const USER_ROLES_ASSIGNED: Self = Self("user.roles.assigned");

    /// Vendor actions. Always `via_superadmin`, and the rows a client's
    /// auditor will ask about first.
    pub const FEATURE_ENABLED: Self = Self("vendor.feature.enabled");
    pub const FEATURE_DISABLED: Self = Self("vendor.feature.disabled");

    /// A caller held a valid token but not the permission the endpoint needed.
    /// Worth recording: a run of these is someone probing what they can reach.
    pub const AUTHZ_DENIED: Self = Self("auth.authz.denied");

    pub fn as_str(self) -> &'static str {
        self.0
    }
}

impl std::fmt::Display for AuditAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0)
    }
}

/// Why a login was refused.
///
/// Recorded in the audit trail and never returned to the caller — the whole
/// point of the single `InvalidCredentials` response is that these are
/// indistinguishable from outside. Support needs the distinction; an attacker
/// must not have it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoginFailure {
    UnknownAccount,
    WrongPassword,
    AccountLocked,
    AccountInactive,
    /// The stored hash could not be used — that row needs a password reset.
    CorruptHash,
}

impl LoginFailure {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::UnknownAccount => "unknown_account",
            Self::WrongPassword => "wrong_password",
            Self::AccountLocked => "account_locked",
            Self::AccountInactive => "account_inactive",
            Self::CorruptHash => "corrupt_hash",
        }
    }
}
