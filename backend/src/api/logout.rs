use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Redirect};

/// GET /api/logout — Expires the Authentik proxy cookie then redirects to OIDC end-session.
///
/// Why this exists:
/// Authentik's OIDC end-session endpoint clears the server-side session but does NOT
/// expire the outpost proxy cookie (authentik_proxy_568f3995). This HttpOnly cookie
/// cannot be cleared by JavaScript. The browser must navigate (not fetch) to this
/// endpoint so the Set-Cookie header is processed as a navigation response.
///
/// Flow: Sign Out button → GET /api/logout → Set-Cookie (expire) + 302 → outpost sign_out → login page
pub async fn logout() -> impl IntoResponse {
    let logout_url = std::env::var("AUTH_LOGOUT_URL").unwrap_or_else(|_| {
        "https://colossus-legal-dev.cogmai.com/outpost.goauthentik.io/sign_out".to_string()
    });

    let mut headers = HeaderMap::new();

    // Expire the proxy cookie. Must match the original cookie's Domain, Path,
    // HttpOnly, Secure, and SameSite attributes exactly.
    let cookie = "authentik_proxy_568f3995=deleted; \
                  Domain=.cogmai.com; \
                  Path=/; \
                  Max-Age=0; \
                  HttpOnly; \
                  Secure; \
                  SameSite=Lax";

    headers.insert(
        "set-cookie",
        HeaderValue::from_str(cookie).expect("valid cookie header"),
    );

    (StatusCode::FOUND, headers, Redirect::to(&logout_url))
}
