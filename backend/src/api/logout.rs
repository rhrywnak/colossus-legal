//! Logout handler — clears Authentik proxy cookies and redirects to
//! the invalidation flow.
//!
//! The Authentik outpost sets an HttpOnly cookie (authentik_proxy_*)
//! on the .cogmai.com domain.  JavaScript cannot delete HttpOnly
//! cookies, so we need a server-side endpoint to expire them.
//!
//! This works around authentik bug #17922 where the outpost's
//! /outpost.goauthentik.io/sign_out endpoint panics.

use axum::{
    http::{
        header::{COOKIE, SET_COOKIE},
        HeaderMap, StatusCode,
    },
    response::IntoResponse,
};

/// POST /api/logout
///
/// 1. Reads all cookies from the request
/// 2. Expires any `authentik_proxy_*` cookies on `.cogmai.com`
/// 3. Returns 200 with expired cookie headers (frontend handles redirect)
pub async fn logout_handler(headers: HeaderMap) -> impl IntoResponse {
    let mut response_headers = HeaderMap::new();

    if let Some(cookie_header) = headers.get(COOKIE) {
        if let Ok(cookie_str) = cookie_header.to_str() {
            for cookie in cookie_str.split(';') {
                let name = cookie.trim().split('=').next().unwrap_or("").trim();
                if name.starts_with("authentik_proxy_") {
                    let expire = format!(
                        "{}=; expires=Thu, 01 Jan 1970 00:00:00 GMT; \
                         path=/; domain=.cogmai.com; Secure; HttpOnly; SameSite=Lax",
                        name
                    );
                    if let Ok(val) = expire.parse() {
                        response_headers.append(SET_COOKIE, val);
                    }
                }
            }
        }
    }

    (StatusCode::OK, response_headers, "logged out")
}
