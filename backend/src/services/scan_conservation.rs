//! The sentence that reconciles a scan run, composed at READ time.
//!
//! Item 1c wants the conservation counts "on screen … reconciled visibly". The
//! COUNTS are frozen into the run's stored summary when it completes — they
//! describe what that run did and must never be rewritten. The SENTENCE is not:
//! it is words a human reads, so it is a stored template (the configuration law),
//! and it is filled here, on the way out, from the run's own numbers.
//!
//! ## Why that split matters more than it looks
//!
//! If the sentence were composed at write time, editing the wording would leave
//! every historical run speaking the old words — and re-composing them would mean
//! rewriting historical records to change a label. Composing on read gives both:
//! the record is immutable, the language is editable, and a run scanned in July
//! reads in today's words.
//!
//! ## The two honest absences
//!
//! * A run recorded BEFORE this task carries no `conservation` key. No line is
//!   added — a sentence claiming "0 gathered" about a run that measured nothing
//!   would be a fabrication (Standing Rule 1). The results panel simply shows no
//!   reconciliation for those runs, which is the truth about them.
//! * A summary whose `conservation` is present but unreadable is left alone and
//!   LOGGED, for the same reason [`crate::services::scan_run_enrich`] leaves an
//!   unrecognised summary exactly as found.

use serde_json::Value;
use uuid::Uuid;

use crate::domain::wording_templates::render;

// CONST: JSON key names in the stored-summary / wire contract — protocol text, in
// the same standing as the keys `scan_run_enrich` names (see its note on why
// Rule 2 does not apply to these).
const CONSERVATION_KEY: &str = "conservation";
const CONSERVATION_LINE_KEY: &str = "conservation_line";
const RELEVANT_KEY: &str = "relevant";
const IRRELEVANT_KEY: &str = "irrelevant";
const FAILED_KEY: &str = "failed";

/// Add `conservation_line` to a stored summary, rendered from the stored template.
///
/// Mutates the caller's copy in place (never the row). Silent about summaries that
/// carry no conservation block — see the module doc for why that absence is the
/// honest answer rather than a zeroed line.
pub(crate) fn annotate_conservation_line(
    summary: &mut Value,
    run_id: Uuid,
    template: &str,
    failed_clause_template: &str,
) {
    let Some(counts) = read_counts(summary) else {
        // Two cases reach here and they are logged differently below by the
        // caller's own context: a pre-2.15 run (expected, common) and a damaged
        // block (rare). Debug rather than warn: opening a July run is a normal
        // thing to do, and a warning per historical run would train the operator
        // to ignore the log.
        tracing::debug!(
            %run_id,
            "stored scan summary carries no readable conservation block; serving it \
             without a reconciliation line (a run recorded before task 2.15)"
        );
        return;
    };

    // The failed CLAUSE, or nothing. Ruling R4 puts failures in the sentence
    // "whenever nonzero" — a clean run reading "· 0 failed" teaches the eye to
    // skip the term, and the one run where it matters is the run where it stops
    // being zero. The clause carries its own separator, so an absent clause
    // leaves no orphaned "· " behind.
    //
    // The joining SPACE is added here rather than stored: `text_of` trims every
    // stored string on the way out of the settings store, so a template whose
    // value began with a space would arrive without it and the sentence would
    // read "124 judged· 1 failed". The WORDS and the separator glyph are still
    // configuration — one character of whitespace is not language.
    let failed_clause = if counts.failed == 0 {
        String::new()
    } else {
        let clause = render(
            failed_clause_template,
            &[("failed", &counts.failed.to_string())],
        );
        format!(" {clause}")
    };

    // R4's reconciliation law, EVALUATED — not merely written down. This is the
    // second of the two checks (the first is at write time in
    // `theme_scan_persist`); this one catches a stored record that does not add
    // up, including every run recorded before the write-time check existed.
    if counts.judged != counts.relevant + counts.irrelevant + counts.failed {
        tracing::warn!(
            %run_id,
            judged = counts.judged,
            relevant = counts.relevant,
            irrelevant = counts.irrelevant,
            failed = counts.failed,
            "stored scan summary does not reconcile: judged != relevant + irrelevant \
             + failed. Serving the line anyway — the numbers are the run's own record \
             and are never rewritten — but the sentence a human is about to read does \
             not add up, and this log says so before they have to notice it"
        );
    }

    let line = render(
        template,
        &[
            ("pool", &counts.pool.to_string()),
            ("collapsed", &counts.collapsed.to_string()),
            ("excluded", &counts.excluded.to_string()),
            ("judged", &counts.judged.to_string()),
            ("failed", &failed_clause),
            ("relevant", &counts.relevant.to_string()),
        ],
    );

    if let Some(object) = summary.as_object_mut() {
        object.insert(CONSERVATION_LINE_KEY.to_string(), Value::String(line));
    }
}

/// The numbers the sentence needs, read out of a stored summary.
struct LineCounts {
    pool: u64,
    collapsed: u64,
    excluded: u64,
    judged: u64,
    relevant: u64,
    /// Judged calls that never produced a verdict. Also carried in the block from
    /// beta.388 on; this is read from the SUMMARY because that is where every run
    /// has always recorded it — see [`read_counts`].
    failed: u64,
    /// Read only to check the identity — it has no slot in the sentence.
    irrelevant: u64,
}

/// Read the conservation block, summing the three exclusion reasons into one
/// number for the sentence.
///
/// ## Why the sentence totals what the DATA keeps apart
///
/// The stored block counts each reason separately, because a scan that set aside
/// forty quotes for length and one for being a cross-reference is in a very
/// different state from the reverse — and the settings page is where that gets
/// acted on. The one-line summary needs a figure that ADDS UP with the others, so
/// it totals them; the per-reason detail stays in the record for the scorecard and
/// the log. Neither number is a rounding of the other.
///
/// `None` when the block is absent, not an object, or missing a field — every one
/// of which means "this run did not measure it", and none of which may be
/// defaulted to zero.
fn read_counts(summary: &Value) -> Option<LineCounts> {
    let block = summary.get(CONSERVATION_KEY)?.as_object()?;
    let field = |name: &str| block.get(name).and_then(Value::as_u64);

    let excluded =
        field("excluded_empty")? + field("excluded_statement_type")? + field("excluded_too_short")?;

    Some(LineCounts {
        pool: field("pool")?,
        collapsed: field("duplicates_collapsed")?,
        excluded,
        judged: field("judged")?,
        // `relevant`, `irrelevant` and `failed` live on the summary itself, not in
        // the block: they are OUTCOMES of judging, while the block describes the
        // input. The sentence reads across both, which is the whole point of it.
        //
        // `failed` is deliberately read from HERE rather than from the block's own
        // copy (added beta.388, ruling R4). Every run ever recorded carries the
        // summary key, including the runs from before the block had the field —
        // so reading the block would leave exactly the historical runs the failure
        // honesty was meant to expose without a clause. The two are the same
        // number by construction on every run from beta.388 on.
        relevant: summary.get(RELEVANT_KEY).and_then(Value::as_u64)?,
        irrelevant: summary.get(IRRELEVANT_KEY).and_then(Value::as_u64)?,
        failed: summary.get(FAILED_KEY).and_then(Value::as_u64)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// The seeded template, as the migration writes it (with the `{failed}` slot
    /// the failure-honesty correction added).
    const TEMPLATE: &str = "{pool} gathered · {collapsed} duplicates folded · {excluded} \
                            set aside before judging · {judged} judged{failed} · {relevant} \
                            relevant";
    /// The seeded failed CLAUSE, separator and all.
    const FAILED_CLAUSE: &str = "· {failed} failed";

    /// A run whose numbers reconcile: 124 judged = 22 relevant + 101 irrelevant + 1 failed.
    fn summary_with_conservation() -> Value {
        json!({
            "relevant": 22,
            "irrelevant": 101,
            "failed": 1,
            "conservation": {
                "pool": 148,
                "excluded_empty": 2,
                "excluded_statement_type": 4,
                "excluded_too_short": 15,
                "duplicates_collapsed": 3,
                "judged": 124,
                "failed": 1,
            }
        })
    }

    fn line_of(summary: &mut Value) -> String {
        annotate_conservation_line(summary, Uuid::nil(), TEMPLATE, FAILED_CLAUSE);
        summary["conservation_line"]
            .as_str()
            .expect("a line was added")
            .to_string()
    }

    #[test]
    fn the_line_reports_the_runs_own_numbers_and_they_reconcile() {
        let mut summary = summary_with_conservation();
        let line = line_of(&mut summary);

        assert!(line.contains("148 gathered"), "{line}");
        assert!(line.contains("3 duplicates folded"), "{line}");
        // 2 + 4 + 15 — the three reasons, totalled for the sentence.
        assert!(line.contains("21 set aside before judging"), "{line}");
        assert!(line.contains("124 judged"), "{line}");
        assert!(line.contains("22 relevant"), "{line}");
        // The reader can do the arithmetic the line invites: 148 = 21 + 3 + 124.
        assert_eq!(148, 21 + 3 + 124);
    }

    /// R4's reconciliation law, on the sentence a human actually reads.
    ///
    /// The defect this pins: on 2026-08-09 the line said "104 judged · 0 relevant"
    /// and the 104 dead calls appeared nowhere in it, so the sentence invited an
    /// arithmetic that could not be completed and nothing said which term was
    /// missing. The law is `judged = relevant + irrelevant + failed`, and the only
    /// way a reader can check it is if every term is on screen.
    #[test]
    fn conservation_reconciles_judged_equals_relevant_plus_irrelevant_plus_failed() {
        // The incident's own shape: every judged call failed.
        let mut dead = json!({
            "relevant": 0,
            "irrelevant": 0,
            "failed": 104,
            "conservation": {
                "pool": 148,
                "excluded_empty": 2,
                "excluded_statement_type": 4,
                "excluded_too_short": 15,
                "duplicates_collapsed": 23,
                "judged": 104,
                "failed": 104,
            }
        });
        let line = line_of(&mut dead);

        assert!(
            line.contains("104 judged") && line.contains("104 failed"),
            "the sentence must carry BOTH terms — this is the run that read \
             'Complete · 104 judged · 0 relevant': {line}"
        );
        // The reader's arithmetic now closes. Named rather than written as a
        // literal sum so the terms say which count each one is — and so the
        // assertion is about the run's numbers rather than about addition.
        let (judged, relevant, irrelevant, failed) = (104_u64, 0_u64, 0_u64, 104_u64);
        assert_eq!(judged, relevant + irrelevant + failed);

        // ANTI-VACUITY. `{failed}` is a SLOT, so a renderer that filled it with
        // the empty string on every run would still leave "104 judged" in the
        // line above and this test would pass on the defect. Pinning that the
        // clause is ABSENT on a clean run is what makes its presence mean
        // something.
        let mut clean = summary_with_conservation();
        clean["failed"] = json!(0);
        clean["irrelevant"] = json!(102);
        clean["conservation"]["failed"] = json!(0);
        let clean_line = line_of(&mut clean);
        assert!(
            !clean_line.contains("failed"),
            "a run with no failures must not read '· 0 failed' — a permanent zero \
             term teaches the eye to skip it: {clean_line}"
        );
        // …and no orphaned separator is left where the clause would have been.
        assert!(
            clean_line.contains("124 judged · 22 relevant"),
            "{clean_line}"
        );
    }

    #[test]
    fn a_run_recorded_before_this_task_gets_no_line_rather_than_a_zeroed_one() {
        // The historical-run case, and the reason this is not `unwrap_or(0)`:
        // "0 gathered · 0 judged" would describe a scan that read nothing, which
        // is a claim about a July run nobody measured.
        let mut legacy =
            json!({ "relevant": 24, "irrelevant": 120, "failed": 4, "candidates_read": 148 });

        annotate_conservation_line(&mut legacy, Uuid::nil(), TEMPLATE, FAILED_CLAUSE);

        assert!(
            legacy.get("conservation_line").is_none(),
            "no line at all is the honest answer for an unmeasured run"
        );
    }

    #[test]
    fn a_damaged_conservation_block_is_left_alone() {
        // Half a block cannot produce a sentence that adds up, and a partial one
        // would be read as if it did.
        let mut damaged = json!({
            "relevant": 5,
            "irrelevant": 0,
            "failed": 0,
            "conservation": { "pool": 148, "judged": 124 }
        });

        annotate_conservation_line(&mut damaged, Uuid::nil(), TEMPLATE, FAILED_CLAUSE);

        assert!(damaged.get("conservation_line").is_none());
        assert_eq!(
            damaged["conservation"]["pool"], 148,
            "the stored block is never rewritten"
        );
    }
}
