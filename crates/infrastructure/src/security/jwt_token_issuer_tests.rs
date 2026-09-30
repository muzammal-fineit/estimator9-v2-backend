//! Tests for [`super::JwtTokenIssuer`], in their own file so the adapter stays
//! under the 150-line limit.

use chrono::Duration;

use super::*;

fn issuer() -> JwtTokenIssuer {
    JwtTokenIssuer::new("a-test-secret-that-is-long-enough")
}

fn claims(expires_at: chrono::DateTime<Utc>) -> TokenClaims {
    TokenClaims {
        user_id: UserId::new(7),
        session_id: domain::identity::SessionId::new(42),
        email: Email::parse("analyst@fineit.io").unwrap(),
        permissions: vec![PermissionName::parse("mev.fit").unwrap()],
        is_superadmin: false,
        expires_at,
    }
}

#[test]
fn a_token_round_trips_through_issue_and_verify() {
    let issuer = issuer();
    let original = claims(Utc::now() + Duration::minutes(15));

    let token = issuer.issue(&original).unwrap();
    let verified = issuer.verify(token.as_str()).unwrap();

    assert_eq!(verified.user_id, original.user_id);
    assert_eq!(verified.session_id, original.session_id);
    assert_eq!(verified.email, original.email);
    assert_eq!(verified.permissions, original.permissions);
    assert!(!verified.is_superadmin);
}

#[test]
fn an_expired_token_is_rejected_even_though_its_signature_is_good() {
    let issuer = issuer();
    let token = issuer
        .issue(&claims(Utc::now() - Duration::seconds(1)))
        .unwrap();

    assert!(matches!(
        issuer.verify(token.as_str()),
        Err(TokenError::Expired)
    ));
}

#[test]
fn a_token_signed_with_another_secret_is_rejected() {
    let token = JwtTokenIssuer::new("the-attackers-own-secret")
        .issue(&claims(Utc::now() + Duration::minutes(15)))
        .unwrap();

    assert!(matches!(
        issuer().verify(token.as_str()),
        Err(TokenError::Invalid)
    ));
}

/// The classic JWT attack: strip the signature and set `alg` to `none`. The
/// issuer pins HS256, so the header is never trusted to choose the algorithm.
#[test]
fn an_unsigned_token_is_rejected() {
    // {"alg":"none","typ":"JWT"} . {"sub":7,...} . <empty signature>
    let forged = "eyJhbGciOiJub25lIiwidHlwIjoiSldUIn0\
                  .eyJzdWIiOjcsImVtYWlsIjoiYW5hbHlzdEBmaW5laXQuaW8iLFwicGVybXNcIjpbXSwiZXhwIjo5OTk5OTk5OTk5fQ\
                  .";

    assert!(matches!(issuer().verify(forged), Err(TokenError::Invalid)));
}

#[test]
fn rubbish_is_rejected_without_panicking() {
    for raw in ["", "not-a-token", "a.b.c", "....."] {
        assert!(
            matches!(issuer().verify(raw), Err(TokenError::Invalid)),
            "{raw:?} should be rejected"
        );
    }
}

#[test]
fn the_superadmin_flag_survives_the_round_trip() {
    let issuer = issuer();
    let mut original = claims(Utc::now() + Duration::minutes(15));
    original.is_superadmin = true;
    original.permissions.clear();

    let token = issuer.issue(&original).unwrap();
    let verified = issuer.verify(token.as_str()).unwrap();

    assert!(verified.is_superadmin);
    assert!(verified.permissions.is_empty());
    assert!(verified.allows(&PermissionName::parse("anything.at_all").unwrap()));
}
