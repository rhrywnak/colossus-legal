//! Read-time annotation of a stored scan summary.
//!
//! A completed run's `summary_json` is a HISTORICAL RECORD: it is exactly what the
//! scan produced, written once and never rewritten. Two things the run-results list
//! must show are deliberately NOT in it, because neither belongs to the scan:
//!
//! * the **candidate ordinal** (`C-14`) — identity owned by the scenario, which may
//!   be assigned after the run (a scan can judge a candidate that gathers later);
//! * the **applied** state — whether this run's judgment for a pick has already been
//!   merged, which changes every time the human merges and is therefore never a
//!   property of the run itself.
//!
//! Both are derived here, at read time, and layered onto a COPY of the stored JSON.
//! The stored row is never modified.
//!
//! ## Why this walks `serde_json::Value` instead of deserializing the summary
//!
//! Deserializing into `ThemeScanSummary` and re-serializing would be more typed,
//! and it would be WRONG: summaries written by earlier builds carry the old field
//! names (`relevant_written`, `dry_run`). A strict deserialize would fail on those
//! rows, so opening a historical run — the exact thing the history list exists to
//! make possible — would 500. Annotating the `Value` in place touches only the keys
//! it adds, so every summary ever written stays readable.
//!
//! A summary whose shape is unrecognized is left EXACTLY as found and logged, never
//! silently blanked: an un-annotated row renders as a run whose picks carry no chip
//! and no applied badge, which is honest, rather than as an empty result list.

use std::collections::{HashMap, HashSet};

use serde_json::Value;
use uuid::Uuid;

// CONST: JSON key names in the stored-summary / wire contract. These are PROTOCOL
// text — the field names the backend serializer writes and the frontend
// deserializer reads — in exactly the same standing as the SQL query-text consts
// elsewhere in this changeset. They cannot be deployment config: changing one
// would break every summary already persisted AND the frontend type in the same
// instant, so it is a coordinated code+data migration, never a YAML edit. Rule 2
// N/A. (They are named rather than inlined so the read key and the written key
// cannot drift apart — `graph_node_id` is read here and matched against ids the
// repository returns.)
/// The JSON key holding the per-pick list inside a stored summary.
const SUGGESTIONS_KEY: &str = "suggestions";
/// The key each suggestion carries identifying its graph node.
const NODE_ID_KEY: &str = "graph_node_id";
/// The annotation keys this module adds.
const ORDINAL_KEY: &str = "ordinal";
const APPLIED_KEY: &str = "applied";

/// What one annotation pass did — the two counts a reader needs to tell a healthy
/// summary from a damaged one.
///
/// Returned instead of a bare count because "annotated 4 of 5" is a different
/// operational state from "annotated 4 of 4", and a single number cannot express
/// the difference (Standing Rule 1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct AnnotationOutcome {
    /// Suggestions that received an ordinal + applied state.
    pub annotated: usize,
    /// Suggestions left untouched because they carry no readable `graph_node_id`.
    /// Should always be 0 — every suggestion is written with its `evidence_id` —
    /// so a non-zero value means a stored row is damaged, and the caller warns.
    pub skipped: usize,
}

/// Annotate every suggestion in `summary` with its ordinal and applied state.
///
/// Mutates in place (the caller owns a copy read from the row). Returns what the
/// pass did, so the caller can log a damaged summary rather than reporting a
/// silent success.
///
/// ## Standing Rule 1: the three distinguishable outcomes
///
/// * summary has a suggestions ARRAY → each entry annotated, counts returned;
/// * summary has no `suggestions` key, or it is not an array → `None`, so the
///   caller logs the unexpected shape rather than reporting a silent success;
/// * an individual suggestion with no readable `graph_node_id` → left untouched
///   and counted as `skipped`, NOT as annotated. Without that separate count, a
///   pick rendering with no chip would be indistinguishable in the logs from one
///   whose ordinal simply has not been assigned yet.
///
/// An absent ordinal is written as JSON `null` rather than omitted-or-zero: the
/// frontend must be able to tell "this pick has no id yet" from "id 0".
pub(crate) fn annotate_suggestions(
    summary: &mut Value,
    ordinals: &HashMap<String, i32>,
    applied: &HashSet<String>,
) -> Option<AnnotationOutcome> {
    // `as_array_mut` yields None for both "key absent" and "present but not an
    // array" — the caller treats either as the same unexpected-shape case.
    let suggestions = summary.get_mut(SUGGESTIONS_KEY)?.as_array_mut()?;

    let mut annotated = 0;
    let mut skipped = 0;
    for suggestion in suggestions.iter_mut() {
        // Read the id before taking a mutable borrow to insert: the id is cloned so
        // the immutable borrow of `suggestion` ends before the mutable one begins.
        let Some(node_id) = suggestion
            .get(NODE_ID_KEY)
            .and_then(Value::as_str)
            .map(str::to_string)
        else {
            // No id to key on. Leave the entry exactly as stored — inventing a
            // default here would render a wrong chip on a real pick — and COUNT it,
            // so the caller can say so out loud.
            skipped += 1;
            continue;
        };

        let Some(object) = suggestion.as_object_mut() else {
            skipped += 1;
            continue;
        };

        object.insert(
            ORDINAL_KEY.to_string(),
            // `Option<i32>` → `Value::Number` or `Value::Null`. The explicit null
            // (rather than omitting the key) keeps "unnumbered" a positive answer.
            ordinals
                .get(&node_id)
                .map_or(Value::Null, |o| Value::from(*o)),
        );
        object.insert(
            APPLIED_KEY.to_string(),
            Value::Bool(applied.contains(&node_id)),
        );
        annotated += 1;
    }

    Some(AnnotationOutcome { annotated, skipped })
}

/// Annotate a summary, logging anything unexpected about its shape.
///
/// Wraps [`annotate_suggestions`] with the observability half so the caller stays a
/// single line. A summary that could not be annotated is left as-is and logged with
/// its run id — the run still renders (Standing Rule 1: degraded, never silent).
pub(crate) fn annotate_summary_logged(
    summary: &mut Value,
    run_id: Uuid,
    ordinals: &HashMap<String, i32>,
    applied: &HashSet<String>,
) {
    match annotate_suggestions(summary, ordinals, applied) {
        Some(outcome) => {
            tracing::debug!(
                %run_id,
                annotated = outcome.annotated,
                "annotated scan-run suggestions"
            );
            // Should be impossible (every suggestion is written with its
            // evidence_id), so a non-zero count means a stored row is damaged.
            // Warned separately because otherwise a pick rendering with no chip is
            // indistinguishable in the logs from one that is merely un-numbered.
            if outcome.skipped > 0 {
                tracing::warn!(
                    %run_id,
                    skipped = outcome.skipped,
                    annotated = outcome.annotated,
                    "stored scan summary contains suggestions with no readable \
                     `graph_node_id`; they were left un-annotated (no candidate id, \
                     no applied state) — the stored row is malformed"
                );
            }
        }
        None => {
            tracing::warn!(
                %run_id,
                "stored scan summary has no readable `suggestions` array; serving it \
                 un-annotated (picks will show no candidate id and no applied state)"
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn ordinals(pairs: &[(&str, i32)]) -> HashMap<String, i32> {
        pairs.iter().map(|(n, o)| (n.to_string(), *o)).collect()
    }

    fn applied(ids: &[&str]) -> HashSet<String> {
        ids.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn annotates_each_suggestion_with_its_ordinal_and_applied_state() {
        let mut summary = json!({
            "suggestions": [
                { "graph_node_id": "ev-a", "confidence": 0.9 },
                { "graph_node_id": "ev-b", "confidence": 0.5 },
            ]
        });

        let count = annotate_suggestions(
            &mut summary,
            &ordinals(&[("ev-a", 14), ("ev-b", 22)]),
            &applied(&["ev-a"]),
        );

        assert_eq!(
            count,
            Some(AnnotationOutcome {
                annotated: 2,
                skipped: 0
            })
        );
        let s = &summary["suggestions"];
        assert_eq!(s[0]["ordinal"], 14);
        assert_eq!(s[0]["applied"], true, "ev-a's judgment was already merged");
        assert_eq!(s[1]["ordinal"], 22);
        assert_eq!(
            s[1]["applied"], false,
            "ev-b is still checkable — not merged from this run"
        );
        // The scan's own data is untouched.
        assert_eq!(s[0]["confidence"], 0.9);
    }

    #[test]
    fn an_unnumbered_pick_gets_an_explicit_null_not_a_zero() {
        // Standing Rule 1: "no id yet" and "id 0" must not collapse. A zero would
        // render as "C-0", a card that does not exist.
        let mut summary = json!({ "suggestions": [{ "graph_node_id": "ev-new" }] });

        annotate_suggestions(&mut summary, &HashMap::new(), &HashSet::new());

        assert!(
            summary["suggestions"][0]["ordinal"].is_null(),
            "an unassigned ordinal must be an explicit null"
        );
        assert_eq!(summary["suggestions"][0]["applied"], false);
    }

    #[test]
    fn a_historical_summary_with_retired_field_names_still_annotates() {
        // The regression this module's Value-walking exists to prevent: summaries
        // written before the rename carry `relevant_written`/`dry_run`. They must
        // still open — a strict deserialize would 500 on exactly the historical runs
        // the history list exists to retrieve.
        let mut summary = json!({
            "relevant_written": 2,
            "dry_run": true,
            "suggestions": [{ "graph_node_id": "ev-old", "proposed_role": "supports" }]
        });

        let count =
            annotate_suggestions(&mut summary, &ordinals(&[("ev-old", 3)]), &HashSet::new());

        assert_eq!(
            count,
            Some(AnnotationOutcome {
                annotated: 1,
                skipped: 0
            }),
            "a legacy-shaped summary must still annotate"
        );
        assert_eq!(summary["suggestions"][0]["ordinal"], 3);
        // The retired fields are carried through untouched — the stored row is a
        // historical record, not something to retro-fit.
        assert_eq!(summary["relevant_written"], 2);
    }

    #[test]
    fn an_unrecognized_shape_reports_none_rather_than_claiming_success() {
        // No suggestions key at all (a running/failed run's partial summary).
        let mut no_key = json!({ "candidates_read": 0 });
        assert_eq!(
            annotate_suggestions(&mut no_key, &HashMap::new(), &HashSet::new()),
            None
        );

        // Present but the wrong type — must not be treated as "zero suggestions".
        let mut wrong_type = json!({ "suggestions": "none" });
        assert_eq!(
            annotate_suggestions(&mut wrong_type, &HashMap::new(), &HashSet::new()),
            None
        );
        assert_eq!(
            wrong_type["suggestions"], "none",
            "an unrecognized summary is left exactly as stored"
        );
    }

    #[test]
    fn a_suggestion_without_an_id_is_left_untouched_and_uncounted() {
        // It cannot be keyed, so it gets no chip and no badge — and it must not be
        // counted as annotated, or the log would overstate what was done.
        let mut summary = json!({
            "suggestions": [
                { "graph_node_id": "ev-a" },
                { "confidence": 0.4 },
            ]
        });

        let count = annotate_suggestions(&mut summary, &ordinals(&[("ev-a", 1)]), &HashSet::new());

        // The un-keyable entry is counted as SKIPPED, not silently absorbed: the
        // caller warns on a non-zero skip, which is what lets an operator tell a
        // damaged stored row from a merely un-numbered pick.
        assert_eq!(
            count,
            Some(AnnotationOutcome {
                annotated: 1,
                skipped: 1
            }),
            "only the keyable suggestion is annotated; the other is counted skipped"
        );
        assert!(summary["suggestions"][1].get("ordinal").is_none());
        assert!(summary["suggestions"][1].get("applied").is_none());
    }
}
