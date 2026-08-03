// =============================================================================
// sentence_bounds_tests.rs — the D6 boundary rules, over REAL page text
// =============================================================================
//
// Every fixture here is a verbatim page from the DEV `document_text` table (see
// `backend/test_fixtures/document_text/README.md` for the export provenance).
// Hand-written prose would pass a naive splitter too; the point of D6 is that
// this corpus's actual formatting is what breaks one.

use super::*;

const TRANSCRIPT_P25: &str = include_str!("../../test_fixtures/document_text/transcript_p25.txt");
const TRANSCRIPT_P14: &str = include_str!("../../test_fixtures/document_text/transcript_p14.txt");
const DISCOVERY_P31: &str =
    include_str!("../../test_fixtures/document_text/discovery_response_p31.txt");
const COURT_ORDER_P9: &str = include_str!("../../test_fixtures/document_text/court_order_p9.txt");
const AFFIDAVIT_P3: &str = include_str!("../../test_fixtures/document_text/affidavit_p3.txt");

/// Byte offset of `needle` in `haystack`, for building a seed in a test.
fn at(haystack: &str, needle: &str) -> usize {
    haystack
        .find(needle)
        .unwrap_or_else(|| panic!("fixture must contain {needle:?}"))
}

// ─── the structural rule: a single letter never terminates ───────────────────

#[test]
fn a_single_letter_before_the_period_is_never_a_sentence_end() {
    // The measured headline finding: 229 line-initial single-letter enumerators
    // across the two discovery documents, and 17 mid-name initials. No
    // abbreviation LIST would catch `a.` — it is a numbering scheme, not an
    // abbreviation — so the guard has to be structural.
    //
    // Measured over all 160 corpus pages: this rule produces ZERO false
    // negatives. No real sentence in the corpus ends in a one-letter word.
    let text = "See subsection a. The estate was divided by appraisal.";
    let seed = at(text, "The estate");
    // Expanding backward from "The estate" must NOT stop after "a." — it must
    // run past it, find no earlier sentence end, and report the honest None.
    assert_eq!(
        sentence_start_before(text, seed),
        None,
        "a bare `a.` must not read as a sentence end"
    );
}

#[test]
fn real_enumerators_in_a_discovery_response_do_not_terminate() {
    // Real page: 10 line-initial `a.` / `b.` / `c.` sub-part labels.
    let enumerators = DISCOVERY_P31
        .lines()
        .filter(|l| {
            let t = l.trim();
            t.len() >= 2 && t.as_bytes()[1] == b'.' && t.as_bytes()[0].is_ascii_lowercase()
        })
        .count();
    assert!(
        enumerators >= 4,
        "fixture must actually carry enumerators, found {enumerators}"
    );

    // Walk the whole page and assert not one of those periods reads as an end.
    for (i, c) in DISCOVERY_P31.char_indices() {
        if c != '.' {
            continue;
        }
        let preceding: String = DISCOVERY_P31[..i].chars().rev().take(2).collect();
        // `preceding` is reversed: "a\n" means the page has "\na." — an enumerator.
        let mut chars = preceding.chars();
        let (Some(letter), Some(before)) = (chars.next(), chars.next()) else {
            continue;
        };
        if letter.is_ascii_lowercase() && (before == '\n' || before == ' ') {
            assert!(
                !is_sentence_end(DISCOVERY_P31, i),
                "the enumerator `{letter}.` at byte {i} must not end a sentence"
            );
        }
    }
}

// ─── the abbreviation list ───────────────────────────────────────────────────

#[test]
fn mr_does_not_terminate_in_either_case() {
    // `Mr.` at 177 occurrences is the corpus leader; `MR.` at 127 is the
    // transcript's speaker tag. Both must survive.
    for (text, tag) in [
        (
            "That is right. MR. PHILLIPS: Yeah, it was for four hundred.",
            "MR.",
        ),
        (
            "He spoke to Mr. Sharp about the claim before the hearing.",
            "Mr.",
        ),
    ] {
        let dot = at(text, tag) + tag.len() - 1;
        assert!(
            !is_sentence_end(text, dot),
            "`{tag}` must not end a sentence in {text:?}"
        );
    }
}

#[test]
fn real_transcript_speaker_tags_do_not_terminate() {
    // The whole page, every `MR.` in it.
    let mut checked = 0;
    let mut search = 0;
    while let Some(found) = TRANSCRIPT_P25[search..].find("MR.") {
        let dot = search + found + 2;
        assert!(
            !is_sentence_end(TRANSCRIPT_P25, dot),
            "the `MR.` at byte {dot} must not end a sentence"
        );
        checked += 1;
        search = dot + 1;
    }
    assert!(
        checked >= 5,
        "fixture must carry several MR. tags, checked {checked}"
    );
}

#[test]
fn legal_abbreviations_in_a_real_court_order_do_not_terminate() {
    // `v.` `Inc.` `No.` `Dec.` `Co.` — 6 of them on this page.
    for token in ["v.", "Inc.", "No.", "Dec."] {
        if let Some(found) = COURT_ORDER_P9.find(token) {
            let dot = found + token.len() - 1;
            assert!(
                !is_sentence_end(COURT_ORDER_P9, dot),
                "`{token}` at byte {dot} must not end a sentence"
            );
        }
    }
}

#[test]
fn the_affidavit_jurat_does_not_terminate() {
    // "STATE OF MICHIGAN ) SS." — measured 16× across the two affidavits.
    let found = at(AFFIDAVIT_P3, "SS.");
    assert!(
        !is_sentence_end(AFFIDAVIT_P3, found + 2),
        "`SS.` must not end a sentence"
    );
}

#[test]
fn the_q_and_a_examination_pattern_is_guarded_for_the_next_deposition() {
    // Measured: ZERO occurrences in TODAY's corpus — no deposition has been
    // ingested yet. Carried anyway (ruling R4): the next transcript ingest is a
    // deposition, whose spine is exactly this pattern, and discovering it by
    // watching a card break is not a plan.
    let text = "Q. Did you agree to divide the property? A. I did, in writing.";
    assert!(!is_sentence_end(text, at(text, "Q.") + 1));
    assert!(!is_sentence_end(text, at(text, "A.") + 1));
}

#[test]
fn abbreviations_are_matched_case_insensitively() {
    // One list entry covers both cases, which is why the corpus needs 11 entries
    // and not 22.
    for (text, tag) in [("Ms. Awad called.", "Ms."), ("MS. AWAD called.", "MS.")] {
        assert!(
            !is_sentence_end(text, at(text, tag) + tag.len() - 1),
            "`{tag}` must not end a sentence"
        );
    }
}

// ─── what DOES terminate ─────────────────────────────────────────────────────

#[test]
fn an_ordinary_sentence_end_terminates() {
    let text = "Marie agreed in writing. The refusal was theirs.";
    let dot = at(text, "writing.") + "writing".len();
    assert!(is_sentence_end(text, dot));
}

#[test]
fn a_question_mark_and_an_exclamation_terminate_without_a_word_guard() {
    let text = "Is that who we're dealing with? Yes!";
    assert!(is_sentence_end(text, at(text, "?")));
    assert!(is_sentence_end(text, at(text, "!")));
}

#[test]
fn a_period_inside_a_number_does_not_terminate() {
    // Guard 1 (what follows): a digit after the period. `$400.50` must not split,
    // and neither must `No.5`.
    let text = "It was for $400.50 in total.";
    assert!(!is_sentence_end(text, at(text, ".50")));
    assert!(is_sentence_end(text, at(text, "total.") + "total".len()));
}

#[test]
fn a_bare_ellipsis_does_not_terminate() {
    // Real transcript wording: "Do you want to kind of explain what you're
    // looking for and...?" — the `?` is the terminator, the dots are prose
    // trailing off. Cutting at a dot would reproduce the fragment problem.
    let text = "explain what you're looking for and...? MR. PHILLIPS: Yes, I can.";
    let first_dot = at(text, "...");
    assert!(!is_sentence_end(text, first_dot));
    assert!(!is_sentence_end(text, first_dot + 1));
    assert!(!is_sentence_end(text, first_dot + 2));
    assert!(is_sentence_end(text, first_dot + 3), "the ? ends it");
}

#[test]
fn a_terminator_before_a_closing_quote_still_terminates() {
    let text = "He said \"we'll refine the claims.\" Then he sat down.";
    assert!(is_sentence_end(text, at(text, "claims.") + "claims".len()));
}

// ─── the two walks ───────────────────────────────────────────────────────────

#[test]
fn backward_expansion_lands_on_a_word_not_a_space() {
    let text = "First sentence here.    Second sentence starts here.";
    let seed = at(text, "starts");
    let start = sentence_start_before(text, seed).expect("a boundary exists");
    assert_eq!(
        &text[start..start + 6],
        "Second",
        "the slice must begin on the word, with the run of spaces skipped"
    );
}

#[test]
fn forward_expansion_keeps_the_full_stop() {
    // A context ending "…any longer" reads as truncated; "…any longer." reads as
    // complete. The terminator is part of the slice.
    let text = "we'll refine the claims so it won't be a secret any longer. MR. PHILLIPS: Yeah.";
    let end = sentence_end_after(text, 0).expect("a boundary exists");
    assert!(
        text[..end].ends_with("longer."),
        "got {:?}",
        &text[..end.min(text.len())]
    );
}

#[test]
fn backward_expansion_reports_none_when_the_page_starts_mid_sentence() {
    // 154 of the 499 locatable corpus cards (30.9%) hit this, measured. The
    // caller must render the page edge honestly, not fake a sentence start.
    let text = "continuing from the previous page with no terminator anywhere";
    assert_eq!(sentence_start_before(text, text.len()), None);
}

#[test]
fn forward_expansion_reports_none_when_the_page_ends_mid_sentence() {
    // 65 of 499 (13.0%), measured.
    let text = "this page ends mid-sentence with no terminator";
    assert_eq!(sentence_end_after(text, 0), None);
}

#[test]
fn a_real_transcript_page_expands_to_both_boundaries() {
    // The mockup's own example: MR. SHARP's "we'll refine the claims" exchange,
    // on transcript page 25. Both ends must be sentence-complete, and neither may
    // stop at one of the four `MR.` tags in between.
    let quote = "we'll refine";
    let seed = at(TRANSCRIPT_P25, quote);
    let start = sentence_start_before(TRANSCRIPT_P25, seed).expect("a start exists on this page");
    let end =
        sentence_end_after(TRANSCRIPT_P25, seed + quote.len()).expect("an end exists on this page");

    let slice = &TRANSCRIPT_P25[start..end];
    assert!(
        slice.ends_with('.') || slice.ends_with('?') || slice.ends_with('!'),
        "the forward end must be a real terminator, got {:?}",
        &slice[slice.len().saturating_sub(40)..]
    );
    // The backward end must not have stopped inside a speaker tag.
    assert!(
        !slice.starts_with("PHILLIPS") && !slice.starts_with("SHARP"),
        "expansion stopped inside a speaker tag: {:?}",
        &slice[..40.min(slice.len())]
    );
}

#[test]
fn the_ocr_transposed_page_is_not_repaired_here() {
    // Transcript page 14 carries the Surya line-transposition wart: "I don't
    // know" and "if she got hers or not." arrive out of order. That is an OCR
    // defect and its fix belongs in OCR — a repair attempted here would be a
    // second, silent normalizer (ruling R4). This test pins the wart so nobody
    // "improves" this module into fixing it.
    assert!(
        TRANSCRIPT_P14.contains("I don't know"),
        "fixture must still carry the transposed fragment"
    );
    // The transposed fragment ends in a period, so expansion will legitimately
    // treat it as a boundary. That is the honest behaviour: the module reports
    // what the text says, warts and all.
    let seed = at(TRANSCRIPT_P14, "if she got hers");
    assert!(
        sentence_start_before(TRANSCRIPT_P14, seed).is_some(),
        "expansion still works on transposed text — it does not error, and it \
         does not silently reorder"
    );
}
