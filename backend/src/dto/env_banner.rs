//! The test-system bar's words, as the browser receives them.

use serde::{Deserialize, Serialize};

/// What `GET /api/env-banner` answers.
///
/// Four strings and nothing else — no token, no host, no version. What the
/// browser knows about which machine it is on comes from its own runtime config
/// (`api::env_banner`'s header says why), and this payload must never become a
/// second, disagreeing source of that answer.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EnvBannerDto {
    /// The warning sentence.
    pub text: String,
    /// The link beside it.
    pub link_label: String,
    /// The line printed at the top of every printed page.
    pub print_line: String,
    /// Where the link goes.
    pub real_url: String,
}
