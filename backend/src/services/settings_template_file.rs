//! Does the file a settings row NAMES actually exist? (task 2.15 Tier 2, R3.)
//!
//! One row — `theme_scan_prompt_file` — holds a filename rather than a number or
//! a sentence, and a filename is the one kind of stored value whose correctness
//! lives outside the database. This module is the check, in the two places it has
//! to happen.
//!
//! ## Why BOTH a boot assertion and a write-path check, and not either alone
//!
//! `settings_store::trial_snapshot` exists to guarantee that "a change accepted
//! here can never produce a store that refuses to boot". A boot assertion added on
//! its own would break exactly that promise: a typo typed into the Settings page
//! passes the text validation (non-blank, no placeholders required), commits, and
//! then kills the service at the next restart — hours later, with nothing on the
//! page having said so.
//!
//! So the write path refuses a filename that does not resolve, at the moment a
//! human can still fix it, AND boot refuses to start if the stored one has since
//! stopped resolving (an un-deployed prompt file, a changed template directory —
//! neither of which the write path can foresee). Together they mean: the row you
//! can save is a row that works, and the service never runs with a prompt it
//! cannot read.
//!
//! ## Why the directory is passed in rather than read from the registry here
//!
//! The registry is a pipeline type; the settings store is not. Taking the resolved
//! directory as a string keeps this module free of that dependency and makes every
//! branch testable against a `tempdir` instead of a deployment.

use std::path::PathBuf;

use crate::pipeline::registry::PipelineRegistry;
use crate::services::settings_store::{SettingsError, KEY_THEME_SCAN_PROMPT_FILE};

/// Where template files live, resolved once by the caller from the registry.
///
/// ## Rust Learning: a newtype over `String`, and what it buys
///
/// This could be a bare `&str` parameter. Wrapping it means a caller cannot pass
/// the FILENAME where the DIRECTORY belongs — the compiler rejects it — and the
/// type name says which of the two a value is at every call site. The cost is one
/// struct; the alternative is two same-typed arguments in the wrong order
/// producing a check that silently always passes.
#[derive(Debug, Clone)]
pub struct TemplateDir(String);

impl TemplateDir {
    /// Wrap a resolved template directory path.
    pub fn new(path: impl Into<String>) -> Self {
        TemplateDir(path.into())
    }

    /// The full path one template filename resolves to.
    pub fn path_of(&self, filename: &str) -> PathBuf {
        PathBuf::from(&self.0).join(filename.trim())
    }

    /// Whether that path is a readable FILE (not a directory, not absent).
    ///
    /// `is_file()` answers "exists AND is a regular file" in one call and treats
    /// every I/O error as false — which is the right reading here: a path this
    /// process cannot stat is a path it cannot read a prompt from, and the caller
    /// reports the name either way.
    pub fn resolves(&self, filename: &str) -> bool {
        self.path_of(filename).is_file()
    }
}

/// For a row that NAMES A FILE, prove the file is there before accepting it.
///
/// Only `theme_scan_prompt_file` is such a row today, and the `match` is what
/// keeps this honest: a future filename row must be added here deliberately rather
/// than inheriting a check by accident (or, worse, being silently unchecked
/// because a broad heuristic like "the value ends in .md" did not fire).
///
/// # Errors
/// Returns [`SettingsError::FileNotFound`] naming the key, the value and the full
/// path that was looked for.
pub(crate) fn check_named_file(
    key: &str,
    candidate: &str,
    templates: &TemplateDir,
) -> Result<(), SettingsError> {
    match key {
        KEY_THEME_SCAN_PROMPT_FILE if !templates.resolves(candidate) => {
            Err(SettingsError::FileNotFound {
                key: key.to_string(),
                value: candidate.to_string(),
                path: templates.path_of(candidate).display().to_string(),
            })
        }
        _ => Ok(()),
    }
}

/// BOOT PRECONDITION: the judging prompt the settings row names is deployed, or
/// the process does not start (ruling R3, 2026-08-08).
///
/// ## Why refusing to boot is proportionate here
///
/// A Theme Scan is the prompt's only consumer, and without it every scan fails at
/// the door — so a process that starts is a process advertising a feature it
/// cannot perform. That is not hypothetical: for eleven days in July, scans died
/// on a missing prompt and the only surface saying so was a toast the next
/// navigation erased. A refusal at boot is the same failure, said once, in the
/// place an operator is already looking.
///
/// Reaching this refusal means the file was removed or the directory moved AFTER
/// the row was set — [`check_named_file`] refuses a filename that does not resolve
/// at the moment it is saved — so it is a deployment fault, and the log line names
/// both the file and the full path.
///
/// Takes the registry so the caller stays one line; the split from `main.rs` also
/// keeps that file's boot sequence readable at a glance.
pub fn assert_scan_prompt_deployed(prompt_file: &str, registry: &PipelineRegistry) {
    let templates = TemplateDir::new(registry.template_dir());
    if templates.resolves(prompt_file) {
        tracing::info!(
            file = %prompt_file,
            "Theme Scan judging prompt resolved from the settings store"
        );
        return;
    }

    tracing::error!(
        key = KEY_THEME_SCAN_PROMPT_FILE,
        file = %prompt_file,
        path = %templates.path_of(prompt_file).display(),
        "BOOT REFUSED: the Theme Scan judging prompt named by the settings store is \
         not deployed. Deploy the file to the template directory, or change the \
         theme_scan_prompt_file row to one that exists."
    );
    std::process::exit(1);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    /// A scratch directory that cleans itself up. Written here rather than pulling
    /// in a temp-dir crate for two tests.
    struct Scratch(PathBuf);

    impl Scratch {
        fn new(name: &str) -> Self {
            let path = std::env::temp_dir().join(format!("colossus-template-check-{name}"));
            let _ = fs::remove_dir_all(&path); // best-effort: a leftover from a previous run
            fs::create_dir_all(&path).expect("test scratch dir is creatable");
            Scratch(path)
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0); // best-effort: test cleanup
        }
    }

    #[test]
    fn a_named_file_that_exists_resolves() {
        let scratch = Scratch::new("exists");
        fs::write(scratch.0.join("theme_scan_prompt_v3.md"), "judge this")
            .expect("test file is writable");
        let dir = TemplateDir::new(scratch.0.to_string_lossy().to_string());

        assert!(dir.resolves("theme_scan_prompt_v3.md"));
        // Surrounding whitespace is trimmed, so a filename pasted with a trailing
        // space out of a terminal still names the file it looks like it names.
        assert!(dir.resolves("  theme_scan_prompt_v3.md "));
    }

    #[test]
    fn a_typo_and_a_directory_both_fail_the_check() {
        let scratch = Scratch::new("typo");
        fs::write(scratch.0.join("theme_scan_prompt_v3.md"), "judge this")
            .expect("test file is writable");
        fs::create_dir_all(scratch.0.join("subdir")).expect("test subdir is creatable");
        let dir = TemplateDir::new(scratch.0.to_string_lossy().to_string());

        assert!(
            !dir.resolves("theme_scan_prompt_v4.md"),
            "a filename nobody deployed must not pass — this is the typo the \
             Settings page refuses"
        );
        assert!(
            !dir.resolves("subdir"),
            "a directory is not a prompt; `is_file` must reject it rather than \
             letting the read fail later with a confusing message"
        );
    }

    /// The Settings page refuses a prompt filename nobody deployed, and says where
    /// it looked.
    ///
    /// This is the write-path half of ruling R3, and the reason it must be a test:
    /// without this refusal a typo commits, the page says "saved", and the service
    /// dies at its next restart — hours later, with nothing having warned anyone.
    /// The message is asserted because the PATH is the actionable part: "no such
    /// file" without it sends a human hunting through three directories.
    #[test]
    fn a_prompt_filename_that_is_not_deployed_is_refused_with_its_path() {
        let scratch = Scratch::new("write-path");
        fs::write(scratch.0.join("theme_scan_prompt_v3.md"), "judge this")
            .expect("test file is writable");
        let dir = TemplateDir::new(scratch.0.to_string_lossy().to_string());

        let error = check_named_file(KEY_THEME_SCAN_PROMPT_FILE, "theme_scan_prompt_v9.md", &dir)
            .expect_err("v9 was never deployed");

        let message = error.to_string();
        assert!(message.contains("theme_scan_prompt_v9.md"), "{message}");
        assert!(message.contains(KEY_THEME_SCAN_PROMPT_FILE), "{message}");
        assert!(
            message.contains(&scratch.0.to_string_lossy().to_string()),
            "the refusal must name the directory it looked in: {message}"
        );

        // The one that IS deployed goes through — the check must not refuse
        // everything, which is the way a guard silently becomes a wall.
        assert!(
            check_named_file(KEY_THEME_SCAN_PROMPT_FILE, "theme_scan_prompt_v3.md", &dir).is_ok()
        );
    }

    /// The check is keyed, and only that key is checked.
    ///
    /// Every other parameter is a number or a sentence, and running a file
    /// existence test over "0.80" would refuse the whole Settings page. The `match`
    /// is what keeps the rule specific; this is what proves the match is doing it,
    /// rather than a branch that happens to fire on the one key ever tested.
    #[test]
    fn a_row_that_names_no_file_is_never_checked_against_the_filesystem() {
        let scratch = Scratch::new("other-keys");
        let dir = TemplateDir::new(scratch.0.to_string_lossy().to_string());

        for key in [
            "confidence_band_high",
            "theme_scan_prefilter_statement_types",
            "scan_history_view_label",
        ] {
            assert!(
                check_named_file(key, "nothing_like_a_file", &dir).is_ok(),
                "{key} does not name a file and must not be checked as one"
            );
        }
    }
}
