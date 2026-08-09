// =============================================================================
// backend/src/domain/scenario_code.rs — the scenario's human handle (S-n)
// =============================================================================
//
// §2a (RATIFIED 2026-08-01) gives every scenario a short stable handle — S-1,
// S-2, … — assigned at creation, never changed, never reused, displayed wherever
// the scenario appears. This module owns how the stored ordinal becomes the
// string a human reads and says out loud.
//
// ## Why the BACKEND formats it
//
// The sibling candidate handle (`C-14`) is formatted in the frontend by
// `candidateChip`. This one deliberately is not, and the difference is the
// standing rule: no business logic in the frontend. A handle's spelling is part
// of the case's vocabulary — it appears in a rehearsal note, a margin scribble,
// and eventually a printed exhibit list — so the prefix belongs in one place that
// every surface reads, not re-typed in each component. The frontend receives
// `"S-3"` and renders it; it never learns that the separator is a hyphen or that
// the prefix is "S".
//
// (The candidate chip is not being changed here — rewiring a shipped, tested
// display is not this task, and its formatting is at least centralized in one
// helper. Worth aligning when the card work lands in 1.3.)

/// The prefix distinguishing a scenario handle from a candidate handle (`C-`).
///
/// ## Rust Learning: a `const` for a vocabulary token, not a config value
///
/// Standing Rule 2 asks whether a value varies across environments, cases or
/// deployments. This one cannot: `S-3` is a name humans have already said aloud,
/// and a deployment that rendered it `SC-3` would be renaming every reference in
/// every notebook. It is law, versioned with the code — the same reasoning that
/// keeps the connection-tier partition out of YAML.
const SCENARIO_CODE_PREFIX: &str = "S-";

/// Render a scenario's stored ordinal as its display handle.
///
/// ## Why this takes the ordinal rather than the whole record
///
/// The formatting depends on exactly one field, so taking only that field means
/// this function is callable from a DTO builder that holds a record, from a log
/// line that holds a number, and from a test that holds neither — with no struct
/// to construct. It also makes the function trivially total: every `i32` maps to a
/// string, so there is no error path and no `Option` for a caller to unwrap.
///
/// Negative ordinals are unreachable — the column carries a `CHECK`-backed
/// non-negative sequence and is assigned only by `insert_scenario` — so they get
/// no special handling here. If one ever appeared it would render as `S--1`,
/// which is visibly wrong rather than silently plausible, and that is the right
/// failure mode for an impossible value.
pub fn scenario_code(ordinal: i32) -> String {
    format!("{SCENARIO_CODE_PREFIX}{ordinal}")
}

/// The prefix on a CANDIDATE fact's handle (`C-14`).
///
/// Same standing as `S-`, and the same argument: a human says "C-14" out loud and
/// writes it in a margin, so it is case vocabulary versioned with the code rather
/// than a deployment tunable.
const CANDIDATE_CODE_PREFIX: &str = "C-";

/// Render a candidate fact's stored ordinal as its display handle.
///
/// ## Why this exists now, when the module header said it did not (task 2.11)
///
/// The header above recorded the asymmetry and its cost: the candidate handle was
/// formatted in `services::scenario_card` and, before that, in the browser. Task
/// 2.11's accusation gaps have to NAME the fact they are about — a gap list of
/// forty-six items that look alike is useless without the handle — so a third site
/// would have been spelling `C-` for itself. Two sites that must agree about a
/// name humans have already said aloud is one site too many, so the spelling moved
/// here beside its sibling and both callers read it.
///
/// Total, like `scenario_code`: every `i32` maps to a string, so there is no error
/// path. A candidate nothing has numbered yet has no ordinal at all, and the
/// caller's `Option::map` is where that absence lives — never a `C-0`, which would
/// name a card that does not exist.
pub fn candidate_code(ordinal: i32) -> String {
    format!("{CANDIDATE_CODE_PREFIX}{ordinal}")
}

/// The prefix on a complaint ALLEGATION's handle (`A-45`).
///
/// ## Why the pilcrow had to go (ONE_CARD_GRAMMAR, §3.3, ratified 2026-08-09)
///
/// It was `¶45`, and the number was already right — `A-45` IS complaint ¶45, the
/// same paragraph, renamed rather than renumbered. What changed is that the
/// handle became TYPEABLE. §2a's whole argument for codes is that a human says
/// them aloud and writes them down; the type-ahead that replaced the
/// 120-checkbox wall adds a third requirement, which is that they can be typed
/// into a box. Nobody can type `¶`, so a prefix a keyboard cannot produce was a
/// handle in name only.
///
/// Same standing as `S-` and `C-`: case vocabulary versioned with the code, not
/// a deployment tunable. A build that rendered it `P-45` would be renaming every
/// reference in every notebook.
const ALLEGATION_CODE_PREFIX: &str = "A-";

/// Render a complaint allegation's paragraph as its display handle.
///
/// ## Rust Learning: `&str` in, `String` out — and why not `i32` like its siblings
///
/// The two handles above take an `i32` because their ordinals are integer
/// columns. A complaint paragraph is stored as TEXT and is not always a bare
/// number: the corpus carries values like `72(a)`, and parsing them into an
/// integer would either lose the suffix or force an error path onto a function
/// that has no honest way to fail. Taking the stored string across means this
/// renders whatever the complaint actually says, which is the §12 discipline
/// applied to a label.
///
/// The caller is responsible for not passing a blank — see `accusation_text`,
/// where an absent or whitespace-only paragraph means the label is the body
/// alone rather than a handle with nothing after the hyphen.
pub fn allegation_code(paragraph: &str) -> String {
    format!("{ALLEGATION_CODE_PREFIX}{paragraph}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_the_ratified_shape() {
        // The exact strings humans will say and write. Pinned so a well-meaning
        // edit to zero-pad or bracket them fails the build — every such change
        // silently invalidates references already made outside this system.
        assert_eq!(scenario_code(1), "S-1");
        assert_eq!(scenario_code(3), "S-3");
        assert_eq!(scenario_code(42), "S-42");
    }

    #[test]
    fn does_not_pad_or_truncate_large_ordinals() {
        assert_eq!(scenario_code(100), "S-100");
        assert_eq!(scenario_code(9999), "S-9999");
    }

    #[test]
    fn is_distinguishable_from_a_candidate_handle() {
        // S-14 and C-14 name completely different things — a scenario and a
        // candidate fact inside one. If the prefixes ever collided, a rehearsal
        // reference would be ambiguous, so assert they differ rather than trusting
        // two constants in two files to stay apart.
        assert!(scenario_code(14).starts_with("S-"));
        assert_ne!(scenario_code(14), "C-14");
    }

    #[test]
    fn a_candidate_handle_renders_the_shape_humans_already_say() {
        // Pinned as literals for the same reason `S-n` is: these strings are
        // already written in Roman's notes, and a well-meaning edit to zero-pad
        // them would invalidate every reference made outside this system.
        assert_eq!(candidate_code(1), "C-1");
        assert_eq!(candidate_code(14), "C-14");
        assert_eq!(candidate_code(148), "C-148");
    }

    #[test]
    fn an_allegation_handle_keeps_the_complaint_s_own_number() {
        // A-45 IS complaint ¶45. The task renamed the prefix and renumbered
        // nothing, so a build that re-sequenced these would silently break every
        // cross-reference between this system and the filed complaint.
        assert_eq!(allegation_code("45"), "A-45");
        assert_eq!(allegation_code("7"), "A-7");
    }

    #[test]
    fn an_allegation_handle_survives_a_paragraph_that_is_not_a_bare_number() {
        // The corpus carries sub-lettered paragraphs. Parsing to an integer would
        // drop the suffix and point the reader at the wrong paragraph, which is
        // worse than an ugly handle.
        assert_eq!(allegation_code("72(a)"), "A-72(a)");
    }

    #[test]
    fn an_allegation_handle_can_be_typed_on_a_keyboard() {
        // The reason the pilcrow left: the type-ahead that replaced the
        // 120-checkbox wall needs a prefix a human can produce. Asserted as a
        // property rather than as a literal, because what matters is not that it
        // is "A" but that it is reachable — a future rename to another ASCII
        // letter is fine and a rename back to a glyph is not.
        let code = allegation_code("45");
        assert!(
            code.chars().all(|c| c.is_ascii()),
            "an allegation handle must be typeable: {code} is not ASCII"
        );
    }

    #[test]
    fn the_two_handles_can_never_be_confused_for_each_other() {
        // S-14 and C-14 name a scenario and a fact inside one. The prefixes are
        // two constants that must stay apart, so assert it rather than trust it.
        assert_ne!(scenario_code(14), candidate_code(14));
        assert!(candidate_code(14).starts_with("C-"));
    }
}
