use std::sync::Arc;

use application::identity::{
    AssignRoles, Authenticate, Authorize, ChangePassword, CreateUser, CredentialCheck,
    DeactivateUser, EndSession, GetUser, ListRoles, ListUsers, RefreshSession,
    RequestPasswordReset, ResetPassword, RevokeSession, SessionIssuer, UpdateProfile, UpdateUser,
};
use application::licensing::ManageFeatures;
use domain::audit::AuditLogger;
use domain::identity::{
    PasswordHasher, PasswordResetRepository, RefreshTokenRepository, RoleRepository,
    SessionRepository, TokenIssuer, UserRepository, UserWriteRepository,
};
use domain::licensing::FeatureStore;
use domain::notification::EmailSender;
use infrastructure::PgPool;
use infrastructure::licensing::EncryptedFeatureStore;
use infrastructure::notification::FileEmailSender;
use infrastructure::persistence::audit::PgAuditLogger;
use infrastructure::persistence::identity::{
    PgPasswordResetRepository, PgRefreshTokenRepository, PgRoleRepository, PgSessionRepository,
    PgUserRepository, PgUserWriteRepository,
};
use infrastructure::security::{Argon2Hasher, JwtTokenIssuer};

use crate::config::{CookieConfig, JwtConfig, LicensingConfig, MailConfig};

/// The use cases the HTTP layer can call, already wired to their adapters.
///
/// Handlers reach for a use case here instead of constructing one, which keeps
/// every choice of concrete implementation inside `build`.
#[derive(Clone)]
pub struct AppState {
    pub identity: IdentityUseCases,
    pub licensing: LicensingUseCases,

    /// Behind an `Arc` because `AppState` is cloned per request and the config
    /// is read-only after startup.
    pub cookies: Arc<CookieConfig>,
}

/// Vendor operations. Separate from `identity` because they are governed by a
/// different authority: the permission system is the client's, and these are
/// deliberately outside it.
#[derive(Clone)]
pub struct LicensingUseCases {
    pub features: Arc<ManageFeatures>,
}

#[derive(Clone)]
pub struct IdentityUseCases {
    pub authenticate: Arc<Authenticate>,
    pub authorize: Arc<Authorize>,
    pub assign_roles: Arc<AssignRoles>,
    pub create_user: Arc<CreateUser>,
    pub deactivate_user: Arc<DeactivateUser>,
    pub update_user: Arc<UpdateUser>,
    pub list_roles: Arc<ListRoles>,
    pub change_password: Arc<ChangePassword>,
    pub update_profile: Arc<UpdateProfile>,
    pub request_password_reset: Arc<RequestPasswordReset>,
    pub reset_password: Arc<ResetPassword>,
    pub refresh_session: Arc<RefreshSession>,
    pub revoke_session: Arc<RevokeSession>,
    pub list_users: Arc<ListUsers>,
    pub get_user: Arc<GetUser>,
}

impl AppState {
    pub fn build(
        pool: PgPool,
        jwt: &JwtConfig,
        mail: &MailConfig,
        licensing: &LicensingConfig,
        cookies: Arc<CookieConfig>,
    ) -> Self {
        let users: Arc<dyn UserRepository> = Arc::new(PgUserRepository::new(pool.clone()));
        let roles: Arc<dyn RoleRepository> = Arc::new(PgRoleRepository::new(pool.clone()));
        let roles_for_listing = roles.clone();
        let refresh: Arc<dyn RefreshTokenRepository> =
            Arc::new(PgRefreshTokenRepository::new(pool.clone()));
        let session_store: Arc<dyn SessionRepository> =
            Arc::new(PgSessionRepository::new(pool.clone()));
        let pool_for_writes = pool.clone();
        let pool_for_resets = pool.clone();
        let audit: Arc<dyn AuditLogger> = Arc::new(PgAuditLogger::new(pool));
        let hasher: Arc<dyn PasswordHasher> = Arc::new(Argon2Hasher::new());
        let tokens: Arc<dyn TokenIssuer> = Arc::new(JwtTokenIssuer::new(&jwt.secret));
        let authorize = Arc::new(Authorize::new(tokens.clone(), audit.clone()));
        let user_writes: Arc<dyn UserWriteRepository> =
            Arc::new(PgUserWriteRepository::new(pool_for_writes));
        let resets: Arc<dyn PasswordResetRepository> =
            Arc::new(PgPasswordResetRepository::new(pool_for_resets));
        let mailer: Arc<dyn EmailSender> = Arc::new(FileEmailSender::new(&mail.log_path));
        let features: Arc<dyn FeatureStore> =
            Arc::new(EncryptedFeatureStore::new(&licensing.state_path));
        let hasher_for_admin = hasher.clone();
        let hasher_for_reset = hasher.clone();
        let hasher_for_change = hasher.clone();
        let credentials = Arc::new(CredentialCheck::new(users.clone(), hasher));
        let ending = Arc::new(EndSession::new(refresh.clone(), session_store.clone()));
        let issuer = Arc::new(SessionIssuer::new(
            roles,
            tokens,
            refresh.clone(),
            jwt.access_ttl,
            jwt.refresh_ttl,
        ));

        Self {
            cookies,
            licensing: LicensingUseCases {
                features: Arc::new(ManageFeatures::new(features, audit.clone())),
            },
            identity: IdentityUseCases {
                authenticate: Arc::new(Authenticate::new(
                    credentials,
                    session_store.clone(),
                    issuer.clone(),
                    audit.clone(),
                )),
                authorize,
                assign_roles: Arc::new(AssignRoles::new(users.clone(), user_writes.clone())),
                create_user: Arc::new(CreateUser::new(user_writes.clone(), hasher_for_admin)),
                deactivate_user: Arc::new(DeactivateUser::new(users.clone(), user_writes.clone())),
                update_user: Arc::new(UpdateUser::new(users.clone(), user_writes.clone())),
                list_roles: Arc::new(ListRoles::new(roles_for_listing)),
                change_password: Arc::new(ChangePassword::new(
                    users.clone(),
                    user_writes.clone(),
                    hasher_for_change,
                    session_store.clone(),
                    issuer.clone(),
                )),
                update_profile: Arc::new(UpdateProfile::new(users.clone(), user_writes.clone())),
                request_password_reset: Arc::new(RequestPasswordReset::new(
                    users.clone(),
                    resets.clone(),
                    mailer,
                    audit.clone(),
                    mail.reset_ttl,
                    mail.reset_url.clone(),
                )),
                reset_password: Arc::new(ResetPassword::new(
                    users.clone(),
                    resets,
                    user_writes,
                    hasher_for_reset,
                )),
                refresh_session: Arc::new(RefreshSession::new(
                    users.clone(),
                    refresh.clone(),
                    session_store,
                    ending.clone(),
                    issuer,
                    audit.clone(),
                )),
                revoke_session: Arc::new(RevokeSession::new(users.clone(), refresh, ending, audit)),
                list_users: Arc::new(ListUsers::new(users.clone())),
                get_user: Arc::new(GetUser::new(users)),
            },
        }
    }
}
