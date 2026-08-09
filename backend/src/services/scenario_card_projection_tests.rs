//! Behaviour tests for the proposal projection.
//!
//! Every one of these asks a question about what a HUMAN would see, not about the
//! shape of a struct. They run without a database, a graph or a settings store,
//! which is the whole reason the precedence law lives in a pure function.

use super::*;

/// A relevant verdict row, with the payload a proposed card renders.
fn verdict(node: &str, role: &str, confidence: f32) -> RelevantVerdictRow {
    RelevantVerdictRow {
        graph_node_id: node.to_string(),
        proposed_role: Some(role.to_string()),
        confidence: Some(confidence),
        reason: Some(format!("the judge's reason for {node}")),
    }
}

fn quotes(pairs: &[(&str, &str)]) -> HashMap<String, String> {
    pairs
        .iter()
        .map(|(node, quote)| ((*node).to_string(), (*quote).to_string()))
        .collect()
}

fn ordinals(pairs: &[(&str, i32)]) -> HashMap<String, i32> {
    pairs
        .iter()
        .map(|(node, ordinal)| ((*node).to_string(), *ordinal))
        .collect()
}

fn ruled(nodes: &[&str]) -> HashSet<String> {
    nodes.iter().map(|n| (*n).to_string()).collect()
}

fn represented(groups: &[ProposalGroup]) -> Vec<&str> {
    groups.iter().map(|g| g.representative.as_str()).collect()
}

#[test]
fn a_ref_row_always_wins_over_a_verdict() {
    // R-a, in all three human-touched shapes. The third is the one the diagnostic
    // (§6, gap 1) found unprotected: a DEFERRED card is `undecided` + a reason, so
    // the merge SQL's `WHERE status = 'undecided'` guard let a re-merge silently
    // replace the model judgment on a card a human had deliberately parked. Here
    // the test is the PRESENCE of a row, not its status, so all three are equal.
    let verdicts = [
        verdict("included-node", "supports", 0.9),
        verdict("excluded-node", "contradicts", 0.8),
        verdict("deferred-node", "corroborates", 0.7),
        verdict("untouched-node", "supports", 0.6),
    ];
    let groups = project(
        &verdicts,
        &quotes(&[
            ("included-node", "one"),
            ("excluded-node", "two"),
            ("deferred-node", "three"),
            ("untouched-node", "four"),
        ]),
        // A row exists for each of the first three — included, dropped, and
        // undecided-with-a-defer-reason all look the same to precedence.
        &ruled(&["included-node", "excluded-node", "deferred-node"]),
        &ordinals(&[]),
    );

    assert_eq!(
        represented(&groups),
        vec!["untouched-node"],
        "only the node no human has touched may be proposed"
    );
    // And the ruled nodes are not hiding inside somebody else's covers list, where
    // a ruling would silently re-write them.
    let touched = ruled(&["included-node", "excluded-node", "deferred-node"]);
    for group in &groups {
        assert!(
            !group.covers.iter().any(|n| touched.contains(n)),
            "a human-touched node must not be covered by another card's ruling: {:?}",
            group.covers
        );
    }
}

#[test]
fn a_rescan_never_alters_a_ruled_or_deferred_card() {
    // §6's guarantee, made a guard rather than an absence. The new run judged the
    // deferred card differently — a different role and a much higher score — and
    // the projection must carry NONE of that onto the card, because the human
    // already parked it with a reason.
    let first_pass = [verdict("deferred-node", "corroborates", 0.55)];
    let rescan = [verdict("deferred-node", "rebuts", 0.99)];

    let before = project(
        &first_pass,
        &quotes(&[("deferred-node", "same quote")]),
        &ruled(&[]),
        &ordinals(&[]),
    );
    assert_eq!(before.len(), 1, "unruled, so the first pass proposes it");

    let after = project(
        &rescan,
        &quotes(&[("deferred-node", "same quote")]),
        // The human deferred it between the two runs.
        &ruled(&["deferred-node"]),
        &ordinals(&[]),
    );
    assert!(
        after.is_empty(),
        "a re-scan must not re-propose or re-judge a card a human has ruled: {after:?}"
    );
}

#[test]
fn proposed_count_is_admitted_minus_ruled() {
    // R-e. The number beside the run's frozen counts is derived, not stored: five
    // admitted verdicts, two of them already ruled, and two of the survivors are
    // byte-identical twins that the human rules once.
    let verdicts = [
        verdict("a", "supports", 0.9),
        verdict("b", "supports", 0.9),
        verdict("twin-1", "supports", 0.8),
        verdict("twin-2", "supports", 0.8),
        verdict("c", "contradicts", 0.7),
    ];
    let groups = project(
        &verdicts,
        &quotes(&[
            ("a", "alpha"),
            ("b", "beta"),
            ("twin-1", "identical sentence"),
            ("twin-2", "identical sentence"),
            ("c", "gamma"),
        ]),
        &ruled(&["a", "b"]),
        &ordinals(&[("twin-1", 45), ("twin-2", 46), ("c", 91)]),
    );

    assert_eq!(
        groups.len(),
        2,
        "five admitted − two ruled = three nodes, folded into two cards: {:?}",
        represented(&groups)
    );
    let covered: usize = groups.iter().map(|g| g.covers.len()).sum();
    assert_eq!(
        covered, 3,
        "every surviving node is settled by exactly one card"
    );
}

#[test]
fn one_ruling_covers_a_byte_identical_twin() {
    // Architect ruling R2: the twins fold, the badge says what it covers, and the
    // ruling settles both — the human's keystroke is de-duplicated, not the record.
    let verdicts = [
        verdict("node-46", "supports", 0.85),
        verdict("node-45", "supports", 0.85),
    ];
    let groups = project(
        &verdicts,
        &quotes(&[
            ("node-45", "the letter proposed the property be divided"),
            ("node-46", "the letter proposed the property be divided"),
        ]),
        &ruled(&[]),
        &ordinals(&[("node-45", 45), ("node-46", 46)]),
    );

    assert_eq!(groups.len(), 1, "one quote, one card");
    assert_eq!(
        groups[0].representative, "node-45",
        "the lowest C-ordinal carries the card, so the badge always reads the same \
         way round however the verdict rows arrive"
    );
    assert_eq!(
        groups[0].covers,
        vec!["node-45".to_string(), "node-46".to_string()],
        "the ruling settles both members, representative first"
    );

    // And the reverse lookup the ruling path uses finds the group from EITHER node.
    let index = index_by_covered_node(&groups);
    assert_eq!(
        index.get("node-46").map(|g| g.representative.as_str()),
        Some("node-45")
    );
    assert_eq!(
        index.get("node-45").map(|g| g.representative.as_str()),
        Some("node-45")
    );
}

#[test]
fn a_twin_whose_partner_was_already_ruled_stands_on_its_own() {
    // The historical case: an older merge put a ref row on one twin and not the
    // other. Precedence removes the ruled one, and what is left is a single-node
    // proposal — NOT a card claiming to cover a node somebody has already decided.
    let verdicts = [
        verdict("node-45", "supports", 0.85),
        verdict("node-46", "supports", 0.85),
    ];
    let groups = project(
        &verdicts,
        &quotes(&[("node-45", "identical"), ("node-46", "identical")]),
        &ruled(&["node-45"]),
        &ordinals(&[("node-45", 45), ("node-46", 46)]),
    );

    assert_eq!(represented(&groups), vec!["node-46"]);
    assert_eq!(
        groups[0].covers,
        vec!["node-46".to_string()],
        "the ruled twin is not dragged back into a ruling it has already had"
    );
}

#[test]
fn a_quoteless_verdict_is_never_folded_with_another() {
    // Two candidates with no quote text are not "the same statement" — they are two
    // statements nothing can key on. Folding them would put one ruling on two
    // unrelated items, which is the single mistake this design must make
    // impossible.
    let verdicts = [
        verdict("silent-a", "supports", 0.5),
        verdict("silent-b", "supports", 0.5),
    ];
    let groups = project(
        &verdicts,
        &quotes(&[("silent-a", "   ")]), // whitespace-only, and silent-b has no entry
        &ruled(&[]),
        &ordinals(&[]),
    );

    assert_eq!(
        groups.len(),
        2,
        "no quote is not a fold key: {:?}",
        represented(&groups)
    );
    assert!(groups.iter().all(|g| g.covers.len() == 1));
}

#[test]
fn deleting_a_run_removes_its_unruled_proposals_and_keeps_rulings() {
    // R-d, as amended by architect ruling R1. Deleting a run cascades its verdict
    // rows away, so the projection is handed nothing — and because a proposal was
    // never a stored row, nothing has to be cleaned up. The rulings are untouched
    // for the structural reason this whole module exists: projection never writes,
    // so it cannot un-write.
    //
    // (The delete itself is guarded: a run any ruling CITES is refused with a 409,
    // which is `theme_scan_run`'s test. This is the read half — what the queue
    // shows the moment a deletable run is gone.)
    let ruled_before = ruled(&["ruled-node"]);

    let after_delete = project(
        // The run is gone, so it has no verdicts to offer.
        &[],
        &quotes(&[("ruled-node", "one"), ("unruled-node", "two")]),
        &ruled_before,
        &ordinals(&[]),
    );

    assert!(
        after_delete.is_empty(),
        "a deleted run proposes nothing: {after_delete:?}"
    );
    assert!(
        ruled_before.contains("ruled-node"),
        "the human's ruling is a stored row and the projection never touches it"
    );
}

#[test]
fn the_judges_own_words_reach_the_card_unchanged() {
    // The proposal carries the reason the human is being asked to trust. A card
    // that showed a role and a band with no justification would be the C-222 shape
    // §7 exists to forbid.
    let verdicts = [verdict("node-14", "supports", 0.91)];
    let groups = project(
        &verdicts,
        &quotes(&[("node-14", "admitted")]),
        &ruled(&[]),
        &ordinals(&[]),
    );

    assert_eq!(groups[0].role.as_deref(), Some("supports"));
    assert_eq!(
        groups[0].reason.as_deref(),
        Some("the judge's reason for node-14")
    );
    assert_eq!(groups[0].confidence, Some(0.91));
}

// ── The proposal as the card shows it ───────────────────────────────────────
//
// `project` decides WHAT is proposed; `to_card_proposal` decides what that looks
// like on screen. The second is where the stored templates are filled, and a
// template filled wrongly is a defect no amount of correct projection hides —
// these tests exist because the first version of this code passed `"{verb}"`
// where `render` wanted `verb`, and would have shipped a literal "Scan: {verb}"
// chip on every proposed card on DEV.

use crate::domain::wording::Wording;

fn wording() -> Wording {
    Wording::for_test()
}

#[test]
fn the_role_chip_names_the_scan_as_the_speaker() {
    // §7.8's discipline, applied to the role: the chip must not read as the
    // RECORD's stance. "Scan: supports" beside a sworn admission is a claim about
    // a model's opinion; a bare "supports" is a claim about the evidence, and they
    // are not the same sentence.
    let group = ProposalGroup {
        representative: "ev-1".to_string(),
        covers: vec!["ev-1".to_string()],
        role: Some("supports".to_string()),
        confidence: Some(0.9),
        reason: Some("direct admission".to_string()),
    };

    let proposal = to_card_proposal(&group, &ordinals(&[]), &wording());

    assert_eq!(proposal.role_label.as_deref(), Some("Scan: supports"));
    assert!(
        !proposal
            .role_label
            .as_deref()
            .unwrap_or_default()
            .contains('{'),
        "an unfilled placeholder reached the card: {:?}",
        proposal.role_label
    );
    // The judge's SENTENCE is deliberately not here any more (ruling R3): it is
    // the card's, so that including the card does not delete it. What a proposal
    // carries is the badge vocabulary — the role chip and the covers count.
}

#[test]
fn a_role_this_build_cannot_name_silences_the_chip_rather_than_leaking_the_token() {
    // Reachable: a scan version that writes a new role token before the backend
    // understands it. Printing "corroborates" raw is the C-222 leak §7.5 forbids,
    // so the chip goes — and the miss is logged (see `decode_role`), because a
    // suppressed chip and a verdict that carried no role look identical on screen.
    let group = ProposalGroup {
        representative: "ev-1".to_string(),
        covers: vec!["ev-1".to_string()],
        role: Some("insinuates".to_string()),
        confidence: Some(0.9),
        reason: Some("the judge's reason".to_string()),
    };

    let proposal = to_card_proposal(&group, &ordinals(&[]), &wording());

    assert!(proposal.role_label.is_none(), "no chip for an unnamed role");
    // The card stays RULABLE: the reason and the band both still reach the human,
    // so a vocabulary gap does not withhold a candidate from the queue. Both now
    // travel outside `CardProposal` — the reason on the card (R3) and the band
    // from the verdict's score — so what this asserts is that suppressing the
    // chip suppressed ONLY the chip.
    assert_eq!(proposal.duplicate_count, group.covers.len());
}

#[test]
fn the_covers_badge_names_the_twins_and_not_the_card_it_sits_on() {
    // The human is reading C-45's card; telling them it covers C-45 says nothing.
    // What they need is that ruling here also settles C-46.
    let group = ProposalGroup {
        representative: "node-45".to_string(),
        covers: vec!["node-45".to_string(), "node-46".to_string()],
        role: Some("supports".to_string()),
        confidence: Some(0.85),
        reason: None,
    };

    let proposal = to_card_proposal(
        &group,
        &ordinals(&[("node-45", 45), ("node-46", 46)]),
        &wording(),
    );

    assert_eq!(proposal.duplicate_count, 2);
    assert_eq!(
        proposal.duplicate_label.as_deref(),
        Some("×2 — covers C-46")
    );
}

#[test]
fn a_card_that_speaks_only_for_itself_carries_no_badge() {
    // "×1" on every card in the queue is noise, and noise on the surface whose
    // whole job is to be readable at speed.
    let group = ProposalGroup {
        representative: "ev-1".to_string(),
        covers: vec!["ev-1".to_string()],
        role: Some("supports".to_string()),
        confidence: Some(0.9),
        reason: None,
    };

    let proposal = to_card_proposal(&group, &ordinals(&[("ev-1", 1)]), &wording());

    assert_eq!(proposal.duplicate_count, 1);
    assert!(proposal.duplicate_label.is_none());
}

#[test]
fn an_unnumbered_twin_is_counted_but_never_given_an_invented_code() {
    // A twin gather has not numbered yet still gets settled by the ruling, so the
    // COUNT must include it — but the badge must not invent a handle for it. "×2 —
    // covers " with an empty list is honest; "covers C-0" is a card that does not
    // exist.
    let group = ProposalGroup {
        representative: "node-45".to_string(),
        covers: vec!["node-45".to_string(), "unnumbered".to_string()],
        role: Some("supports".to_string()),
        confidence: Some(0.85),
        reason: None,
    };

    let proposal = to_card_proposal(&group, &ordinals(&[("node-45", 45)]), &wording());

    assert_eq!(proposal.duplicate_count, 2, "the ruling still settles both");
    let label = proposal
        .duplicate_label
        .expect("a folded card carries a badge");
    assert!(label.contains("×2"), "the count is real: {label}");
    assert!(!label.contains("C-0"), "no invented handle: {label}");
}
