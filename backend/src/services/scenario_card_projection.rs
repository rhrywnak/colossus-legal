//! Proposals as a READ-TIME PROJECTION — the precedence law, in one pure place.
//!
//! ## What a "proposal" is, and what it is not
//!
//! §6 says a scan "proposes candidates as pending rulings". Until this module it
//! proposed nothing: a human had to tick findings in a report and press Merge
//! before the queue could see a single verdict (the select-twice defect, measured
//! in `CC_REPORT_SCAN_WORKFLOW_DIAGNOSTIC.md`). Now the queue simply READS the
//! latest completed run's admitted verdicts and renders them as PROPOSED cards.
//!
//! Nothing here writes. A proposal is a VIEW over scan output — the same kind of
//! fact as C6–C9, which are computed on read and never stored as truth — and the
//! human's ruling remains the only writer into a scenario's working state.
//!
//! ## The three laws this module IS
//!
//! * **R-a — a ref row always wins.** Any node a human has touched (included,
//!   excluded, or deferred = undecided + a reason) is never re-proposed and never
//!   altered. Precedence is explicit here because a projection has no `ON CONFLICT
//!   … WHERE status = 'undecided'` clause to lean on: the merge SQL's single
//!   `WHERE` used to be the whole of the status-preserving law, and it did NOT
//!   protect a deferred card (diagnostic §6, gap 1). Here it does — presence of a
//!   row is the test, not its status.
//! * **R-b — one run projects.** Enforced upstream by the query
//!   (`scan_run_projection`); this module never unions two runs because it is only
//!   ever handed one run's verdicts.
//! * **R-e — proposed = admitted minus ruled.** Not a stored counter: the number
//!   falls out of the fold below and is served beside the run's frozen counts,
//!   labelled live.
//!
//! ## Why the fold lives here and not in SQL
//!
//! The scan writes one verdict row per node, twins included, on purpose — the
//! scorecard joins Roman's ledger to `scan_run_verdicts` on `graph_node_id`, and a
//! twin with no row would read as a statement the scan LOST. So the audit trail is
//! per node and the SURFACE is per group, and something has to bridge them. The
//! bridge is the quote text — the same key the pre-filter collapses on
//! (`theme_scan_prefilter::collapse_exact_duplicates`) — and quote text lives in
//! the graph, not in Postgres. Hence: pure function, quotes passed in.
//!
//! Ruling a folded card settles every member (architect ruling R2, 2026-08-08),
//! which is why [`ProposalGroup::covers`] is the group's whole surviving
//! membership and not a display badge. Each member still gets its own ref row and
//! its own ledger anchor, so a repeated sentence keeps its own document and page
//! and lands at each of its appearances on the timeline; only the human's
//! keystroke is de-duplicated.
//!
//! ## "Pure" here means no I/O, not no LOG
//!
//! Nothing in this module reads a database, a graph, a clock or a settings store —
//! which is what lets the precedence law be a unit test rather than a promise. It
//! does emit ONE `tracing::warn!`, in `decode_role`, and that is deliberate: a role
//! token this build cannot name suppresses a chip, and a suppressed chip is
//! indistinguishable on screen from a verdict that carried no role at all
//! (Standing Rule 1). Logging is not I/O for the purposes that make this module
//! testable — the tests below run unchanged with no subscriber installed.

use std::collections::{HashMap, HashSet};

use crate::domain::card_language::stance_verb;
use crate::domain::fact_role::FactRole;
use crate::domain::scenario_code::candidate_code;
use crate::domain::wording::Wording;
use crate::domain::wording_templates::render;
use crate::dto::scenario_card::CardProposal;
use crate::repositories::pipeline_repository::RelevantVerdictRow;

/// One proposal as the surface shows it: a representative card that speaks for
/// every byte-identical twin folded into it.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ProposalGroup {
    /// The node whose card carries this proposal.
    pub representative: String,
    /// Every node this ONE ruling settles — the representative first, then its
    /// surviving twins. Always non-empty, so no caller needs a special case.
    pub covers: Vec<String>,
    /// The role the judge assigned, as its stored token.
    pub role: Option<String>,
    /// The judge's self-reported score. Banded before it reaches a card and NEVER
    /// serialized raw (§7.8 — "never a naked percentage").
    pub confidence: Option<f32>,
    /// The judge's justification, verbatim from the verdict row.
    pub reason: Option<String>,
}

/// Fold one run's admitted verdicts into the proposals the queue should show.
///
/// Returns one group per surviving proposal, keyed for the caller by
/// [`ProposalGroup::representative`]. A node the human has already touched appears
/// in NO group — neither as a representative nor inside a `covers` list — which is
/// R-a made structural rather than remembered.
///
/// * `verdicts` — the projecting run's `relevant = true` rows, one per node.
/// * `quotes` — `graph_node_id → verbatim quote`, the fold key. A node missing
///   from this map cannot be folded and stands alone (see below).
/// * `ruled` — every node that HAS a `scenario_fact_refs` row, whatever its status.
/// * `ordinals` — `graph_node_id → C-ordinal`, used only to choose which member
///   of a group is the representative.
///
/// ## Why a node with no quote is never folded
///
/// An absent or empty quote is not a fold key — collapsing every quote-less node
/// into one group would merge unrelated statements under a single ruling, which is
/// the one mistake this whole design exists to make impossible. Such nodes stand
/// alone, exactly as they do today. (The pre-filter sets aside empty quotes before
/// judging, so a projected node with no quote is already an edge case; it degrades
/// to one-card-one-ruling rather than to something wrong.)
pub(crate) fn project(
    verdicts: &[RelevantVerdictRow],
    quotes: &HashMap<String, String>,
    ruled: &HashSet<String>,
    ordinals: &HashMap<String, i32>,
) -> Vec<ProposalGroup> {
    // R-a first, before anything is grouped. Dropping ruled nodes up front is what
    // makes the fold honest: a group whose representative was ruled last week must
    // not carry its un-ruled twin as a "covered" node nobody can see, and a group
    // whose every member is ruled must vanish rather than render as an empty card.
    let admitted: Vec<&RelevantVerdictRow> = verdicts
        .iter()
        .filter(|v| !ruled.contains(&v.graph_node_id))
        .collect();

    // ## Rust Learning: an index that borrows its keys from the data being walked
    //
    // The map holds `&str` keys borrowed from `quotes`, not owned `String` copies.
    // That is allowed because `quotes` outlives this function's locals and is only
    // READ here — the borrow checker's whole question is "does the thing being
    // pointed at outlive the pointer?", and here it plainly does. The alternative
    // (cloning every quote into the key) would copy a page of text per candidate to
    // build an index thrown away three lines later. `collapse_exact_duplicates`
    // makes the same trade in the other direction (owned keys) because THERE the
    // source vector is being consumed while the index is alive.
    let mut by_quote: HashMap<&str, usize> = HashMap::new();
    let mut groups: Vec<ProposalGroup> = Vec::new();

    for verdict in admitted {
        let quote = quotes
            .get(&verdict.graph_node_id)
            .map(String::as_str)
            .filter(|q| !q.trim().is_empty());

        // An existing group for this exact quote: fold in as a covered twin.
        if let Some(position) = quote.and_then(|q| by_quote.get(q).copied()) {
            groups[position].covers.push(verdict.graph_node_id.clone());
            continue;
        }

        if let Some(q) = quote {
            by_quote.insert(q, groups.len());
        }
        groups.push(ProposalGroup {
            representative: verdict.graph_node_id.clone(),
            covers: vec![verdict.graph_node_id.clone()],
            role: verdict.proposed_role.clone(),
            confidence: verdict.confidence,
            reason: verdict.reason.clone(),
        });
    }

    for group in &mut groups {
        elect_representative(group, ordinals);
    }
    groups
}

/// Put the lowest-numbered member first and make it the card that carries the
/// proposal.
///
/// ## Why the lowest C-ordinal rather than "whichever arrived first"
///
/// The badge says "×2 — covers C-46", and a human reading it will look for C-45.
/// If the representative were decided by query order, the same pair could render
/// as "C-46 covers C-45" after an unrelated change to the verdict read — the card
/// would move without anything having happened. The ordinal is stable for the life
/// of the candidate (it is minted once and never changes), so electing on it makes
/// the pairing stable too. Un-numbered members sort last, and the node id breaks a
/// tie so the choice is total rather than dependent on hash order.
fn elect_representative(group: &mut ProposalGroup, ordinals: &HashMap<String, i32>) {
    group.covers.sort_by_key(|node| {
        let ordinal = ordinals.get(node).copied();
        (ordinal.is_none(), ordinal, node.clone())
    });
    // `covers` is never empty — every group is created with its own representative
    // in it — so the first element is the elected one.
    if let Some(first) = group.covers.first() {
        group.representative = first.clone();
    }
}

/// Turn one group into the display-ready proposal a card carries.
///
/// Pure, and composed HERE rather than in `build_card` for one reason: the badge
/// names the OTHER cards a ruling settles, which needs the whole scenario's
/// ordinal index — a per-card builder has only its own number.
///
/// ## Domain note: an unmappable role is a silent chip, never a raw token
///
/// `proposed_role` is a stored string, and a build that cannot name it must not
/// print it: "corroborates" is precisely the C-222 leak §7.5 forbids, and a token
/// this build does not define could be anything at all. The chip is then absent
/// and the caller logs the token — the reason and the band still reach the human,
/// so the card stays rulable rather than being withheld over a vocabulary gap.
pub(crate) fn to_card_proposal(
    group: &ProposalGroup,
    ordinals: &HashMap<String, i32>,
    wording: &Wording,
) -> CardProposal {
    let role_label = group.role.as_deref().and_then(decode_role).map(|role| {
        // NOTE the key has NO braces: `render` finds `{verb}` in the template and
        // looks up `verb`. Passing `"{verb}"` matched nothing and emitted the token
        // verbatim — a literal "Scan: {verb}" chip on every proposed card. The
        // direct test below is what caught it, and is why it exists.
        render(
            &wording.card_proposed_role_template,
            &[("verb", stance_verb(role))],
        )
    });

    CardProposal {
        role_label,
        duplicate_count: group.covers.len(),
        duplicate_label: covers_label(group, ordinals, wording),
    }
}

/// The stored role token as a [`FactRole`], or `None` for a token this build does
/// not define.
///
/// ## Rust Learning: a lookup over the vocabulary's own `ALL` slice
///
/// `FactRole` deliberately has no `From<&str>` — the sanctioned decode is serde,
/// which is where the LOUD parse failure lives for the model's reply. Here the
/// value is already stored, so the question is milder ("can this build name it?")
/// and the answer comes from the same `ALL` list every other enumerating caller
/// reads. Matching on string literals instead would be a second copy of the
/// vocabulary, which is what `ALL` exists to prevent.
///
/// ## Why the miss is LOGGED and not merely suppressed (Standing Rule 1)
///
/// Suppressing the chip is the right screen behaviour — a raw token is the C-222
/// leak §7.5 forbids — but it is the WRONG log behaviour, because "the model
/// assigned no role" and "the model assigned a role this build cannot name" then
/// look identical to an operator, and only the second is a deploy problem. It is
/// reachable: a scan version that writes a new role token before the backend
/// understands it would empty the chip on every affected card silently. A `warn`
/// rather than an `error` because the card is still rulable — the reason and the
/// band both reach the human — so this is a vocabulary gap to fix, not a failure
/// to stop for.
fn decode_role(token: &str) -> Option<FactRole> {
    let role = FactRole::ALL.iter().copied().find(|r| r.code() == token);
    if role.is_none() {
        tracing::warn!(
            %token,
            known = ?FactRole::ALL.iter().map(|r| r.code()).collect::<Vec<_>>(),
            "a stored scan verdict carries a role token this build does not define; \
             the proposed-role chip is suppressed on its card"
        );
    }
    role
}

/// The "×2 — covers C-46" badge, or `None` for a card that speaks only for itself.
///
/// The codes listed are the covered TWINS, not the representative: the human is
/// reading C-45's card, and telling them it covers C-45 would say nothing. A twin
/// with no ordinal yet contributes nothing to the list rather than an invented
/// handle — but it is still counted, because the ruling still settles it.
fn covers_label(
    group: &ProposalGroup,
    ordinals: &HashMap<String, i32>,
    wording: &Wording,
) -> Option<String> {
    if group.covers.len() < 2 {
        return None;
    }
    let codes: Vec<String> = group
        .covers
        .iter()
        .filter(|node| **node != group.representative)
        .filter_map(|node| ordinals.get(node).copied().map(candidate_code))
        .collect();

    // Placeholder NAMES, unbraced — see the note in `to_card_proposal`.
    Some(render(
        &wording.card_proposed_covers_template,
        &[
            ("count", &group.covers.len().to_string()),
            ("codes", &codes.join(", ")),
        ],
    ))
}

/// Index the groups by every node they speak for.
///
/// The ruling path needs the opposite lookup from the card path: given the node a
/// human just pressed I on, WHICH nodes does that ruling settle? Building it here
/// (rather than in the caller) keeps "a covered twin is settled by its
/// representative's ruling" a property of this module, where the fold that created
/// the relationship lives.
pub(crate) fn index_by_covered_node(groups: &[ProposalGroup]) -> HashMap<&str, &ProposalGroup> {
    let mut index = HashMap::new();
    for group in groups {
        for node in &group.covers {
            index.insert(node.as_str(), group);
        }
    }
    index
}

#[cfg(test)]
#[path = "scenario_card_projection_tests.rs"]
mod tests;
