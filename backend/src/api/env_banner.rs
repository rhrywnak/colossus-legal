//! The test-system bar's words.
//!
//! CC_TASK_ENV_BANNER_v1. `GET /api/env-banner` — the four stored strings the
//! warning is drawn from.
//!
//! ## ⚑ This endpoint does NOT decide whether the bar shows
//!
//! The browser does, from its own runtime config
//! (`window.__COLOSSUS_CONFIG__.environment`, written into `config.js` at deploy
//! time). Asking the backend "am I the test machine?" would be asking the wrong
//! process: the frontend container is the one that was deployed to a machine,
//! and its answer arrives before any request is made — which is what lets the
//! real system draw nothing at all, with no flash and no reserved space.
//!
//! ## Why it needs no `AuthUser`
//!
//! It returns four sentences that are already on every screen of the deployment
//! that shows them, and it is read at first paint by a shell that may be drawn
//! before anything else has loaded. Requiring auth would buy nothing — the app's
//! routes are behind ForwardAuth anyway — and would cost the warning its
//! independence from the session.

use axum::{extract::State, Json};

use crate::dto::env_banner::EnvBannerDto;
use crate::state::AppState;

/// The bar's words, from the settings store.
///
/// Infallible by construction: the four rows are required at boot, so a running
/// server has them. No failure arm exists to swallow.
pub async fn get_env_banner(State(state): State<AppState>) -> Json<EnvBannerDto> {
    let words = &state.settings.current().env_banner_wording;
    Json(EnvBannerDto {
        text: words.text.clone(),
        link_label: words.link_label.clone(),
        print_line: words.print_line.clone(),
        real_url: words.real_url.clone(),
    })
}
