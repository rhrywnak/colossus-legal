// =============================================================================
// scenario_card_context_tests.rs — the D6 context law, over REAL page text
// =============================================================================
//
// Fixtures are verbatim pages from the DEV `document_text` table; see
// `backend/test_fixtures/document_text/README.md` for provenance. The whole point
// of D6 is that this corpus's real formatting breaks a naive window, so a test
// over invented prose would prove nothing.

use super::*;

const TRANSCRIPT_P25: &str = include_str!("../../test_fixtures/document_text/transcript_p25.txt");
const TRANSCRIPT_P14: &str = include_str!("../../test_fixtures/document_text/transcript_p14.txt");
const DISCOVERY_P31: &str =
    include_str!("../../test_fixtures/document_text/discovery_response_p31.txt");

/// The seed width the store actually holds (`Settings::for_test`, which
/// `settings_store_tests` pins against the migration seed).
const WINDOW: usize = 240;

/// Every standalone 1–2 digit line in `text` — what must NOT survive to display.
fn gutter_lines(text: &str) -> Vec<&str> {
    text.lines()
        .filter(|line| {
            let t = line.trim();
            matches!(t.len(), 1 | 2) && t.chars().all(|c| c.is_ascii_digit())
        })
        .collect()
}

// ─── rule 4: gutter numerals never reach the display ─────────────────────────

#[test]
fn gutter_numerals_are_stripped_from_both_flanks() {
    // Transcript page 25 carries 26 gutter lines. Measured on DEV, 87 of the 499
    // locatable cards rendered at least one before this fix; on this document it
    // was every single card.
    assert_eq!(
        gutter_lines(TRANSCRIPT_P25).len(),
        26,
        "fixture must carry the court reporter's 26 margin numerals"
    );

    let quote = "we'll refine the claims so it won't be a\nsecret any longer.";
    let context = assemble(Some(TRANSCRIPT_P25), quote, WINDOW);

    assert!(
        !context.before.text.is_empty() && !context.after.text.is_empty(),
        "this quote grounds on the page and must get context both sides"
    );
    for (side, flank) in [("before", &context.before), ("after", &context.after)] {
        assert!(
            gutter_lines(&flank.text).is_empty(),
            "context_{side} still renders a gutter numeral:\n{:?}",
            flank.text
        );
    }
}

#[test]
fn the_anchor_quote_is_never_touched_by_the_transform() {
    // §3.5 / §12: anchors are sacred. This module slices only OUTSIDE the located
    // span, so the quote cannot be groomed even in principle — but the guarantee
    // is load-bearing enough to assert, because "strip the gutter from the card"
    // is exactly the change someone would later apply one function too high.
    let quote = "we'll refine the claims so it won't be a\nsecret any longer.";
    let context = assemble(Some(TRANSCRIPT_P25), quote, WINDOW);

    // Neither flank may contain any part of the quote's own words.
    let joined = format!("{} {}", context.before.text, context.after.text);
    assert!(
        !joined.contains("refine the claims"),
        "a flank swallowed the anchor: {joined:?}"
    );
    // And the page itself is unmodified — this module takes &str and returns
    // owned Strings, so there is nothing to write back, but pin the intent.
    assert!(TRANSCRIPT_P25.contains("MR. SHARP: Well, we'll refine"));
}

// ─── rule 1: sentence boundaries decide ──────────────────────────────────────

#[test]
fn a_flank_no_longer_begins_mid_word() {
    // The measured defect that opened D6. On transcript page 3 the old hard window
    // produced `"lle Hanley--Hanley is not here, but…"` — the tail of "And Camille
    // Hanley". Same class of cut, asserted here on page 25: whatever the flank
    // starts with, it must be a word boundary in the page, not a fragment.
    let quote = "MR. PHILLIPS: Okay.";
    let context = assemble(Some(TRANSCRIPT_P25), quote, WINDOW);
    let before = context.before.text.trim_start();
    if before.is_empty() {
        return; // the quote sits at the page top; nothing to assert
    }

    // The flank is NOT a verbatim page substring — its gutter numerals have been
    // removed — so the check is on its first WORD: that word must appear in the
    // page as a whole word, not as the tail of a longer one. `"lle"` (from
    // "Camille") would fail; `"And"` passes.
    let first_word = before
        .split_whitespace()
        .next()
        .expect("a non-empty flank has a first word");
    let whole_word_somewhere = TRANSCRIPT_P25
        .match_indices(first_word)
        .any(|(at, _)| at == 0 || !TRANSCRIPT_P25[..at].ends_with(char::is_alphabetic));
    assert!(
        whole_word_somewhere,
        "the flank begins mid-word: first token {first_word:?} never occurs as a \
         whole word in the page"
    );
}

#[test]
fn a_complete_flank_ends_on_a_terminator_and_carries_no_notice() {
    let quote = "we'll refine the claims so it won't be a\nsecret any longer.";
    let context = assemble(Some(TRANSCRIPT_P25), quote, WINDOW);

    if context.after.complete {
        assert!(
            context.after.text.ends_with('.')
                || context.after.text.ends_with('?')
                || context.after.text.ends_with('!'),
            "a complete flank must end on real punctuation: {:?}",
            context.after.text
        );
        assert_eq!(
            context.after.notice, None,
            "a complete flank must not carry a page-edge notice"
        );
    }
}

#[test]
fn expansion_runs_outward_so_context_is_never_shorter_than_the_seed_allowed() {
    // §3.1: the window is a guide, the sentence is the law, and expansion goes
    // OUTWARD. A tiny seed must still yield a whole sentence.
    let quote = "MR. SHARP: That's fine.";
    let narrow = assemble(Some(TRANSCRIPT_P25), quote, 5);
    let wide = assemble(Some(TRANSCRIPT_P25), quote, WINDOW);

    assert!(
        !narrow.before.text.is_empty(),
        "a 5-char seed must still expand to a sentence, got empty"
    );
    assert!(
        narrow.before.text.chars().count() > 5,
        "expansion is outward: a 5-char seed produced {} chars",
        narrow.before.text.chars().count()
    );
    // And the wider seed reaches at least as far back.
    assert!(wide.before.text.chars().count() >= narrow.before.text.chars().count());
}

#[test]
fn enumerator_sub_parts_do_not_cut_a_discovery_flank_short() {
    // Real page: 10 line-initial `a.` / `b.` / `c.` sub-part labels. A naive
    // splitter ends the context at the first one.
    let quote = DISCOVERY_P31
        .lines()
        .find(|l| l.trim().len() > 40)
        .expect("the page has a substantial line")
        .trim();
    let context = assemble(Some(DISCOVERY_P31), quote, WINDOW);

    // Whatever it produced, it must not have stopped immediately after a lone
    // lowercase letter and a period.
    for (side, flank) in [("before", &context.before), ("after", &context.after)] {
        let t = flank.text.trim_end();
        if t.len() >= 2 {
            let tail: Vec<char> = t.chars().rev().take(2).collect();
            let ends_on_enumerator = tail[0] == '.'
                && tail[1].is_ascii_lowercase()
                && t.chars().rev().nth(2).is_some_and(|c| c.is_whitespace());
            assert!(
                !ends_on_enumerator,
                "context_{side} was cut at an enumerator: {t:?}"
            );
        }
    }
}

// ─── rules 3 + Standing Rule 1: page edges are honest ────────────────────────

#[test]
fn a_page_that_begins_mid_sentence_says_so_instead_of_faking_a_start() {
    // 154 of 499 locatable corpus cards hit this, measured. The synthetic page
    // here has no terminator before the quote at all — the strongest form of it.
    let page = "continuing from the previous page THE QUOTE and then more text.";
    let context = assemble(Some(page), "THE QUOTE", WINDOW);

    assert!(!context.before.complete);
    assert_eq!(
        context.before.notice.as_deref(),
        Some("— page begins here"),
        "the page-edge notice is composed server-side (language law)"
    );
    assert_eq!(context.before.text, "continuing from the previous page");
}

#[test]
fn a_page_that_ends_mid_sentence_says_so_instead_of_faking_an_end() {
    // 65 of 499, measured.
    let page = "A complete sentence first. THE QUOTE and then a tail with no stop";
    let context = assemble(Some(page), "THE QUOTE", WINDOW);

    assert!(!context.after.complete);
    assert_eq!(context.after.notice.as_deref(), Some("— page ends here"));
    assert_eq!(context.after.text, "and then a tail with no stop");
}

#[test]
fn one_end_may_be_complete_while_the_other_is_a_page_edge() {
    // Ruling R3: "A card may honestly show one complete end and one page-edge
    // end; never imply sentence-completeness the data doesn't have."
    //
    // The page is built so the BACKWARD seed lands past a real terminator (so
    // that end is complete) while the FORWARD seed runs off the page with no
    // terminator at all (so that end is a page edge).
    let filler = "and then more words with no terminator at all ".repeat(8);
    let page = format!("Alpha ends here. {filler}THE QUOTE a tail with no stop");
    let context = assemble(Some(&page), "THE QUOTE", WINDOW);

    assert!(
        context.before.complete,
        "the left end IS sentence-complete: {:?}",
        context.before
    );
    assert_eq!(context.before.notice, None);
    assert!(
        context.before.text.starts_with(&filler[..20]),
        "the flank must start just after 'Alpha ends here.': {:?}",
        &context.before.text[..30.min(context.before.text.len())]
    );

    assert!(!context.after.complete, "the right end is a page edge");
    assert_eq!(context.after.notice.as_deref(), Some("— page ends here"));
}

#[test]
fn a_page_start_is_reported_as_a_page_edge_even_when_it_reads_like_a_sentence() {
    // Deliberate and load-bearing. When the flank runs to the top of the page,
    // whether the sentence began there or on the PREVIOUS page is unknowable from
    // one `document_text` row — so this reports the page edge rather than
    // inferring completeness from a capital letter. Standing Rule 1: the card must
    // not claim a boundary the data cannot supply.
    let page = "A complete sentence first. THE QUOTE and then more.";
    let context = assemble(Some(page), "THE QUOTE", WINDOW);

    assert_eq!(context.before.text, "A complete sentence first.");
    assert!(
        !context.before.complete,
        "the flank reached the page top, so completeness is unknown"
    );
    assert_eq!(context.before.notice.as_deref(), Some("— page begins here"));
}

#[test]
fn a_quote_at_the_very_top_of_a_page_gets_no_flank_and_no_notice() {
    // An absent flank needs no page-edge explanation — there is nothing above it
    // to explain, and annotating the absence would be a different lie.
    let page = "THE QUOTE opens the page. Then more text follows.";
    let context = assemble(Some(page), "THE QUOTE", WINDOW);

    assert_eq!(context.before.text, "");
    assert_eq!(context.before.notice, None);
    assert!(
        context.before.complete,
        "an empty flank did not stop mid-sentence, so it is not 'incomplete'"
    );
}

// ─── the honest limits, unchanged from 1.7A ──────────────────────────────────

#[test]
fn no_stored_page_text_yields_no_context_rather_than_a_guess() {
    let context = assemble(None, "any quote at all", WINDOW);
    assert_eq!(context.before.text, "");
    assert_eq!(context.after.text, "");
    assert_eq!(context.before.notice, None);
    assert_eq!(context.after.notice, None);
}

#[test]
fn a_quote_not_on_the_cited_page_yields_no_context_rather_than_a_nearby_window() {
    // 26 of the corpus's 525 page-bearing quotes (5.0%), measured: the cross-page
    // pair (grounding records the LEFT page; this loads that page alone) and the
    // OCR-transposition case. A window drawn around the nearest similar words
    // would read as evidence — the 1.7A honest limit, carried forward unchanged.
    let context = assemble(
        Some(TRANSCRIPT_P25),
        "this sentence appears on no page of this transcript whatsoever",
        WINDOW,
    );
    assert_eq!(context.before.text, "");
    assert_eq!(context.after.text, "");
}

#[test]
fn a_normalized_grounded_quote_still_gets_context_from_the_original_bytes() {
    // The 1.7A repair (D3) must survive 1.7C. This quote differs from the page in
    // whitespace only — the second locate tier finds it, and the flanks come back
    // in the page's OWN capitalization and punctuation, not the normalized copy's.
    let quote = "we'll   refine the claims so it won't be a secret any longer.";
    let context = assemble(Some(TRANSCRIPT_P25), quote, WINDOW);

    assert!(
        !context.before.text.is_empty(),
        "a normalized match must still produce context (defect D3)"
    );
    assert!(
        context.before.text.contains("MR."),
        "the flank must carry the page's own uppercase speaker tag, not a \
         lowercased normalized copy: {:?}",
        context.before.text
    );
}

#[test]
fn the_transposed_ocr_page_still_assembles_without_reordering_anything() {
    // Transcript page 14 carries the Surya line-transposition wart. 1.7C renders
    // it as it is — the fix belongs in OCR, and a repair here would be a second,
    // silent normalizer (ruling R4).
    let quote = "And a lot of it's junk, tools, nuts and\nbolts kind of thing?";
    let context = assemble(Some(TRANSCRIPT_P14), quote, WINDOW);

    assert!(
        !context.after.text.is_empty(),
        "the page must still assemble"
    );
    assert!(
        gutter_lines(&context.after.text).is_empty(),
        "gutters still stripped on a transposed page"
    );
}

// ─── the measured size envelope ──────────────────────────────────────────────

#[test]
fn context_stays_bounded_by_the_page_with_no_expansion_parameter() {
    // Ruling R3: the page IS the bound. No setting, no constant. Measured over the
    // whole corpus, the largest total context any real quote produced was 1389
    // characters and the longest page is 4323 — so "a 4-page sentence-less stretch
    // swallows the card" is structurally impossible: the slice source is one
    // `document_text` row. This test pins that property on the real page.
    let quote = "MR. PHILLIPS: Okay.";
    let context = assemble(Some(TRANSCRIPT_P25), quote, WINDOW);
    let total = context.before.text.len() + context.after.text.len();
    assert!(
        total <= TRANSCRIPT_P25.len(),
        "context ({total}) cannot exceed its page ({})",
        TRANSCRIPT_P25.len()
    );
}
