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

/// Add `conservation_line` to a stored summary, rendered from the stored template.
///
/// Mutates the caller's copy in place (never the row). Silent about summaries that
/// carry no conservation block — see the module doc for why that absence is the
/// honest answer rather than a zeroed line.
pub(crate) fn annotate_conservation_line(summary: &mut Value, run_id: Uuid, template: &str) {
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

    let line = render(
        template,
        &[
            ("pool", &counts.pool.to_string()),
            ("collapsed", &counts.collapsed.to_string()),
            ("excluded", &counts.excluded.to_string()),
            ("judged", &counts.judged.to_string()),
            ("relevant", &counts.relevant.to_string()),
        ],
    );

    if let Some(object) = summary.as_object_mut() {
        object.insert(CONSERVATION_LINE_KEY.to_string(), Value::String(line));
    }
}

/// The five numbers the sentence needs, read out of a stored summary.
struct LineCounts {
    pool: u64,
    collapsed: u64,
    excluded: u64,
    judged: u64,
    relevant: u64,
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
        // `relevant` lives on the summary itself, not in the block: it is an
        // OUTCOME of judging, while the block describes the input. The sentence
        // reads across both, which is the whole point of it.
        relevant: summary.get(RELEVANT_KEY).and_then(Value::as_u64)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// The seeded template, as the migration writes it.
    const TEMPLATE: &str = "{pool} gathered · {collapsed} duplicates folded · {excluded} \
                            set aside before judging · {judged} judged · {relevant} relevant";

    fn summary_with_conservation() -> Value {
        json!({
            "relevant": 22,
            "conservation": {
                "pool": 148,
                "excluded_empty": 2,
                "excluded_statement_type": 4,
                "excluded_too_short": 15,
                "duplicates_collapsed": 3,
                "judged": 124,
            }
        })
    }

    #[test]
    fn the_line_reports_the_runs_own_numbers_and_they_reconcile() {
        let mut summary = summary_with_conservation();

        annotate_conservation_line(&mut summary, Uuid::nil(), TEMPLATE);

        let line = summary["conservation_line"]
            .as_str()
            .expect("a line was added");
        assert!(line.contains("148 gathered"), "{line}");
        assert!(line.contains("3 duplicates folded"), "{line}");
        // 2 + 4 + 15 — the three reasons, totalled for the sentence.
        assert!(line.contains("21 set aside before judging"), "{line}");
        assert!(line.contains("124 judged"), "{line}");
        assert!(line.contains("22 relevant"), "{line}");
        // The reader can do the arithmetic the line invites: 148 = 21 + 3 + 124.
        assert_eq!(148, 21 + 3 + 124);
    }

    #[test]
    fn a_run_recorded_before_this_task_gets_no_line_rather_than_a_zeroed_one() {
        // The historical-run case, and the reason this is not `unwrap_or(0)`:
        // "0 gathered · 0 judged" would describe a scan that read nothing, which
        // is a claim about a July run nobody measured.
        let mut legacy = json!({ "relevant": 24, "candidates_read": 148 });

        annotate_conservation_line(&mut legacy, Uuid::nil(), TEMPLATE);

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
            "conservation": { "pool": 148, "judged": 124 }
        });

        annotate_conservation_line(&mut damaged, Uuid::nil(), TEMPLATE);

        assert!(damaged.get("conservation_line").is_none());
        assert_eq!(
            damaged["conservation"]["pool"], 148,
            "the stored block is never rewritten"
        );
    }
}
