//! Unit tests for [`super`] — the card payload assembly (task 1.2, v2 §7).
//!
//! The headline test is `every_section_seven_element_is_present_on_a_complete_card`:
//! §7 makes a card missing any element a DEFECT, so the contract is the test. The
//! rest cover the defer detection (the July failure), the vocabulary law, and the
//! quote-in-context windowing.
//!
//! All of it is pure — the assembly takes data and returns a payload, so no
//! database or graph is needed to prove the contract holds.

use super::*;

/// The seeded snapshot — band cutoffs 0.80/0.50, context window 240. The
/// assertions below are written against those, the same values the migration
/// stores and the product ships with.
fn settings() -> Settings {
    Settings::for_test()
}
use crate::bias::dto::{ActorOption, DocumentRef};
use crate::dto::scenario_card::{CardHumanLink, QuestionSource};
use crate::repositories::scenario_card_repository::CardExtrasRow;
use chrono::{DateTime, Utc};

/// A candidate with everything the record can carry.
fn full_instance() -> BiasInstance {
    BiasInstance {
        evidence_id: "ev-1".to_string(),
        title: "Interrogatory answer 14".to_string(),
        verbatim_quote: Some("I do not recall that meeting.".to_string()),
        question: Some("Did you attend the meeting on March 3, 2019?".to_string()),
        page_number: Some(14),
        pattern_tags: Vec::new(),
        stated_by: Some(ActorOption {
            id: "actor-1".to_string(),
            name: "R. Phillips".to_string(),
            actor_type: "person".to_string(),
            tagged_statement_count: 3,
        }),
        about: Vec::new(),
        document: Some(DocumentRef {
            id: "doc-7".to_string(),
            title: "CFS interrogatory responses".to_string(),
            document_type: Some("discovery_response".to_string()),
        }),
    }
}

/// A raw extras row linking `ev-1` to one accusation through one element.
fn linked_row(edge_class: &str) -> CardExtrasRow {
    CardExtrasRow {
        evidence_id: "ev-1".to_string(),
        statement_type: Some("partial_admission".to_string()),
        grounding_status: Some("exact".to_string()),
        edge_class: Some(edge_class.to_string()),
        allegation_id: Some("alleg-54".to_string()),
        allegation_summary: Some("CFS knew of the meeting and did not act".to_string()),
        allegation_title: Some("Notice".to_string()),
        allegation_paragraph: Some("54".to_string()),
        element_name: Some("Notice".to_string()),
        count_number: Some(2),
        count_name: Some("Negligence".to_string()),
    }
}

/// An extras row for an item that links to NOTHING — the C-222 class.
fn unlinked_row() -> CardExtrasRow {
    CardExtrasRow {
        evidence_id: "ev-1".to_string(),
        statement_type: Some("partial_admission".to_string()),
        grounding_status: Some("exact".to_string()),
        edge_class: None,
        allegation_id: None,
        allegation_summary: None,
        allegation_title: None,
        allegation_paragraph: None,
        element_name: None,
        count_number: None,
        count_name: None,
    }
}

fn extras_for(rows: Vec<CardExtrasRow>) -> CollapsedExtras {
    collapse_extras(rows)
        .remove("ev-1")
        .expect("the fixture rows are all for ev-1")
}

fn scored_ref() -> CardRefState {
    CardRefState {
        status: Some(FactStatus::Undecided),
        confidence: Some(0.70),
        defer_reason: None,
    }
}

// ── The §7 contract ───────────────────────────────────────────────────────────

/// THE CONTRACT TEST. Every §7 element is present on a complete card.
///
/// §7 makes a card missing any element a defect, so this asserts each element by
/// name. A future edit that drops one — or serves it empty — fails here, which is
/// the point: the contract is not a document someone has to remember, it is a
/// test.
#[test]
fn every_section_seven_element_is_present_on_a_complete_card() {
    let extras = extras_for(vec![linked_row(crate::neo4j::schema::REBUTS)]);
    let page = "Q. Did you attend the meeting on March 3, 2019? \
                A. I do not recall that meeting. Q. Do you recall who was present?";
    let card = build_card(
        &full_instance(),
        Some(&extras),
        &scored_ref(),
        Some(14),
        Some(page),
        &settings(),
        HumanTouches::none(),
    );

    // §7.1 quote IN CONTEXT — the quote, and text either side of it.
    assert_eq!(card.quote.text, "I do not recall that meeting.");
    assert!(
        !card.quote.context_before.is_empty(),
        "§7.1 requires context, not a bare quote"
    );
    assert!(!card.quote.context_after.is_empty(), "§7.1");
    assert!(card.quote.question.is_some(), "§7.1 — the paired question");

    // §7.2 pinpoint — document, page, and a jump target the browser did not build.
    assert_eq!(card.pinpoint.document_id, "doc-7");
    assert_eq!(card.pinpoint.document_title, "CFS interrogatory responses");
    assert_eq!(card.pinpoint.page, Some(14));
    assert_eq!(
        card.pinpoint.viewer_href,
        "/documents/doc-7?page=14&tab=document"
    );
    // The label ships composed so the client joins nothing.
    assert_eq!(card.pinpoint.label, "CFS interrogatory responses at 14");

    // §7.3 speaker + statement kind.
    assert_eq!(card.speaker.name.as_deref(), Some("R. Phillips"));
    assert_eq!(card.speaker.attribution, "extracted");
    assert_eq!(card.statement_kind.as_deref(), Some("partial admission"));

    // §7.5 stance WITH its object — never a bare verb.
    let stance = card.stance.as_ref().expect("§7.5 requires a stance");
    assert_eq!(stance.verb, "disputes");
    assert!(stance.object.contains("CFS knew of the meeting"));
    assert!(stance.summary.contains("disputes"));

    // §7.6 bears-on, in complaint language.
    assert_eq!(card.bears_on.len(), 1, "§7.6");
    assert!(card.bears_on[0].accusation.contains("¶54"));
    assert_eq!(card.bears_on[0].elements, vec!["Notice".to_string()]);
    assert_eq!(
        card.bears_on[0].count.as_deref(),
        Some("Count 2 — Negligence")
    );

    // §7.7 grounding.
    let grounding = card.grounding.as_ref().expect("§7.7 requires grounding");
    assert_eq!(grounding.state, "exact");
    assert!(grounding.label.contains("Grounded"));

    // §7.8 banded confidence — a band and a sentence, never a percentage.
    assert_eq!(card.confidence.band, ConfidenceBand::Medium);
    assert!(!card.confidence.label.contains('%'));

    // Task 1.1's handle, and the workbench state in plain language.
    assert_eq!(card.code.as_deref(), Some("C-14"));
    assert_eq!(card.status_label, "Not yet decided");

    // A complete card is rulable.
    assert!(!card.defer_required);
    assert_eq!(card.defer_required_reason, None);
}

// ── The July failure ──────────────────────────────────────────────────────────

/// The C-222 class: a scored item with no accusation link serves NO stance.
///
/// This is the regression test for the failure that produced this whole task. The
/// card must not say "disputes" with nothing after it; it must serve `stance:
/// null`, flag itself unrulable, and say why in a sentence a human can act on.
#[test]
fn an_item_with_no_accusation_link_serves_a_defer_flag_not_a_bare_stance() {
    let extras = extras_for(vec![unlinked_row()]);
    let card = build_card(
        &full_instance(),
        Some(&extras),
        &scored_ref(),
        Some(222),
        None,
        &settings(),
        HumanTouches::none(),
    );

    assert!(
        card.stance.is_none(),
        "a stance with no object must not be served at all"
    );
    assert!(card.bears_on.is_empty());
    assert!(
        card.defer_required,
        "the card must declare itself unrulable"
    );

    let reason = card
        .defer_required_reason
        .as_ref()
        .expect("defer_required must carry a reason");
    // Names the cause…
    assert!(
        reason.contains("not linked to any accusation"),
        "the reason must name what is missing: {reason}"
    );
    // …says the scan DID score it, which is what made C-222 look rulable…
    assert!(
        reason.contains("A scan scored this item"),
        "the reason must explain why it looked rulable: {reason}"
    );
    // …and says what can actually be done about it TODAY (task 1.7E, item 6).
    assert!(
        reason.contains("It can only be deferred for now")
            && reason.contains("returns when linking arrives"),
        "the reason must offer the way forward that exists: {reason}"
    );
    // The regression guard for the promise that was withdrawn. Nothing links an
    // item to an accusation until task 2.10, so a card that tells a human to do it
    // sends them hunting for a control that is not there.
    assert!(
        !reason.contains("Link it to an accusation"),
        "the card must not promise linking before 2.10 ships it: {reason}"
    );
}

#[test]
fn an_unscored_unlinked_item_gets_the_simpler_reason() {
    // No scan touched this one, so the reason must not claim a scan did — the
    // difference between "the model was confident about nothing" and "nobody has
    // looked" is exactly the distinction the band vocabulary preserves.
    let extras = extras_for(vec![unlinked_row()]);
    let card = build_card(
        &full_instance(),
        Some(&extras),
        &CardRefState::default(),
        Some(9),
        None,
        &settings(),
        HumanTouches::none(),
    );

    let reason = card.defer_required_reason.expect("still unrulable");
    assert!(reason.starts_with("This item is not linked to any accusation"));
    assert!(!reason.contains("A scan scored"));
    // The SAME honest ending as the scored branch (task 1.7E, item 6). Asserted
    // here too because the prefix check above still passes over a reverted ending
    // — the two branches share one tail, so they need the same guard or one of
    // them can quietly go back to promising a control that does not exist.
    assert!(
        reason.contains("It can only be deferred for now")
            && reason.contains("returns when linking arrives"),
        "the unscored reason must offer the way forward that exists: {reason}"
    );
    assert!(
        !reason.contains("Link it to an accusation"),
        "the card must not promise linking before 2.10 ships it: {reason}"
    );
}

#[test]
fn an_item_with_no_quote_is_unrulable_for_the_citability_reason() {
    // The other unrulable class. The message matches the law task 1.1 enforces on
    // the write path, so the human meets the same explanation in both places.
    let mut instance = full_instance();
    instance.verbatim_quote = None;
    let extras = extras_for(vec![linked_row(crate::neo4j::schema::REBUTS)]);

    let card = build_card(
        &instance,
        Some(&extras),
        &scored_ref(),
        Some(3),
        None,
        &settings(),
        HumanTouches::none(),
    );
    assert!(card.defer_required);
    let reason = card.defer_required_reason.expect("unrulable");
    assert!(reason.contains("no verbatim quote"), "{reason}");
    assert!(reason.contains("cannot be cited"), "{reason}");
}

/// An item the graph knows nothing extra about is still a card.
#[test]
fn a_candidate_with_no_extras_still_builds_and_flags_itself() {
    let card = build_card(
        &full_instance(),
        None,
        &CardRefState::default(),
        None,
        None,
        &settings(),
        HumanTouches::none(),
    );

    assert!(card.stance.is_none());
    assert!(card.bears_on.is_empty());
    assert!(card.grounding.is_none());
    assert_eq!(card.statement_kind, None);
    assert!(card.defer_required);
    // No ordinal yet → no chip, rather than a fabricated one.
    assert_eq!(card.code, None);
}

// ── The vocabulary law ────────────────────────────────────────────────────────

/// No banned word reaches any string on a served card.
///
/// The payload is the enforcement point for the language law (v2 header), so this
/// walks EVERY human-readable string on a fully-populated card — including every
/// entry of `bears_on`, which is why the fixture below carries two accusations —
/// and checks each against the canon's banned list. A leak here is exactly the
/// July defect returning.
///
/// Note the `bears_on` strings are pass-through from author-written graph data
/// rather than vocabulary this system injects, so a hit there would mean a
/// complaint paragraph literally contains a banned word. That is worth knowing
/// too: the canon governs what the CARD says, whatever the source.
#[test]
fn no_banned_word_appears_in_any_card_string() {
    use crate::domain::card_language::BANNED_CARD_WORDS;

    // Build one card per edge class so every stance verb is exercised, and give
    // each card TWO accusations so the sweep reaches beyond `bears_on[0]` — the
    // first accusation is also reachable through `stance.summary`, so a
    // single-allegation fixture would leave the rest of the list unswept.
    for edge in crate::domain::case_state::partition::ConnectionTier::Topical.edge_types() {
        let mut second = linked_row(edge);
        second.allegation_id = Some("alleg-55".to_string());
        second.allegation_summary = Some("CFS failed to record the call".to_string());
        second.allegation_paragraph = Some("55".to_string());
        second.element_name = Some("Causation".to_string());
        let extras = extras_for(vec![linked_row(edge), second]);
        let card = build_card(
            &full_instance(),
            Some(&extras),
            &scored_ref(),
            Some(14),
            Some("context before the quote. I do not recall that meeting. and after."),
            &settings(),
            HumanTouches::none(),
        );

        let mut strings = vec![
            card.quote.text.clone(),
            card.quote.context_before.clone(),
            card.quote.context_after.clone(),
            card.pinpoint.document_title.clone(),
            card.confidence.label.clone(),
            card.status_label.clone(),
        ];
        strings.extend(card.quote.question.clone());
        strings.extend(card.statement_kind.clone());
        strings.extend(card.speaker.name.clone());
        strings.extend(card.grounding.as_ref().map(|g| g.label.clone()));
        strings.extend(card.defer_required_reason.clone());
        // Every bears-on string, for every accusation — not just the first.
        for bears_on in &card.bears_on {
            strings.push(bears_on.accusation.clone());
            strings.extend(bears_on.elements.clone());
            strings.extend(bears_on.count.clone());
        }
        if let Some(stance) = card.stance.as_ref() {
            strings.push(stance.verb.clone());
            strings.push(stance.summary.clone());
        }

        for string in &strings {
            let lowered = string.to_lowercase();
            for banned in BANNED_CARD_WORDS {
                assert!(
                    !lowered.contains(banned),
                    "card string {string:?} (edge {edge}) contains banned word \
                     {banned:?} — see the naming canon §9"
                );
            }
        }
    }
}

#[test]
fn each_edge_class_renders_its_canon_verb() {
    use crate::neo4j::schema;
    for (edge, expected) in [
        (schema::CORROBORATES, "supports"),
        (schema::REBUTS, "disputes"),
        (schema::CHARACTERIZES, "comments on"),
        (schema::ABOUT, "mentions"),
    ] {
        let extras = extras_for(vec![linked_row(edge)]);
        let card = build_card(
            &full_instance(),
            Some(&extras),
            &scored_ref(),
            None,
            None,
            &settings(),
            HumanTouches::none(),
        );
        let stance = card.stance.expect("a mapped edge yields a stance");
        assert_eq!(stance.verb, expected, "edge {edge}");
    }
}

// ── Collapsing the fan-out ────────────────────────────────────────────────────

/// One accusation bearing on several elements shows ONCE, with all its elements.
///
/// Measured on DEV: ¶12 feeds three elements of Count 2 ("Defendant had a duty to
/// disclose", "The undisclosed facts were material", "Plaintiff was damaged by the
/// reliance"). Repeating the accusation three times makes the human deduplicate by
/// eye; keeping only the first element understates what the item does for the
/// case. Both are wrong, so the card groups.
#[test]
fn one_accusation_with_several_elements_lists_them_all_under_one_entry() {
    let mut second = linked_row(crate::neo4j::schema::REBUTS);
    second.element_name = Some("Causation".to_string());
    let extras = extras_for(vec![linked_row(crate::neo4j::schema::REBUTS), second]);

    // Both elements survive the collapse…
    assert_eq!(
        extras.links.len(),
        2,
        "one link per (edge, allegation, element)"
    );

    // …and the card shows the accusation once, carrying both.
    let card = build_card(
        &full_instance(),
        Some(&extras),
        &scored_ref(),
        None,
        None,
        &settings(),
        HumanTouches::none(),
    );
    assert_eq!(card.bears_on.len(), 1, "the accusation appears once");
    assert_eq!(
        card.bears_on[0].elements,
        vec!["Notice".to_string(), "Causation".to_string()],
        "every element the accusation bears on must be listed"
    );
}

#[test]
fn an_accusation_wired_to_no_element_carries_an_empty_list() {
    // A real state: the complaint paragraph exists, the legal theory is not yet
    // mapped to it. Empty, not absent, and not a fabricated element name.
    let mut row = linked_row(crate::neo4j::schema::REBUTS);
    row.element_name = None;
    row.count_number = None;
    row.count_name = None;
    let extras = extras_for(vec![row]);

    let card = build_card(
        &full_instance(),
        Some(&extras),
        &scored_ref(),
        None,
        None,
        &settings(),
        HumanTouches::none(),
    );
    assert_eq!(card.bears_on.len(), 1);
    assert!(card.bears_on[0].elements.is_empty());
    assert_eq!(card.bears_on[0].count, None);
}

#[test]
fn two_distinct_allegations_are_both_carried() {
    let mut other = linked_row(crate::neo4j::schema::REBUTS);
    other.allegation_id = Some("alleg-55".to_string());
    other.allegation_summary = Some("CFS failed to record the call".to_string());
    other.allegation_paragraph = Some("55".to_string());
    let extras = extras_for(vec![linked_row(crate::neo4j::schema::REBUTS), other]);

    let card = build_card(
        &full_instance(),
        Some(&extras),
        &scored_ref(),
        None,
        None,
        &settings(),
        HumanTouches::none(),
    );
    assert_eq!(card.bears_on.len(), 2, "both accusations must be shown");
}

// ── Quote in context ──────────────────────────────────────────────────────────

#[test]
fn context_is_taken_from_around_the_quote() {
    let page = "BEFORE TEXT. I do not recall that meeting. AFTER TEXT.";
    let card = build_card(
        &full_instance(),
        None,
        &CardRefState::default(),
        None,
        Some(page),
        &settings(),
        HumanTouches::none(),
    );
    assert!(card.quote.context_before.contains("BEFORE TEXT"));
    assert!(card.quote.context_after.contains("AFTER TEXT"));
}

#[test]
fn a_quote_absent_from_its_page_yields_empty_context_not_a_guess() {
    // The quote does not appear in the page text (an OCR difference, a wrong page
    // number). Empty context is honest; inventing a window around an arbitrary
    // offset would put words next to a quote that never sat beside them.
    let card = build_card(
        &full_instance(),
        None,
        &CardRefState::default(),
        None,
        Some("a page whose text does not contain the quote"),
        &settings(),
        HumanTouches::none(),
    );
    assert!(card.quote.context_before.is_empty());
    assert!(card.quote.context_after.is_empty());
}

/// The window is read through its seam, and both edges use the same value.
///
/// Task 1.6 served this from the settings store; the test that survives that
/// change is one asserting the SEAM is used, not one asserting the number. Both
/// edges reading one call is what stops a lopsided window.
///
/// ## Rewritten by task 1.7C (defect D6)
///
/// It used to assert `chars().count() == window` on both sides. That assertion
/// encoded the very defect D6 removes: a HARD cut at exactly `window`
/// characters, landing mid-word and mid-sentence. Under the new law the window is
/// a SEED and sentence boundaries decide, so an equality on the width is no longer
/// the property — the property is that one value still drives both edges, which a
/// widening comparison states directly and a fixed number cannot.
#[test]
fn both_context_edges_use_the_same_window_value() {
    // Sentences in the filler, so expansion has boundaries to stop at rather than
    // running to the page edges (which would make the seed unobservable).
    let filler = "Filler sentence for the window. ".repeat(40);
    let page = format!("{filler}I do not recall that meeting. {filler}");

    let narrow = Settings {
        quote_context_window_chars: 60,
        ..settings()
    };
    let wide = Settings {
        quote_context_window_chars: 480,
        ..settings()
    };
    let build = |s: &Settings| {
        build_card(
            &full_instance(),
            None,
            &CardRefState::default(),
            None,
            Some(&page),
            s,
            HumanTouches::none(),
        )
    };
    let small = build(&narrow);
    let large = build(&wide);

    // A bigger seed reaches further on BOTH sides — that is the seam being read
    // and applied symmetrically. If either edge ignored the setting, one of these
    // two would not move.
    assert!(
        large.quote.context_before.chars().count() > small.quote.context_before.chars().count(),
        "the before-edge must widen with the seed: {} vs {}",
        small.quote.context_before.chars().count(),
        large.quote.context_before.chars().count()
    );
    assert!(
        large.quote.context_after.chars().count() > small.quote.context_after.chars().count(),
        "the after-edge must widen with the seed: {} vs {}",
        small.quote.context_after.chars().count(),
        large.quote.context_after.chars().count()
    );
    // And the seed is a FLOOR on the SLICE, not a ceiling — expansion goes
    // outward (§3.1). The comparison allows two characters of slack because the
    // display transform trims the flank's edges, and a slice that begins just
    // after a "… . " sentence break carries exactly that much trimmable
    // whitespace. Slack for the trim, not for the expansion.
    assert!(large.quote.context_before.chars().count() >= 480 - 2);
    assert!(large.quote.context_after.chars().count() >= 480 - 2);
    // Both ends landed on real sentence boundaries, so neither is a page edge.
    assert!(large.quote.context_before_complete);
    assert!(large.quote.context_after_complete);
}

#[test]
fn context_windowing_never_splits_a_multibyte_character() {
    // Legal PDFs are full of curly quotes and accented names. Slicing a &str at a
    // byte offset inside one of those PANICS, so every offset the assembly
    // computes must be a character boundary. That panic is what this test exists
    // to catch, and it catches it by running at all.
    //
    // 1.7C note: the old width equality is gone (see the test above), but the
    // fixture is unchanged — an all-`é` page is still the sharpest probe, because
    // every single offset in it is a potential mid-character slice.
    let window = settings().quote_context_window_chars;
    let filler = "é".repeat(window * 2);
    let page = format!("{filler}I do not recall that meeting.{filler}");
    let card = build_card(
        &full_instance(),
        None,
        &CardRefState::default(),
        None,
        Some(&page),
        &settings(),
        HumanTouches::none(),
    );

    // No terminator anywhere in the filler, so both flanks run to the page edges —
    // which is the whole filler, every character intact.
    assert!(card.quote.context_before.chars().all(|c| c == 'é'));
    assert!(card.quote.context_after.chars().all(|c| c == 'é'));
    assert_eq!(card.quote.context_before.chars().count(), window * 2);
    assert_eq!(card.quote.context_after.chars().count(), window * 2);
    // Both edges are page edges, and the card says so rather than implying the
    // multibyte filler was a sentence (Standing Rule 1).
    assert!(!card.quote.context_before_complete);
    assert!(!card.quote.context_after_complete);
}

/// THE 1.7A DEFECT (D3). A quote whose wording only matches after normalization
/// still gets its context — and the context is the PAGE's own characters.
///
/// ## What was wrong
///
/// The assembler located context with an exact, case-sensitive `find`. The
/// ingest path grounds in two tiers — exact, then normalized — so every quote
/// that grounded `normalized` failed that `find` and was served with two empty
/// strings. Measured on DEV: 320 of 320 normalized-grounded items with stored
/// page text, i.e. every one of them. The card that most needs its surroundings
/// is precisely the one whose wording does not match the page character for
/// character.
///
/// ## Why the assertion on the page's own characters matters
///
/// The cheap fix slices the NORMALIZED text — which is lowercased,
/// whitespace-collapsed and re-punctuated. That would put de-punctuated
/// lowercase prose beside a verbatim quote on the surface a lawyer reads to
/// decide what the quote means. This assertion is what stops a later
/// "simplification" from doing it.
#[test]
fn a_normalized_quote_still_gets_context_in_the_pages_own_characters() {
    // The page carries smart quotes and a hyphenated line break; the stored
    // quote carries straight punctuation and no break. This is the real shape of
    // a `normalized` grounding, not a contrived one.
    let mut instance = full_instance();
    instance.verbatim_quote = Some("I do not recall that meeting.".to_string());
    let page = "BEFORE TEXT. \u{201C}I do not re-\ncall that meeting.\u{201D} AFTER TEXT.";

    let card = build_card(
        &instance,
        None,
        &CardRefState::default(),
        None,
        Some(page),
        &settings(),
        HumanTouches::none(),
    );

    assert!(
        !card.quote.context_before.is_empty() && !card.quote.context_after.is_empty(),
        "a normalized-grounded quote must still be shown in context (§7.1): \
         before={:?} after={:?}",
        card.quote.context_before,
        card.quote.context_after
    );
    assert!(
        card.quote.context_before.contains("BEFORE TEXT"),
        "{:?}",
        card.quote.context_before
    );
    assert!(
        card.quote.context_after.contains("AFTER TEXT"),
        "{:?}",
        card.quote.context_after
    );
    // The page's own smart quotes survive: the context is sliced from the
    // ORIGINAL text, never from the normalized copy.
    assert!(
        card.quote.context_before.contains('\u{201C}'),
        "context must be the page's own characters, not normalized prose: {:?}",
        card.quote.context_before
    );
    assert!(
        card.quote.context_after.contains('\u{201D}'),
        "context must be the page's own characters, not normalized prose: {:?}",
        card.quote.context_after
    );
}

/// The configured seed applies to a normalized match exactly as to an exact one.
///
/// ## Rewritten by task 1.7C (defect D6)
///
/// Same reason as `both_context_edges_use_the_same_window_value`: the old
/// `== window` equality asserted the hard cut D6 replaced. The property that
/// matters is that the SECOND locate tier is not a lesser citizen — a
/// normalized-grounded card gets the same seed, the same outward expansion, and
/// the same sentence-complete ends as an exactly-grounded one.
#[test]
fn a_normalized_match_honours_the_configured_window() {
    let window = settings().quote_context_window_chars;
    let filler = "Filler sentence for the window. ".repeat(40);
    // Smart quotes inside the quoted span, so only the normalized tier can find it.
    let page = format!("{filler}\u{201C}I do not recall that meeting.\u{201D} {filler}");

    let mut instance = full_instance();
    instance.verbatim_quote = Some("\"I do not recall that meeting.\"".to_string());

    let card = build_card(
        &instance,
        None,
        &CardRefState::default(),
        None,
        Some(&page),
        &settings(),
        HumanTouches::none(),
    );

    // The seed is honoured as a floor on both sides…
    assert!(card.quote.context_before.chars().count() >= window);
    assert!(card.quote.context_after.chars().count() >= window);
    // …and both ends are genuinely sentence-complete, not page edges.
    assert!(card.quote.context_before_complete);
    assert!(card.quote.context_after_complete);
    assert_eq!(card.quote.context_before_notice, None);
    assert_eq!(card.quote.context_after_notice, None);
}

/// THE HONEST LIMIT, kept explicit so the D3 fix cannot be widened into a guess.
///
/// Two classes of card still show no context, and both are correct:
///
/// * the quote spans a page boundary — grounding records the LEFT page of the
///   pair, and the assembler loads that page alone, so the words genuinely are
///   not all here (task 2.5's territory);
/// * the page text was never stored — already logged by name at read time.
///
/// Neither may be papered over with a window drawn around the nearest similar
/// words. Beside a verbatim quote, that would read as evidence.
#[test]
fn a_quote_that_is_not_on_this_page_still_yields_no_context() {
    let mut instance = full_instance();
    instance.verbatim_quote = Some("the cheque was cashed on the Tuesday".to_string());

    let card = build_card(
        &instance,
        None,
        &CardRefState::default(),
        None,
        // Deliberately similar prose: a fuzzy "best effort" would match here.
        Some("BEFORE. The cheque and the Tuesday meeting were discussed. AFTER."),
        &settings(),
        HumanTouches::none(),
    );

    assert!(
        card.quote.context_before.is_empty(),
        "{:?}",
        card.quote.context_before
    );
    assert!(
        card.quote.context_after.is_empty(),
        "{:?}",
        card.quote.context_after
    );
}

#[test]
fn a_page_shorter_than_the_window_returns_what_there_is() {
    // 1.7C (D6) changes the expected strings in two ways, both deliberate:
    //
    // 1. They are TRIMMED. The old assertion expected `"Short. "` and `" End."`
    //    with the seam whitespace attached, which the display transform now
    //    removes — a context opening with a stray space reads as a rendering fault.
    // 2. They carry completeness. The before-flank ran to the top of the page, so
    //    whether that sentence started here or on the previous page is unknowable
    //    and the card says so. The after-flank ran to the bottom but ALREADY ends
    //    on a full stop, so it is complete and carries no notice — the asymmetry
    //    documented on `sentence_bounds::ends_with_sentence_end`.
    let page = "Short. I do not recall that meeting. End.";
    let card = build_card(
        &full_instance(),
        None,
        &CardRefState::default(),
        None,
        Some(page),
        &settings(),
        HumanTouches::none(),
    );

    assert_eq!(card.quote.context_before, "Short.");
    assert!(!card.quote.context_before_complete);
    assert_eq!(
        card.quote.context_before_notice.as_deref(),
        Some("— page begins here")
    );

    assert_eq!(card.quote.context_after, "End.");
    assert!(
        card.quote.context_after_complete,
        "'End.' is a finished sentence, so it must NOT be labelled a page edge"
    );
    assert_eq!(card.quote.context_after_notice, None);
}

// ── Pinpoint ──────────────────────────────────────────────────────────────────

#[test]
fn a_pageless_item_gets_a_viewer_link_without_a_page_anchor() {
    let mut instance = full_instance();
    instance.page_number = None;
    let card = build_card(
        &instance,
        None,
        &CardRefState::default(),
        None,
        None,
        &settings(),
        HumanTouches::none(),
    );

    assert_eq!(card.pinpoint.page, None);
    assert_eq!(card.pinpoint.viewer_href, "/documents/doc-7?tab=document");
    assert!(!card.pinpoint.viewer_href.contains("page="));
    // A page-less item reads as the document alone — never "… at null".
    assert_eq!(card.pinpoint.label, "CFS interrogatory responses");
}

// ── Human defer reason (task 1.1) ─────────────────────────────────────────────

#[test]
fn a_humans_defer_reason_rides_the_card_separately_from_the_system_flag() {
    // Two different sentences with two different authors: `defer_reason` is the
    // human's ("I parked this because…"), `defer_required_reason` is the system's
    // ("this cannot be ruled on because…"). Collapsing them would attribute the
    // system's words to the human.
    let extras = extras_for(vec![linked_row(crate::neo4j::schema::REBUTS)]);
    let ref_state = CardRefState {
        status: Some(FactStatus::Undecided),
        confidence: Some(0.9),
        defer_reason: Some("waiting on the unredacted page".to_string()),
    };
    let card = build_card(
        &full_instance(),
        Some(&extras),
        &ref_state,
        None,
        None,
        &settings(),
        HumanTouches::none(),
    );

    assert_eq!(
        card.defer_reason.as_deref(),
        Some("waiting on the unredacted page")
    );
    // This card IS rulable — a human parked it, the system did not refuse it.
    assert!(!card.defer_required);
    assert_eq!(card.defer_required_reason, None);
}

#[test]
fn status_labels_are_plain_language_for_every_state() {
    for (status, expected) in [
        (FactStatus::Undecided, "Not yet decided"),
        (FactStatus::Included, "In the scenario"),
        (FactStatus::Dropped, "Set aside"),
    ] {
        let ref_state = CardRefState {
            status: Some(status),
            ..CardRefState::default()
        };
        let card = build_card(
            &full_instance(),
            None,
            &ref_state,
            None,
            None,
            &settings(),
            HumanTouches::none(),
        );
        assert_eq!(card.status_label, expected);
    }
}

// ─── The human's correction of a machine question (task 1.7F Part B) ─────────

/// An override row, as the repository would hand one over.
fn override_row(text: &str, author: &str) -> EvidenceSummaryOverrideRecord {
    let written = DateTime::parse_from_rfc3339("2026-08-04T11:28:25Z")
        .expect("a literal timestamp parses")
        .with_timezone(&Utc);
    EvidenceSummaryOverrideRecord {
        graph_node_id: "ev-1".to_string(),
        summary_text: text.to_string(),
        authored_by: author.to_string(),
        created_at: written,
        updated_at: written,
    }
}

/// With nobody having corrected it, the card shows the machine's question and
/// says so.
#[test]
fn an_uncorrected_question_is_labelled_system() {
    let card = build_card(
        &full_instance(),
        None,
        &CardRefState::default(),
        None,
        None,
        &settings(),
        HumanTouches::none(),
    );

    assert_eq!(
        card.quote.question.as_deref(),
        Some("Did you attend the meeting on March 3, 2019?"),
        "with no override the machine's own question is what shows"
    );
    let authorship = card
        .quote
        .question_authorship
        .expect("a card with a question always reports who wrote it");
    assert_eq!(authorship.source, QuestionSource::System);
    assert_eq!(authorship.label, "System");
}

/// A correction replaces the text on screen and names its author and date.
#[test]
fn a_corrected_question_shows_the_humans_words_and_credits_them() {
    let row = override_row("Did you receive Marie Awad's certified letter?", "roman");
    let card = build_card(
        &full_instance(),
        None,
        &CardRefState::default(),
        None,
        None,
        &settings(),
        HumanTouches {
            question_override: Some(&row),
            links: &[],
        },
    );

    assert_eq!(
        card.quote.question.as_deref(),
        Some("Did you receive Marie Awad's certified letter?")
    );
    let authorship = card
        .quote
        .question_authorship
        .expect("a corrected question reports its author");
    assert_eq!(authorship.source, QuestionSource::Human);
    assert!(
        authorship.label.contains("roman"),
        "the badge names who wrote the text now on screen: {}",
        authorship.label
    );
    assert!(
        authorship.label.contains("4 Aug 2026"),
        "the badge dates the correction so a reader can see how old it is: {}",
        authorship.label
    );
}

/// THE ORIGINAL-UNTOUCHABLE TEST, at the composition layer (ruling R2).
///
/// The machine's words are an input to this function and stay one. Composing a
/// card with an override must not alter the instance it was built from, and
/// building the SAME instance again with no override — which is exactly what
/// happens after a revert deletes the row — must return the machine's sentence
/// byte-identically. If either half fails, "revert restores the system text"
/// stops being true and there is no second copy to restore from.
#[test]
fn an_override_never_rewrites_the_machines_words() {
    let instance = full_instance();
    let machine_question = instance
        .question
        .clone()
        .expect("the fixture carries a machine question");

    let row = override_row("A human's replacement question.", "roman");
    let overridden = build_card(
        &instance,
        None,
        &CardRefState::default(),
        None,
        None,
        &settings(),
        HumanTouches {
            question_override: Some(&row),
            links: &[],
        },
    );
    assert_eq!(
        overridden.quote.question.as_deref(),
        Some("A human's replacement question.")
    );

    // The source data is untouched: the override was applied to the OUTPUT.
    assert_eq!(
        instance.question.as_deref(),
        Some(machine_question.as_str()),
        "composing an overridden card must not mutate the instance it read from"
    );

    // …and the revert path — the same build with the row gone — restores the
    // machine's sentence exactly, because it was never edited.
    let reverted = build_card(
        &instance,
        None,
        &CardRefState::default(),
        None,
        None,
        &settings(),
        HumanTouches::none(),
    );
    assert_eq!(
        reverted.quote.question.as_deref(),
        Some(machine_question.as_str()),
        "revert must restore the machine's words byte for byte"
    );
    assert_eq!(
        reverted
            .quote
            .question_authorship
            .expect("still reports authorship")
            .source,
        QuestionSource::System,
        "a reverted card is a system card again"
    );
}

/// A human may supply a question the extraction never produced.
///
/// Correcting an omission is a correction. The presence of the HUMAN's text
/// decides whether the line renders, not the machine's silence.
#[test]
fn a_correction_can_add_a_question_the_machine_never_had() {
    let instance = BiasInstance {
        question: None,
        ..full_instance()
    };
    let row = override_row("The question the extraction missed.", "roman");
    let card = build_card(
        &instance,
        None,
        &CardRefState::default(),
        None,
        None,
        &settings(),
        HumanTouches {
            question_override: Some(&row),
            links: &[],
        },
    );

    assert_eq!(
        card.quote.question.as_deref(),
        Some("The question the extraction missed.")
    );
    assert_eq!(
        card.quote
            .question_authorship
            .expect("reports authorship")
            .source,
        QuestionSource::Human
    );
}

/// A card with no question at all reports no authorship.
///
/// An authorship badge hanging off a card with nothing to attribute is a field
/// the client would have to interpret, and documentary evidence genuinely has no
/// interrogatory behind it.
#[test]
fn a_card_with_no_question_has_no_authorship_badge() {
    let instance = BiasInstance {
        question: None,
        ..full_instance()
    };
    let card = build_card(
        &instance,
        None,
        &CardRefState::default(),
        None,
        None,
        &settings(),
        HumanTouches::none(),
    );

    assert!(card.quote.question.is_none());
    assert!(card.quote.question_authorship.is_none());
}

// ─── Task 2.10: a human supplies the link the extraction never made ──────────
//
// The card's side of the feature. `scenario_human_links_tests` proves the
// sentence in isolation; these prove what a WHOLE card becomes, which is what
// §7.5 and ruling R2 are actually about.

/// One human link, as the card path receives it.
fn human_link(paragraph: &str, cut: crate::domain::link_cut::LinkCut) -> CardHumanLink {
    CardHumanLink {
        allegation_id: format!("alleg-{paragraph}"),
        label: format!("¶{paragraph} — refused to divide the property amicably"),
        cut,
        cut_label: "They'll use it against us".to_string(),
    }
}

/// An unlinked card, with human links attached.
fn linked_by_hand(links: &[CardHumanLink]) -> ScenarioCard {
    let extras = extras_for(vec![unlinked_row()]);
    build_card(
        &full_instance(),
        Some(&extras),
        &scored_ref(),
        Some(222),
        None,
        &settings(),
        HumanTouches {
            question_override: None,
            links,
        },
    )
}

/// THE UNLOCK. A human link clears the defer flag the extraction's silence set.
///
/// This is the whole task in one assertion: 94 of S-2's 148 cards are defer-only
/// for exactly the reason this removes (measured on DEV, 2026-08-04).
#[test]
fn a_human_link_makes_an_unlinked_card_rulable() {
    let links = vec![human_link("41", crate::domain::link_cut::LinkCut::Against)];
    let card = linked_by_hand(&links);

    assert!(
        !card.defer_required,
        "the reason §7.5 refused Include and Exclude is gone"
    );
    assert_eq!(card.defer_required_reason, None);
}

/// RULING R2, ON A WHOLE CARD: no stance verb is emitted.
///
/// The machine said nothing about this statement. `stance` stays `null`, and
/// nothing anywhere on the card speaks in the machine's voice — asserted against
/// every canon verb, over every string the card carries, so a future change that
/// routed human links into `build_stance` fails here rather than shipping a
/// finding nobody made.
#[test]
fn no_stance_verb_is_emitted_for_a_human_linked_machine_unlinked_card() {
    let links = vec![human_link("41", crate::domain::link_cut::LinkCut::Against)];
    let card = linked_by_hand(&links);

    assert!(
        card.stance.is_none(),
        "the extraction found no stance, so the card must not carry one"
    );
    assert!(
        card.bears_on.is_empty(),
        "bears_on is the MACHINE's list; a human link does not join it"
    );

    let summary = card
        .human_link_summary
        .as_ref()
        .expect("a linked card says what the human did");

    for edge in crate::domain::case_state::partition::ConnectionTier::Topical.edge_types() {
        let Some(verb) = crate::domain::card_language::stance_verb_for_edge(edge) else {
            continue;
        };
        assert!(
            !summary.contains(&format!("This {verb}")),
            "the card must not compose a machine stance sentence (found \
             '{verb}'): {summary}"
        );
    }
    assert!(summary.starts_with("You linked"), "{summary}");
}

/// The §7.5 slot holds exactly ONE of three things, and never none.
///
/// `cardRows` fills that slot with the stance, else the human sentence, else the
/// defer reason. Without the middle branch a correctly-behaving human-linked card
/// would have all three absent — and the §7 completeness contract would go red on
/// a card that is working perfectly.
#[test]
fn exactly_one_of_stance_human_summary_and_defer_reason_is_present() {
    let against = crate::domain::link_cut::LinkCut::Against;

    let machine = build_card(
        &full_instance(),
        Some(&extras_for(vec![linked_row("REBUTS")])),
        &scored_ref(),
        Some(1),
        None,
        &settings(),
        HumanTouches::none(),
    );
    let human = linked_by_hand(&[human_link("41", against)]);
    let neither = build_card(
        &full_instance(),
        Some(&extras_for(vec![unlinked_row()])),
        &scored_ref(),
        Some(2),
        None,
        &settings(),
        HumanTouches::none(),
    );

    for (name, card) in [("machine", machine), ("human", human), ("neither", neither)] {
        let filled = [
            card.stance.is_some(),
            card.human_link_summary.is_some(),
            card.defer_required_reason.is_some(),
        ]
        .iter()
        .filter(|present| **present)
        .count();
        assert_eq!(
            filled, 1,
            "the {name} card must fill the §7.5 slot exactly once"
        );
    }
}

/// The links reach the card as chips, in the order the human added them.
#[test]
fn every_human_link_reaches_the_card_with_its_own_cut() {
    let links = vec![
        human_link("41", crate::domain::link_cut::LinkCut::Against),
        human_link("92", crate::domain::link_cut::LinkCut::Supports),
    ];
    let card = linked_by_hand(&links);

    assert_eq!(card.human_links.len(), 2);
    assert_eq!(card.human_links[0].allegation_id, "alleg-41");
    assert_eq!(
        card.human_links[1].cut,
        crate::domain::link_cut::LinkCut::Supports
    );
}

/// A quoteless card stays unrulable however many accusations a human links it to.
///
/// The two refusal classes were always independent and must stay so: a statement
/// with no verbatim words cannot be cited, and linking it to ¶41 does not give it
/// words. Collapsing the two would let a human include an item that can never be
/// quoted in court.
#[test]
fn linking_does_not_unlock_a_card_that_has_no_quote() {
    let quoteless = BiasInstance {
        verbatim_quote: None,
        ..full_instance()
    };
    let card = build_card(
        &quoteless,
        Some(&extras_for(vec![unlinked_row()])),
        &scored_ref(),
        Some(3),
        None,
        &settings(),
        HumanTouches {
            question_override: None,
            links: &[human_link("41", crate::domain::link_cut::LinkCut::Against)],
        },
    );

    assert!(card.defer_required, "no quote is still no quote");
    let reason = card.defer_required_reason.expect("a reason");
    assert!(
        reason.contains("no verbatim quote"),
        "the surviving refusal must be the citability one: {reason}"
    );
}

/// A card nobody linked carries no summary and no links.
#[test]
fn an_untouched_card_carries_no_human_link_fields() {
    let card = build_card(
        &full_instance(),
        Some(&extras_for(vec![linked_row("REBUTS")])),
        &scored_ref(),
        Some(4),
        None,
        &settings(),
        HumanTouches::none(),
    );
    assert!(card.human_links.is_empty());
    assert!(card.human_link_summary.is_none());
}
