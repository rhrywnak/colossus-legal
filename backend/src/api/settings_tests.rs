//! Tests for [`super`] — what the Settings page is told (task 1.6, v2 §2b).
//!
//! The handlers need a live Postgres and are DEV-verified. What is pinned here is
//! everything they COMPOSE: the bounds sentence, the dormancy note, the
//! last-changed line, and the error mapping — every one of them a user-visible
//! string that the language law puts on this side of the wire.

use super::*;

use chrono::{TimeZone, Utc};

use crate::repositories::pipeline_repository::PipelineRepoError;

fn record(kind: ValueKind, consumed_by: Option<&str>, updated_by: &str) -> AppSettingRecord {
    AppSettingRecord {
        key: "talking_points_cap".to_string(),
        value: "3".to_string(),
        value_kind: kind.code().to_string(),
        default_value: "3".to_string(),
        min_value: Some(1.0),
        max_value: None,
        meaning: "The most talking points one scenario may carry (§10).".to_string(),
        consumed_by: consumed_by.map(str::to_string),
        updated_at: Utc
            .with_ymd_and_hms(2026, 8, 1, 12, 0, 0)
            .single()
            .expect("a real timestamp"),
        updated_by: updated_by.to_string(),
    }
}

// ── The bounds sentence ──────────────────────────────────────────────────────

#[test]
fn two_sided_bounds_read_as_a_range() {
    assert_eq!(
        bounds_label(Some(0.0), Some(1.0)).as_deref(),
        Some("Between 0 and 1")
    );
}

#[test]
fn a_one_sided_bound_does_not_invent_the_other_end() {
    // "At least 1" is the truth. "Between 1 and ∞" or "1 to 999" would both be
    // this function making up a ceiling the store does not declare.
    assert_eq!(bounds_label(Some(1.0), None).as_deref(), Some("At least 1"));
    assert_eq!(
        bounds_label(None, Some(10.0)).as_deref(),
        Some("At most 10")
    );
}

#[test]
fn an_unbounded_parameter_shows_no_bounds_line_at_all() {
    // The card-test ratio declares neither end. An empty "Bounds:" label would
    // suggest a constraint exists and the page failed to load it.
    assert!(bounds_label(None, None).is_none());
}

// ── The dormancy note ────────────────────────────────────────────────────────

/// A dormant parameter says BOTH halves: who will read it, and that nothing does.
///
/// Naming only the task leaves a human to infer the second half, and the
/// inference most people make is the optimistic one — that turning the knob does
/// something now.
#[test]
fn a_dormant_parameter_says_plainly_that_changing_it_does_nothing_yet() {
    let note = dormant_note(Some("task 2.4")).expect("a dormant parameter is labelled");
    assert!(note.contains("task 2.4"), "{note}");
    assert!(note.contains("no effect today"), "{note}");
}

#[test]
fn a_live_parameter_carries_no_dormancy_note() {
    // A "this does nothing" label on a live parameter would be worse than none.
    assert!(dormant_note(None).is_none());
}

// ── The last-changed line ────────────────────────────────────────────────────

/// A never-touched parameter is visibly different from one set to the same value.
///
/// Both show `3`. "Never changed since it shipped" and "Last changed by Roman"
/// are operationally distinct states, and Standing Rule 1 says they must read
/// differently.
#[test]
fn a_seeded_parameter_says_it_has_never_been_changed() {
    let line = last_changed(&record(ValueKind::Count, None, "system (seed)"));
    assert!(line.contains("Never changed"), "{line}");
}

#[test]
fn an_edited_parameter_names_who_changed_it_and_when() {
    let line = last_changed(&record(ValueKind::Count, None, "Roman"));
    assert!(line.contains("Roman"), "{line}");
    assert!(line.contains("2026-08-01"), "{line}");
}

// ── The composed row ─────────────────────────────────────────────────────────

#[test]
fn a_row_carries_everything_2b_requires_of_the_page() {
    // §2b: "every parameter with its current value, its default, and a one-line
    // plain-language meaning". All three, asserted rather than assumed.
    let dto = to_dto(&record(ValueKind::Count, None, "Roman"));
    assert_eq!(dto.value, "3");
    assert_eq!(dto.default_value, "3");
    assert!(!dto.meaning.is_empty());
    assert!(
        !dto.input_hint.is_empty(),
        "a human needs to know what to type"
    );
}

#[test]
fn the_input_hint_matches_the_stored_kind() {
    let ratio = to_dto(&record(ValueKind::Ratio, None, "Roman"));
    assert!(ratio.input_hint.contains("n/m"), "{}", ratio.input_hint);

    let count = to_dto(&record(ValueKind::Count, None, "Roman"));
    assert!(
        count.input_hint.contains("whole number"),
        "{}",
        count.input_hint
    );
}

/// A row whose kind this build cannot read still RENDERS.
///
/// The Settings page is where someone would go to repair such a row. Refusing to
/// show it — or showing it with a confident but wrong hint — would hide the one
/// screen that can fix it.
#[test]
fn an_unreadable_kind_still_renders_with_an_honest_hint() {
    let mut broken = record(ValueKind::Count, None, "Roman");
    broken.value_kind = "percentage".to_string();

    let dto = to_dto(&broken);
    assert!(dto.input_hint.contains("percentage"), "{}", dto.input_hint);
    assert!(
        dto.input_hint.contains("does not recognise"),
        "the hint must admit the build cannot read it: {}",
        dto.input_hint
    );
}

// ── The error mapping ────────────────────────────────────────────────────────

fn a_repo_error() -> PipelineRepoError {
    PipelineRepoError::from(sqlx::Error::RowNotFound)
}

/// An invalid value reaches the human with the detail that makes it fixable.
#[test]
fn an_invalid_value_is_a_400_carrying_the_bound_it_crossed() {
    let error = settings_error_to_app_error(SettingsError::Invalid {
        source: crate::domain::settings::SettingError::BelowMinimum {
            key: "talking_points_cap".to_string(),
            value: "0".to_string(),
            min: 1.0,
        },
    });
    let AppError::BadRequest { message, details } = error else {
        panic!("a value the human typed is their mistake to fix, not a 500");
    };
    assert_eq!(details, json!({ "reason": "invalid_setting" }));
    assert!(message.contains("talking_points_cap"), "{message}");
    assert!(message.contains("at least 1"), "{message}");
}

#[test]
fn a_no_op_change_is_a_400_that_says_so() {
    let error = settings_error_to_app_error(SettingsError::Unchanged {
        key: "talking_points_cap".to_string(),
        value: "3".to_string(),
    });
    let AppError::BadRequest { message, .. } = error else {
        panic!("a no-op is refused, not accepted");
    };
    assert!(message.contains("nothing to change"), "{message}");
}

/// An unknown key is a 404, not a 400.
///
/// The key names a resource. A page offering a parameter this build has never
/// heard of is a page out of date with the deployment, and "no such parameter" is
/// the accurate report — a 400 would suggest the VALUE was the problem.
#[test]
fn an_unknown_key_is_a_404_naming_it() {
    let error = settings_error_to_app_error(SettingsError::UnknownKey {
        key: "invented_parameter".to_string(),
    });
    let AppError::NotFound { message } = error else {
        panic!("an unknown key is a missing resource");
    };
    assert!(message.contains("invented_parameter"), "{message}");
}

/// A failed read and a failed write tell the human different things.
#[test]
fn a_failed_read_and_a_failed_write_report_different_verbs() {
    let read = settings_error_to_app_error(SettingsError::Read {
        source: a_repo_error(),
    });
    let write = settings_error_to_app_error(SettingsError::Write {
        source: a_repo_error(),
    });

    let (AppError::Internal { message: read_msg }, AppError::Internal { message: write_msg }) =
        (read, write)
    else {
        panic!("a backend fault is a 500 on both sides");
    };
    assert_eq!(read_msg, "failed to load the settings");
    assert_eq!(write_msg, "failed to save the setting");
    assert_ne!(read_msg, write_msg);
}

// ── The configuration law, asserted against the source ───────────────────────

/// No compiled-in parameter survives anywhere in the scenario surface.
///
/// THE POINT OF TASK 1.6, as a build failure rather than a promise (Standing Rule
/// 21). Each name below was a `const` before this task; the settings store now
/// serves all four. A future change that reintroduces any of them — as a
/// fallback, a test fixture promoted to production, or a "temporary" default —
/// fails here.
#[test]
fn the_four_retired_constants_are_gone_and_stay_gone() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");

    fn walk(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk(&path, out);
            } else if path.extension().and_then(|e| e.to_str()) == Some("rs") {
                out.push(path);
            }
        }
    }

    let mut files = Vec::new();
    walk(&root, &mut files);
    files.sort();

    // Anti-vacuity: a broken walk would silently pass this test.
    assert!(
        files.len() > 50,
        "the source scan found only {} files — it is checking nothing",
        files.len()
    );

    let mut offenders = Vec::new();
    for path in &files {
        // This file names the retired constants in order to search for them.
        if path.ends_with("settings_tests.rs") {
            continue;
        }
        let Ok(source) = std::fs::read_to_string(path) else {
            continue;
        };
        for retired in [
            "HIGH_CONFIDENCE_CUTOFF",
            "MEDIUM_CONFIDENCE_CUTOFF",
            "DEFAULT_CONTEXT_WINDOW_CHARS",
            "DEFAULT_TALKING_POINTS_CAP",
        ] {
            if source.contains(retired) {
                offenders.push(format!("{} names {retired}", path.display()));
            }
        }
    }

    assert!(
        offenders.is_empty(),
        "a retired parameter constant is back in the source. Every tunable is \
         served from app_settings (v2 §2b): 'a compiled-in parameter is a defect \
         of the same class as a hardcoded person name'.\n  {}",
        offenders.join("\n  ")
    );
}
