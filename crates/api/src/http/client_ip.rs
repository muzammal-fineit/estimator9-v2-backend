use std::net::{IpAddr, SocketAddr};

use axum::extract::ConnectInfo;
use axum::http::{Extensions, HeaderMap};

/// The caller's address, from the only two sources worth trusting.
///
/// **`X-Real-IP` first.** The nginx config sets it to `$remote_addr`, which
/// *overwrites* anything the caller sent, so behind the proxy it is the real
/// peer.
///
/// **Peer address second**, for direct connections — local development, and a
/// health probe on the loopback.
///
/// **`X-Forwarded-For` is deliberately not used.** nginx sets it with
/// `$proxy_add_x_forwarded_for`, which *appends* to the caller's value rather
/// than replacing it, so its first entry is whatever the caller chose. Keying a
/// rate limiter on that would let anyone mint a fresh bucket per request by
/// varying one header, which is worse than having no limiter at all — it looks
/// like protection and is not.
///
/// The whole scheme rests on the API being reachable only through the proxy.
/// In production the server must bind loopback, not `0.0.0.0`.
pub fn trusted_client_ip(headers: &HeaderMap, extensions: &Extensions) -> Option<IpAddr> {
    let from_proxy = headers
        .get("x-real-ip")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.trim().parse().ok());

    from_proxy.or_else(|| {
        extensions
            .get::<ConnectInfo<SocketAddr>>()
            .map(|ConnectInfo(address)| address.ip())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn headers(pairs: &[(&str, &str)]) -> HeaderMap {
        let mut map = HeaderMap::new();
        for (name, value) in pairs {
            if let (Ok(n), Ok(v)) = (
                name.parse::<axum::http::HeaderName>(),
                value.parse::<axum::http::HeaderValue>(),
            ) {
                map.insert(n, v);
            }
        }
        map
    }

    fn with_peer(ip: &str) -> Extensions {
        let mut ext = Extensions::new();
        if let Ok(address) = format!("{ip}:54321").parse::<SocketAddr>() {
            ext.insert(ConnectInfo(address));
        }
        ext
    }

    #[test]
    fn prefers_the_proxy_header_over_the_peer_address() {
        let ip = trusted_client_ip(
            &headers(&[("x-real-ip", "203.0.113.7")]),
            &with_peer("10.0.0.1"),
        );

        assert_eq!(ip, "203.0.113.7".parse().ok());
    }

    #[test]
    fn falls_back_to_the_peer_address_when_there_is_no_proxy() {
        let ip = trusted_client_ip(&headers(&[]), &with_peer("10.0.0.1"));

        assert_eq!(ip, "10.0.0.1".parse().ok());
    }

    /// The bypass this function exists to prevent: a caller varying
    /// `X-Forwarded-For` must not move between rate-limit buckets.
    #[test]
    fn ignores_x_forwarded_for_entirely() {
        let spoofed = headers(&[("x-forwarded-for", "198.51.100.99, 10.0.0.1")]);

        assert_eq!(
            trusted_client_ip(&spoofed, &with_peer("10.0.0.1")),
            "10.0.0.1".parse().ok(),
            "X-Forwarded-For must not influence the key"
        );
    }

    #[test]
    fn unparseable_or_absent_means_unknown() {
        assert_eq!(trusted_client_ip(&headers(&[]), &Extensions::new()), None);
        assert_eq!(
            trusted_client_ip(&headers(&[("x-real-ip", "not-an-ip")]), &Extensions::new()),
            None
        );
    }
}
