//! backend/tests/timeline_subsets_auth.rs
//!
//! ⚑ T1.4, STEP 8 — an unauthenticated subset write is refused.
//!
//! Its own binary, and not a case in `timeline_subsets_integration.rs`, for two
//! reasons that both matter:
//!
//! * It needs NO database. Folding it into the integration walk would have made
//!   the one proof that can always run depend on a live Postgres.
//! * It writes `AUTH_MODE` into the process environment, which is global. A test
//!   sharing a process with six database steps must never do that — the
//!   `registry_tests` env-var race is this project's own record of what happens
//!   when one does.
//!
//! Run it:
//!   `cargo test -p colossus-legal-backend --test timeline_subsets_auth -- \
//!      --ignored --test-threads=1`

use axum::extract::FromRequestParts;

use colossus_legal_backend::auth::AuthUser;

#[tokio::test]
#[ignore = "mutates AUTH_MODE in this process; run with --test-threads=1"]
async fn an_unauthenticated_write_is_refused_before_any_handler_body_runs() {
    // ⚑ STEP 8, exercised rather than described. Every write handler in
    // `api::timeline_subsets` declares `user: AuthUser`, and THIS is what that
    // declaration does to a request with no Authentik headers: axum runs the
    // extractor before the body, and the extractor refuses.
    //
    // No database and no `AppState`: the impl is
    // `impl<S> FromRequestParts<S> for AuthUser where S: Send + Sync`, generic
    // over the state, so `()` is a legal state to extract against — which is the
    // whole reason the shared crate wrote it that way.
    //
    // The env write is why this test is `#[ignore]`d and asks for
    // `--test-threads=1`: `AUTH_MODE` is process-global, and a parallel test
    // reading it mid-flight would see whichever value happened to be set.
    std::env::set_var("AUTH_MODE", "required");

    let request = axum::http::Request::builder()
        .method("POST")
        .uri("/api/timeline/subsets")
        .body(())
        .expect("a request with no auth headers");
    let (mut parts, ()) = request.into_parts();

    let refusal = AuthUser::from_request_parts(&mut parts, &())
        .await
        .expect_err("a write with no Authentik headers must be refused");
    assert_eq!(refusal.error, "unauthorized");
    assert!(
        refusal.user.is_none(),
        "nobody is named, because nobody signed in"
    );

    // And the positive control, so the test is not passing because the extractor
    // refuses everything: the same request WITH the header succeeds.
    let signed = axum::http::Request::builder()
        .method("POST")
        .uri("/api/timeline/subsets")
        .header("x-authentik-username", "roman")
        .body(())
        .expect("a signed request");
    let (mut parts, ()) = signed.into_parts();
    let user = AuthUser::from_request_parts(&mut parts, &())
        .await
        .expect("a request carrying the Authentik header is accepted");
    assert_eq!(user.username, "roman");

    std::env::remove_var("AUTH_MODE");
}
