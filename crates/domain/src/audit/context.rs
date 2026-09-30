use std::net::IpAddr;

use crate::identity::{Email, UserId};

/// Who did it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Actor {
    User {
        id: UserId,
        email: Email,
    },

    /// Nobody authenticated — a failed login has no actor yet. The attempted
    /// address belongs in the entry's metadata, not here: it names an account
    /// that may not exist.
    Anonymous,

    /// The application acting on its own behalf — scheduled jobs, migrations.
    System,
}

impl Actor {
    pub fn id(&self) -> Option<UserId> {
        match self {
            Self::User { id, .. } => Some(*id),
            _ => None,
        }
    }

    /// Denormalised into the row so the record survives the account being
    /// renamed or removed.
    pub fn email(&self) -> Option<&Email> {
        match self {
            Self::User { email, .. } => Some(email),
            _ => None,
        }
    }
}

/// Where it came from.
///
/// Both fields are optional and often absent in development: the address is
/// read from a proxy header, and there is no proxy in front of a local server.
/// An absent value is recorded as absent rather than guessed at.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RequestContext {
    pub ip: Option<IpAddr>,
    pub user_agent: Option<String>,
}

impl RequestContext {
    /// For callers with no request behind them — CLI tools, scheduled work.
    pub fn none() -> Self {
        Self::default()
    }
}
