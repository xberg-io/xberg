//! Optional bearer-token authentication for the REST API.
//!
//! The token comes from `XBERG_API_TOKEN`. With a token set, every route except `GET /health`
//! requires `Authorization: Bearer <token>`. Without one the API is open, which is only
//! acceptable when it cannot be reached from another host: binding a non-loopback address
//! without a token is refused at startup unless `XBERG_ALLOW_UNAUTHENTICATED` is truthy.

use std::net::IpAddr;
use std::sync::Arc;

use axum::{
    Json,
    extract::{Request, State},
    http::{HeaderValue, StatusCode, header},
    middleware::Next,
    response::{IntoResponse, Response},
};

use super::types::ErrorResponse;
use crate::{Result, XbergError};

/// Environment variable holding the bearer token that protects the API.
pub(crate) const API_TOKEN_ENV: &str = "XBERG_API_TOKEN";

/// Environment variable that lets the server start on a non-loopback address without a token.
pub(crate) const ALLOW_UNAUTHENTICATED_ENV: &str = "XBERG_ALLOW_UNAUTHENTICATED";

/// Authentication settings resolved from the environment.
#[derive(Clone, Default)]
pub(crate) struct ApiAuth {
    token: Option<Arc<str>>,
    allow_unauthenticated: bool,
}

impl std::fmt::Debug for ApiAuth {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ApiAuth")
            .field("token", &self.token.as_ref().map(|_| "<redacted>"))
            .field("allow_unauthenticated", &self.allow_unauthenticated)
            .finish()
    }
}

impl ApiAuth {
    pub(crate) fn new(token: Option<&str>, allow_unauthenticated: bool) -> Self {
        Self {
            token: token.map(str::trim).filter(|t| !t.is_empty()).map(Arc::from),
            allow_unauthenticated,
        }
    }

    pub(crate) fn from_env() -> Self {
        let allow = std::env::var(ALLOW_UNAUTHENTICATED_ENV)
            .map(|v| matches!(v.trim().to_ascii_lowercase().as_str(), "1" | "true" | "yes" | "on"))
            .unwrap_or(false);
        Self::new(std::env::var(API_TOKEN_ENV).ok().as_deref(), allow)
    }

    pub(crate) fn token_configured(&self) -> bool {
        self.token.is_some()
    }

    pub(crate) fn expected_token(&self) -> Option<Arc<str>> {
        self.token.clone()
    }

    /// Refuse an unauthenticated server that is reachable from other hosts.
    pub(crate) fn check_bind(&self, ip: IpAddr) -> Result<()> {
        if self.token.is_some() || ip.is_loopback() {
            return Ok(());
        }
        if self.allow_unauthenticated {
            tracing::warn!(
                "API bound to {ip} without authentication ({ALLOW_UNAUTHENTICATED_ENV} is set); \
                 anyone who can reach this port can run extractions"
            );
            return Ok(());
        }
        Err(XbergError::validation(format!(
            "Refusing to bind {ip} without authentication. Set {API_TOKEN_ENV} to a secret bearer token, \
             or set {ALLOW_UNAUTHENTICATED_ENV}=true to run an open API (for example behind a trusted \
             reverse proxy or on a private network)."
        )))
    }
}

/// Compare two byte strings without short-circuiting on the first difference or on length.
fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    let mut diff = a.len() ^ b.len();
    for i in 0..a.len().max(b.len()) {
        let x = a.get(i).copied().unwrap_or(0);
        let y = b.get(i).copied().unwrap_or(0);
        diff |= usize::from(x ^ y);
    }
    std::hint::black_box(diff) == 0
}

fn bearer_token(request: &Request) -> Option<&str> {
    let value = request.headers().get(header::AUTHORIZATION)?.to_str().ok()?;
    let (scheme, token) = value.split_once(' ')?;
    scheme.eq_ignore_ascii_case("bearer").then_some(token.trim())
}

fn unauthorized() -> Response {
    let body = ErrorResponse {
        error_type: "Unauthorized".to_string(),
        message: "Missing or invalid bearer token".to_string(),
        traceback: None,
        status_code: StatusCode::UNAUTHORIZED.as_u16(),
    };
    let mut response = (StatusCode::UNAUTHORIZED, Json(body)).into_response();
    response
        .headers_mut()
        .insert(header::WWW_AUTHENTICATE, HeaderValue::from_static("Bearer"));
    response
}

/// Middleware rejecting requests that do not carry the configured bearer token.
///
/// CORS preflight (`OPTIONS`) is answered by the CORS layer before this runs.
pub(crate) async fn require_bearer_token(State(expected): State<Arc<str>>, request: Request, next: Next) -> Response {
    match bearer_token(&request) {
        Some(provided) if constant_time_eq(provided.as_bytes(), expected.as_bytes()) => next.run(request).await,
        _ => unauthorized(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constant_time_eq_matches_only_identical_bytes() {
        assert!(constant_time_eq(b"secret", b"secret"));
        assert!(!constant_time_eq(b"secret", b"secreT"));
        assert!(!constant_time_eq(b"secret", b"secre"));
        assert!(!constant_time_eq(b"", b"secret"));
        assert!(constant_time_eq(b"", b""));
    }

    #[test]
    fn blank_token_counts_as_unset() {
        assert!(!ApiAuth::new(Some("   "), false).token_configured());
        assert!(ApiAuth::new(Some("abc"), false).token_configured());
    }

    #[test]
    fn check_bind_allows_loopback_without_token() {
        let auth = ApiAuth::new(None, false);
        assert!(auth.check_bind("127.0.0.1".parse().unwrap()).is_ok());
        assert!(auth.check_bind("::1".parse().unwrap()).is_ok());
    }

    #[test]
    fn check_bind_refuses_off_loopback_without_token() {
        let err = ApiAuth::new(None, false)
            .check_bind("0.0.0.0".parse().unwrap())
            .unwrap_err()
            .to_string();
        assert!(
            err.contains(API_TOKEN_ENV) && err.contains(ALLOW_UNAUTHENTICATED_ENV),
            "{err}"
        );
    }

    #[test]
    fn check_bind_allows_off_loopback_with_token_or_opt_out() {
        let ip: IpAddr = "0.0.0.0".parse().unwrap();
        assert!(ApiAuth::new(Some("t"), false).check_bind(ip).is_ok());
        assert!(ApiAuth::new(None, true).check_bind(ip).is_ok());
    }

    #[test]
    fn debug_output_redacts_the_token() {
        let rendered = format!("{:?}", ApiAuth::new(Some("hunter2"), false));
        assert!(!rendered.contains("hunter2"));
    }
}
