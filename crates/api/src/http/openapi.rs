use utoipa::openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme};
use utoipa::{Modify, OpenApi};

/// The document's metadata. Paths and schemas are *not* listed here — they are
/// collected from the `OpenApiRouter` in `routes.rs`, so a route that is not
/// mounted cannot appear in the spec, and a mounted route cannot be missing
/// from it.
#[derive(OpenApi)]
#[openapi(
    info(
        title = "Estimator9 API",
        description = "IFRS 9 provisioning engine — MEV series, model fitting, and \
                       the accounts that drive them.\n\n\
                       Obtain a token from `POST /auth/login` and send it as \
                       `Authorization: Bearer <token>`. Endpoints marked with a padlock \
                       require one; those that also need a permission list it under \
                       their 403 response.\n\n\
                       Tokens are short-lived and carry the caller's permissions, so a \
                       role changed after sign-in takes effect on the next token rather \
                       than the next request.",
        contact(name = "FineIT", email = "akmal@tech.fineit.io"),
    ),
    modifiers(&SecurityAddon),
    tags(
        (name = "auth", description = "Obtaining and using access tokens."),
        (name = "roles", description = "The authorization vocabulary: roles and the permissions they carry. Read-only — reference data changes with a release, not through the API."),
        (name = "identity", description = "User accounts. Roles and permissions arrive with the RBAC slice."),
        (name = "vendor", description = "Vendor-only. Controls which software features this installation may use. Unreachable by any client role, however privileged — these are not the client's decisions to make."),
        (name = "meta", description = "Liveness and build information."),
    ),
)]
pub struct ApiDoc;

/// Declares the bearer scheme. Paths opt in individually with
/// `security(("bearer" = []))`, so the spec marks exactly what is protected —
/// `/health` and `/auth/login` are genuinely open and say so.
struct SecurityAddon;

impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        let components = openapi.components.get_or_insert_with(Default::default);

        components.add_security_scheme(
            "bearer",
            SecurityScheme::Http(
                HttpBuilder::new()
                    .scheme(HttpAuthScheme::Bearer)
                    .bearer_format("JWT")
                    .description(Some(
                        "Issued by POST /auth/login. Expires; refresh by logging in again.",
                    ))
                    .build(),
            ),
        );
    }
}
