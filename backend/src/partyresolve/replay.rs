//! The pure half of `verify_party_resolution`: classify each mention's outcome
//! and render the operator-facing report.
//!
//! Split from the binary for the reason the rest of the one-shot family is split
//! (`remap::plan`, `rekey::plan`): everything here is a decision about data, so
//! it is testable without a database, a graph, or a reprocess. The binary keeps
//! only what genuinely needs two connections.

use std::fmt::Write as _;

use crate::api::pipeline::party_alias::{AliasLookup, PartyAliasIndex};

/// One mention, resolved three ways.
pub struct Replay {
    pub document: String,
    pub surface: String,
    /// What `extraction_items.neo4j_node_id` carries right now — the last real
    /// ingest's decision, as later amended by `merge_parties`.
    pub stored: String,
    /// What the resolver decides TODAY, without the alias stage. Produced by the
    /// real `resolve_parties` fed a party list whose aliases have been stripped,
    /// so the alias index is empty and the second stage can never fire. That is
    /// the pre-fix code path exactly, not a model of it.
    pub today: String,
    /// What the resolver decides with the alias stage live.
    pub with_fix: String,
    pub via: &'static str,
}

/// Name the route a mention took, for the `via` column.
pub fn classify(
    index: &PartyAliasIndex,
    entity_type: &str,
    surface: &str,
    today: &str,
    with_fix: &str,
    existing_ids: &[String],
) -> &'static str {
    let landed_on_existing = existing_ids.iter().any(|id| id == with_fix);
    if landed_on_existing && today == with_fix {
        // Already attached to this node before the fix — the canonical name
        // matched, exactly as it does today.
        return "name";
    }
    match index.lookup(entity_type, surface) {
        AliasLookup::Matched(id) if id == with_fix => "alias",
        AliasLookup::Matched(_) => "name",
        AliasLookup::Ambiguous(_) => "ambiguous",
        AliasLookup::Stoplisted => "stoplist",
        AliasLookup::NoMatch => {
            if landed_on_existing {
                "name"
            } else {
                "new"
            }
        }
    }
}

pub fn count_regressions(replays: &[Replay], existing_ids: &[String]) -> usize {
    let is_existing = |id: &str| existing_ids.iter().any(|e| e == id);
    replays
        .iter()
        .filter(|r| {
            // Was attached to a real node and now is not, OR moved to a different
            // real node. Both are regressions; neither may be non-zero.
            (is_existing(&r.today) && !is_existing(&r.with_fix))
                || (is_existing(&r.today) && is_existing(&r.with_fix) && r.today != r.with_fix)
        })
        .count()
}

/// The per-mention list and the summary.
pub fn render(replays: &[Replay], index: &PartyAliasIndex, existing_ids: &[String]) -> String {
    let is_existing = |id: &str| existing_ids.iter().any(|e| e == id);
    let mut out = String::new();
    let _ = writeln!(out, "=== PARTY RESOLUTION REPLAY — READ-ONLY ===\n");
    let _ = writeln!(
        out,
        "Every party mention in the corpus, resolved by the REAL resolve_parties()."
    );
    let _ = writeln!(
        out,
        "`today` is the id the row already carries from the last real ingest.\n"
    );

    let mut current = "";
    for r in replays {
        if r.document != current {
            current = &r.document;
            let _ = writeln!(out, "\n--- {current} ---");
        }
        let moved = if r.today == r.with_fix {
            ""
        } else {
            "   <-- CHANGED"
        };
        let drift = if r.stored == r.with_fix {
            ""
        } else {
            "   [stored differs]"
        };
        let _ = writeln!(
            out,
            "  {:<30} today -> {:<30} with fix -> {:<30} via {}{}{}",
            format!("{:?}", r.surface),
            r.today,
            r.with_fix,
            r.via,
            moved,
            drift
        );
    }

    let identical = replays.iter().filter(|r| r.today == r.with_fix).count();
    let now_existing = replays
        .iter()
        .filter(|r| !is_existing(&r.today) && is_existing(&r.with_fix))
        .count();
    let now_new = replays
        .iter()
        .filter(|r| is_existing(&r.today) && !is_existing(&r.with_fix))
        .count();
    let moved: Vec<&Replay> = replays
        .iter()
        .filter(|r| is_existing(&r.today) && is_existing(&r.with_fix) && r.today != r.with_fix)
        .collect();
    let ambiguous = replays.iter().filter(|r| r.via == "ambiguous").count();
    let stoplisted = replays.iter().filter(|r| r.via == "stoplist").count();

    let _ = writeln!(out, "\n\n=== SUMMARY ===\n");
    let _ = writeln!(out, "Mentions walked            : {}", replays.len());
    let _ = writeln!(out, "Resolve identically        : {identical}");
    let _ = writeln!(
        out,
        "Now resolve to an existing : {now_existing}   (were NEW or a variant node)   <- the win"
    );
    let _ = writeln!(
        out,
        "Now NEW (were attached)    : {now_new}   <- MUST be zero"
    );
    let _ = writeln!(
        out,
        "Moved to a DIFFERENT node  : {}   <- MUST be zero",
        moved.len()
    );
    let _ = writeln!(out, "Blocked by ambiguity       : {ambiguous}");
    let _ = writeln!(out, "Blocked by stoplist        : {stoplisted}");

    // A separate question from the fix's effect: where would the NEXT ingest put
    // a mention that `merge_parties` has already repointed? Any row here is a
    // merge the pipeline is about to undo, and it is the reason the Step-2 block
    // re-runs the merge before re-ingesting.
    let undo: Vec<&Replay> = replays
        .iter()
        .filter(|r| is_existing(&r.stored) && r.stored != r.with_fix)
        .collect();
    let _ = writeln!(
        out,
        "\nStored id differs from with-fix : {}   (merge work the next ingest would undo)",
        undo.len()
    );
    for r in &undo {
        let _ = writeln!(
            out,
            "  {} {:?} stored {} -> would become {}",
            r.document, r.surface, r.stored, r.with_fix
        );
    }

    for r in &moved {
        let _ = writeln!(
            out,
            "\n  REGRESSION: {} {:?} {} -> {}",
            r.document, r.surface, r.today, r.with_fix
        );
    }

    let ambiguous_keys = index.ambiguous_keys();
    let _ = writeln!(
        out,
        "\n\n=== STRINGS CLAIMED BY MORE THAN ONE NODE ({}) ===",
        ambiguous_keys.len()
    );
    let _ = writeln!(
        out,
        "These never resolve a mention. Merge the nodes to make them bind again.\n"
    );
    for a in &ambiguous_keys {
        let _ = writeln!(
            out,
            "  [{}] {:?} claimed by {:?}",
            a.entity_type, a.key, a.node_ids
        );
    }

    out
}

#[cfg(test)]
#[path = "replay_tests.rs"]
mod tests;
