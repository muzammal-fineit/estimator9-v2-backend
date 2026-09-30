use std::sync::Arc;

use domain::audit::{Actor, AuditAction, AuditEntry, AuditLogger, RequestContext};
use domain::identity::TokenClaims;
use domain::licensing::{Feature, FeatureStore, feature};

use crate::ApplicationError;
use crate::audit_trail::record;

/// Reads and changes which features this installation may use.
///
/// Vendor-only. The permission system is the *client's* policy, so these
/// operations sit outside it entirely and check `is_superadmin` directly —
/// there is no permission a client administrator could be granted to reach
/// them, by design.
pub struct ManageFeatures {
    store: Arc<dyn FeatureStore>,
    audit: Arc<dyn AuditLogger>,
}

impl ManageFeatures {
    pub fn new(store: Arc<dyn FeatureStore>, audit: Arc<dyn AuditLogger>) -> Self {
        Self { store, audit }
    }

    /// The catalogue with each feature's current state.
    ///
    /// The catalogue comes from the binary and the state from the store, so a
    /// feature this release does not know about cannot appear here however the
    /// stored file was written.
    pub async fn list(&self) -> Result<Vec<(Feature, bool)>, ApplicationError> {
        let enabled = self.store.load().await?;

        Ok(feature::CATALOGUE
            .iter()
            .map(|f| (*f, enabled.is_enabled(f.name)))
            .collect())
    }

    pub async fn set(
        &self,
        actor: &TokenClaims,
        name: &str,
        enable: bool,
        context: RequestContext,
    ) -> Result<(), ApplicationError> {
        let Some(target) = feature::lookup(name) else {
            return Err(ApplicationError::not_found("feature", name));
        };

        let mut features = self.store.load().await?;

        // Nothing to do, and nothing to record. An audit entry for a non-event
        // makes the trail harder to read, not easier.
        if features.is_enabled(target.name) == enable {
            return Ok(());
        }

        features.set(target.name, enable);
        self.store.save(&features).await?;

        // Vendor actions are the ones a client's auditor asks about, so this is
        // recorded even though the change never touched their database.
        record(
            self.audit.as_ref(),
            AuditEntry::new(
                if enable {
                    AuditAction::FEATURE_ENABLED
                } else {
                    AuditAction::FEATURE_DISABLED
                },
                Actor::User {
                    id: actor.user_id,
                    email: actor.email.clone(),
                },
                context,
            )
            .during(actor.session_id)
            .via_superadmin(true)
            .about("feature", target.name),
        )
        .await;

        Ok(())
    }
}
