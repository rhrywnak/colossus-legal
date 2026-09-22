// =============================================================================
// backend/src/domain/wording_env_banner.rs — the test-system warning bar
// =============================================================================
//
// CC_TASK_ENV_BANNER_v1. Chuck and Marie can reach both machines, and nothing on
// screen said which one they were on: practice typed on the test machine is not
// there for trial. These are the words that say so.
//
// ## Why these are stored rows and not literals
//
// They are sentences a witness reads, on the one surface whose whole job is to
// be believed. Roman rewords them from the Settings page without a build.
//
// ## ⚑ The browser carries a COPY of the first sentence, and that is deliberate
//
// The bar must be on screen at first paint — before any request returns, and
// even when the backend is down, which is a state the test machine reaches more
// often than the real one. So the frontend ships a compiled fallback of
// `env_banner_text` and swaps in the stored value when it arrives. The two are
// pinned equal by a test that reads the migration (frontend
// `envBanner.test.ts`); they cannot drift.
//
// ## The URL is a row too
//
// `env_banner_real_url` is the address of the OTHER machine — a per-deployment
// fact, which Rule 2 keeps out of code. The warning never waits for it: the
// sentence paints immediately and the link appears with the stored words.

/// The words the test-system bar speaks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnvBannerWording {
    /// The warning itself. User words only — never the token this build read.
    pub text: String,
    /// The link beside it.
    pub link_label: String,
    /// The shorter line printed at the top of every printed page.
    pub print_line: String,
    /// Where the link goes: the real system's address.
    pub real_url: String,
}

// KEYS: the stable identifiers. Renaming one is a migration.
pub(crate) const KEY_ENV_BANNER_TEXT: &str = "env_banner_text";
pub(crate) const KEY_ENV_BANNER_LINK_LABEL: &str = "env_banner_link_label";
pub(crate) const KEY_ENV_BANNER_PRINT_LINE: &str = "env_banner_print_line";
pub(crate) const KEY_ENV_BANNER_REAL_URL: &str = "env_banner_real_url";

/// Every key this block reads, so a missing one is caught at boot BY NAME.
pub const ENV_BANNER_WORDING_KEYS: &[&str] = &[
    KEY_ENV_BANNER_TEXT,
    KEY_ENV_BANNER_LINK_LABEL,
    KEY_ENV_BANNER_PRINT_LINE,
    KEY_ENV_BANNER_REAL_URL,
];

/// Build an [`EnvBannerWording`] from the stored rows, or say which key is wrong.
///
/// # Errors
/// Whatever `read` returns for the first key that is missing, mis-kinded or blank.
pub fn build_env_banner_wording<E>(
    read: impl Fn(&str) -> Result<String, E>,
) -> Result<EnvBannerWording, E> {
    Ok(EnvBannerWording {
        text: read(KEY_ENV_BANNER_TEXT)?,
        link_label: read(KEY_ENV_BANNER_LINK_LABEL)?,
        print_line: read(KEY_ENV_BANNER_PRINT_LINE)?,
        real_url: read(KEY_ENV_BANNER_REAL_URL)?,
    })
}

#[cfg(test)]
#[path = "wording_env_banner_tests.rs"]
pub(crate) mod tests;
