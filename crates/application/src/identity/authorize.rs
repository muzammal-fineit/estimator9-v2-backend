use std::sync::Arc;

use domain::audit::{Actor, AuditAction, AuditEntry, AuditLogger, RequestContext};
use domain::identity::{PermissionName, TokenClaims, TokenIssuer};

use crate::ApplicationError;
use crate::audit_trail::record;

/// Turns a bearer token into verified claims, and decides whether those claims
/// permit an operation.
///
/// Both halves live here rather than in the HTTP layer so the rules are
/// testable without a request, and so the denial is audited wherever the check
/// happens — a guard that can be bypassed by forgetting to log is not a guard.
pub struct Authorize {
    tokens: Arc<dyn TokenIssuer>,
    audit: Arc<dyn AuditLogger>,
}

impl Authorize {
    pub fn new(tokens: Arc<dyn TokenIssuer>, audit: Arc<dyn AuditLogger>) -> Self {
        Self { tokens, audit }
    }

    /// Verifies a token. Expired and forged tokens are the same answer to the
    /// caller — knowing *why* a token was rejected tells a forger how close
    /// they got.
    pub fn verify(&self, raw: &str) -> Result<TokenClaims, ApplicationError> {
        self.tokens
            .verify(raw)
            .map_err(|_| ApplicationError::Unauthenticated)
    }

    /// Requires the vendor account specifically.
    ///
    /// Not expressible as a permission: permissions are the client's policy,
    /// and the whole point of these operations is that no client role can
    /// reach them. A denial is audited like any other.
    pub async fn require_superadmin(
        &self,
        claims: &TokenClaims,
        context: RequestContext,
    ) -> Result<(), ApplicationError> {
        if claims.is_superadmin {
            return Ok(());
        }

        record(
            self.audit.as_ref(),
            AuditEntry::new(
                AuditAction::AUTHZ_DENIED,
                Actor::User {
                    id: claims.user_id,
                    email: claims.email.clone(),
                },
                context,
            )
            .during(claims.session_id)
            .with("required_permission", "superadmin")
            .with("held_permissions", held(claims)),
        )
        .await;

        Err(ApplicationError::Forbidden {
            permission: "this operation is restricted to the vendor".to_owned(),
        })
    }

    /// Checks a permission, recording a denial.
    ///
    /// A superadmin is allowed without the permission set being consulted at
    /// all — see [`TokenClaims::allows`] — and the resulting audit entry is
    /// marked so an auditor can find every action taken by bypass.
    pub async fn require(
        &self,
        claims: &TokenClaims,
        permission: PermissionName,
        context: RequestContext,
    ) -> Result<(), ApplicationError> {
        if claims.allows(&permission) {
            return Ok(());
        }

        let entry = AuditEntry::new(
            AuditAction::AUTHZ_DENIED,
            Actor::User {
                id: claims.user_id,
                email: claims.email.clone(),
            },
            context,
        )
        .during(claims.session_id)
        .with("required_permission", permission.as_str())
        .with("held_permissions", held(claims));

        record(self.audit.as_ref(), entry).await;

        Err(ApplicationError::Forbidden {
            permission: permission.as_str().to_owned(),
        })
    }
}

/// What the caller actually held, so a denial can be diagnosed without
/// reconstructing their token.
fn held(claims: &TokenClaims) -> String {
    if claims.permissions.is_empty() {
        return "none".to_owned();
    }

    claims
        .permissions
        .iter()
        .map(PermissionName::as_str)
        .collect::<Vec<_>>()
        .join(",")
}
