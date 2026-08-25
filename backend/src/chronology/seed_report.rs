//! The count proof the one-shot prints and writes to a file.
//!
//! Separate from the plan so the rendering can be asserted verbatim in a test —
//! the report IS the artifact Roman reads before typing `--apply`, so a change
//! to its wording is a change a test should notice.
//!
//! Plain text, not a table library: it is read in a terminal and pasted into a
//! runbook, and both want fixed-width lines with nothing to render.

use super::seed::{SeedPlan, NO_DOCUMENT_YET, REPOINT_MAP, SEED_PRECISION, SEED_SOURCE};

/// Render the whole proof: the map, every row, and the totals.
///
/// `applied` switches the tense — the same numbers describe what WOULD be
/// written and what WAS written, and printing "would write" after a successful
/// `--apply` would be a small lie that costs an operator real confusion.
pub fn render_report(plan: &SeedPlan, case_id: &str, created_by: &str, applied: bool) -> String {
    let mut out = String::new();
    out.push_str("=== CHRONOLOGY SEED — one-shot ===\n\n");
    out.push_str(&format!("case_id    : {case_id}\n"));
    out.push_str(&format!("created_by : {created_by}\n"));
    out.push_str(&format!(
        "precision  : {SEED_PRECISION} (every seeded event)\n"
    ));
    out.push_str(&format!("source tag : attributes.source = {SEED_SOURCE}\n"));
    out.push_str(&format!(
        "mode       : {}\n\n",
        if applied {
            "APPLIED"
        } else {
            "DRY RUN — nothing written"
        }
    ));

    out.push_str(&render_repoint_map());
    out.push_str(&render_rows(plan));
    out.push_str(&render_totals(plan, applied));
    out
}

/// The re-point map, quoted in full so it can be eyeballed before `--apply`.
fn render_repoint_map() -> String {
    let mut out = String::from("--- DOCUMENT RE-POINT MAP (design R12) ---\n");
    for (from, to) in REPOINT_MAP {
        if from == to {
            out.push_str(&format!(
                "  {from}\n      → itself (already a real document)\n"
            ));
        } else {
            out.push_str(&format!("  {from}\n      → {to}\n"));
        }
    }
    out.push_str("\n--- NO DOCUMENT EXISTS (event seeded, NO link row written) ---\n");
    for id in NO_DOCUMENT_YET {
        out.push_str(&format!("  {id}\n"));
    }
    out.push('\n');
    out
}

/// Every row the plan would write, one event per block.
fn render_rows(plan: &SeedPlan) -> String {
    let mut out = String::from("--- EVERY ROW ---\n");
    for event in &plan.events {
        let approx = if event.approximate { " ~approx" } else { "" };
        out.push_str(&format!(
            "  {}  {}{}  {}\n",
            event.source_id, event.event_date, approx, event.title
        ));
        out.push_str(&format!("        attributes: {}\n", event.attributes));
        match (&event.link, &event.unlinkable_target) {
            (Some(link), _) => {
                let changed = if link.original_target_id == link.target_id {
                    "unchanged"
                } else {
                    "RE-POINTED"
                };
                out.push_str(&format!(
                    "        link ({changed}): {} [{}]\n",
                    link.target_id,
                    link.label.as_deref().unwrap_or("no label")
                ));
            }
            (None, Some(target)) => {
                out.push_str(&format!(
                    "        NO LINK — no document exists for {target}\n"
                ));
            }
            (None, None) => {
                out.push_str("        no document reference in the source\n");
            }
        }
    }
    out.push('\n');
    out
}

/// The totals a runbook step reads.
fn render_totals(plan: &SeedPlan, applied: bool) -> String {
    let verb = if applied { "written" } else { "to write" };
    let mut out = String::from("--- COUNT PROOF ---\n");
    out.push_str(&format!("  events {verb}      : {}\n", plan.events.len()));
    out.push_str(&format!("  link rows {verb}   : {}\n", plan.link_count()));
    out.push_str(&format!(
        "  distinct targets   : {}\n",
        plan.target_ids().len()
    ));
    out.push_str(&format!(
        "  events with no document yet : {}\n",
        plan.unlinkable().len()
    ));
    out
}

#[cfg(test)]
#[path = "seed_report_tests.rs"]
mod tests;
