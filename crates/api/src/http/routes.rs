use std::time::Duration;

use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::{Json, Router};
use tower_http::trace::TraceLayer;
// The trait is only needed for `ApiDoc::openapi()`; `OpenApi` below is the
// document struct, which shares its name.
use utoipa::OpenApi as _;
use utoipa::openapi::OpenApi;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;
use utoipa_swagger_ui::SwaggerUi;

use super::cors;
use super::dto::error::ErrorResponse;
use super::handlers::{auth, health, identity, roles, vendor};
use super::openapi::ApiDoc;
use super::rate_limit;
use crate::config::CorsConfig;
use crate::config::RateLimitConfig;
use crate::state::AppState;

pub const SWAGGER_UI_PATH: &str = "/swagger-ui";
pub const OPENAPI_JSON_PATH: &str = "/api-docs/openapi.json";

/// Build the application.
///
/// `enable_docs` controls only whether Swagger UI and the spec endpoint are
/// mounted; the API itself is identical either way.
pub fn router(
    state: AppState,
    enable_docs: bool,
    limits: &RateLimitConfig,
    origins: &CorsConfig,
) -> Router {
    let (router, openapi) = api_router(limits).split_for_parts();

    let mut app = router.with_state(state);

    if enable_docs {
        app = app.merge(SwaggerUi::new(SWAGGER_UI_PATH).url(OPENAPI_JSON_PATH, openapi));
    }

    // CORS outermost, so a preflight and an error response both carry the
    // headers a browser needs to show the real failure rather than a generic
    // "blocked by CORS".
    app.fallback(not_found)
        .layer(TraceLayer::new_for_http())
        .layer(cors::layer(origins))
}

/// Axum's default for an unmatched route is an empty 404 body. A client that
/// parses `{"error": ...}` everywhere else should not have to special-case the
/// one response with nothing in it.
async fn not_found() -> impl IntoResponse {
    (
        StatusCode::NOT_FOUND,
        Json(ErrorResponse::new("no such endpoint")),
    )
}

/// The spec on its own, for the `openapi` binary. Builds the same router and
/// throws it away, which is what makes the dumped file and the served routes
/// impossible to disagree.
pub fn openapi() -> OpenApi {
    api_router(&RateLimitConfig::disabled()).split_for_parts().1
}

/// The single registration of every route. `routes!` reads the method and path
/// from the handler's `#[utoipa::path]`, so the two cannot drift apart.
///
/// `/health` carries no limiter: throttling a monitoring probe turns a busy
/// minute into a false alarm.
fn api_router(limits: &RateLimitConfig) -> OpenApiRouter<AppState> {
    OpenApiRouter::with_openapi(ApiDoc::openapi())
        .routes(routes!(health::health))
        .merge(login_routes(limits))
        .merge(password_reset_routes(limits))
        .merge(session_routes(limits))
        .merge(identity_routes(limits))
        .merge(vendor_routes(limits))
}

/// Login alone, on the tightest quota. It is the one endpoint where repeated
/// guessing is worth an attacker's time, and the Argon2 cost per attempt makes
/// it the most expensive thing an anonymous caller can ask for.
fn login_routes(limits: &RateLimitConfig) -> OpenApiRouter<AppState> {
    let routes = OpenApiRouter::new().routes(routes!(auth::login));

    throttled(routes, limits, limits.login_per_minute, MINUTE)
}

/// Per hour, not per minute: these send mail to whatever address a caller
/// names, so what matters is how many messages one address can cause in a day.
fn password_reset_routes(limits: &RateLimitConfig) -> OpenApiRouter<AppState> {
    let routes = OpenApiRouter::new()
        .routes(routes!(auth::forgot_password))
        .routes(routes!(auth::reset_password));

    throttled(routes, limits, limits.password_reset_per_hour, HOUR)
}

/// Everything that already requires a token, on the ordinary quota. `/auth/me`
/// in particular is called on app start and after every navigation, so a
/// login-sized limit here would throttle normal use.
fn session_routes(limits: &RateLimitConfig) -> OpenApiRouter<AppState> {
    let routes = OpenApiRouter::new()
        .routes(routes!(auth::refresh))
        .routes(routes!(auth::logout))
        .routes(routes!(auth::me))
        .routes(routes!(auth::update_profile))
        .routes(routes!(auth::change_password));

    throttled(routes, limits, limits.default_per_minute, MINUTE)
}

/// The vendor panel. Ordinary quota — it is reachable by exactly one account,
/// so brute force is not the risk here.
fn vendor_routes(limits: &RateLimitConfig) -> OpenApiRouter<AppState> {
    let routes = OpenApiRouter::new()
        .routes(routes!(vendor::list_features))
        .routes(routes!(vendor::set_feature));

    throttled(routes, limits, limits.default_per_minute, MINUTE)
}

const MINUTE: Duration = Duration::from_secs(60);
const HOUR: Duration = Duration::from_secs(3600);

fn throttled(
    routes: OpenApiRouter<AppState>,
    limits: &RateLimitConfig,
    burst: u32,
    window: Duration,
) -> OpenApiRouter<AppState> {
    if limits.enabled {
        rate_limit::throttle(routes, burst, window)
    } else {
        routes
    }
}

/// One function per bounded context. New contexts get their own `merge` above.
fn identity_routes(limits: &RateLimitConfig) -> OpenApiRouter<AppState> {
    let routes = OpenApiRouter::new()
        .routes(routes!(identity::list_users))
        .routes(routes!(identity::get_user))
        .routes(routes!(identity::assign_roles))
        .routes(routes!(identity::create_user))
        .routes(routes!(identity::deactivate_user))
        .routes(routes!(identity::update_user))
        .routes(routes!(roles::list_roles))
        .routes(routes!(roles::list_permissions));

    throttled(routes, limits, limits.default_per_minute, MINUTE)
}
