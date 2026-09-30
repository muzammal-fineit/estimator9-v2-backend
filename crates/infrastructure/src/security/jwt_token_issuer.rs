use chrono::{TimeZone, Utc};
use domain::identity::{
    AccessToken, Email, PermissionName, SessionId, TokenClaims, TokenError, TokenIssuer, UserId,
};
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};

/// HS256 over a shared secret. Adequate while this API is the only thing that
/// verifies its own tokens; a second service would want an asymmetric
/// algorithm so it can verify without being able to mint.
pub struct JwtTokenIssuer {
    encoding: EncodingKey,
    decoding: DecodingKey,
    validation: Validation,
}

impl JwtTokenIssuer {
    pub fn new(secret: &str) -> Self {
        // Pinned to one algorithm. Accepting whatever the token's header asks
        // for is the classic JWT vulnerability — `alg: none`, or an RS256 key
        // presented as an HS256 secret.
        let mut validation = Validation::new(Algorithm::HS256);
        validation.set_required_spec_claims(&["exp", "sub"]);

        // No clock-skew allowance. The default is 60 seconds, which matters
        // when one service signs and another verifies; here the same process
        // does both, so leeway would only extend the life of an expired token.
        validation.leeway = 0;

        Self {
            encoding: EncodingKey::from_secret(secret.as_bytes()),
            decoding: DecodingKey::from_secret(secret.as_bytes()),
            validation,
        }
    }
}

/// The JWT payload. Short names because every byte travels on every request.
#[derive(Debug, Serialize, Deserialize)]
struct Payload {
    /// A string, not a number. RFC 7519 defines `sub` as a StringOrURI, and
    /// `jsonwebtoken` enforces that when it checks required claims — a numeric
    /// `sub` is treated as absent, and every token is rejected before its
    /// expiry is even looked at. Standard client libraries expect a string too.
    sub: String,

    /// Session id. Short name because it rides on every request.
    sid: i64,

    email: String,
    perms: Vec<String>,
    #[serde(default)]
    sa: bool,
    exp: i64,
}

impl TokenIssuer for JwtTokenIssuer {
    fn issue(&self, claims: &TokenClaims) -> Result<AccessToken, TokenError> {
        let payload = Payload {
            sub: claims.user_id.value().to_string(),
            sid: claims.session_id.value(),
            email: claims.email.to_string(),
            perms: claims
                .permissions
                .iter()
                .map(|p| p.as_str().to_owned())
                .collect(),
            sa: claims.is_superadmin,
            exp: claims.expires_at.timestamp(),
        };

        jsonwebtoken::encode(&Header::new(Algorithm::HS256), &payload, &self.encoding)
            .map(AccessToken::new)
            .map_err(TokenError::backend)
    }

    fn verify(&self, raw: &str) -> Result<TokenClaims, TokenError> {
        let data = jsonwebtoken::decode::<Payload>(raw, &self.decoding, &self.validation).map_err(
            |err| match err.kind() {
                jsonwebtoken::errors::ErrorKind::ExpiredSignature => TokenError::Expired,
                // Everything else collapses to one variant: a caller learning
                // *why* their token failed learns how to forge a better one.
                _ => TokenError::Invalid,
            },
        )?;

        let payload = data.claims;

        // A token whose contents no longer parse is not a token we can act on,
        // however good its signature was.
        let user_id = payload
            .sub
            .parse::<i64>()
            .map_err(|_| TokenError::Invalid)?;

        let email = Email::parse(payload.email).map_err(|_| TokenError::Invalid)?;

        let permissions = payload
            .perms
            .into_iter()
            .map(PermissionName::parse)
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| TokenError::Invalid)?;

        let expires_at = Utc
            .timestamp_opt(payload.exp, 0)
            .single()
            .ok_or(TokenError::Invalid)?;

        Ok(TokenClaims {
            user_id: UserId::new(user_id),
            session_id: SessionId::new(payload.sid),
            email,
            permissions,
            is_superadmin: payload.sa,
            expires_at,
        })
    }
}

#[cfg(test)]
#[path = "jwt_token_issuer_tests.rs"]
mod tests;
