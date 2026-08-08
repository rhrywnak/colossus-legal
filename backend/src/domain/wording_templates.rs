// =============================================================================
// backend/src/domain/wording_templates.rs — the rules for ALL stored text
// =============================================================================
//
// Split out of `domain::wording` in task 2.11 B2, and the seam is one that was
// always there: these are the rules for EVERY stored string, and there are now
// five modules' worth — `wording` (48 keys, the curation surfaces),
// `wording_accusation` (27, the working view's accusation section),
// `wording_rehearsal` (41, the rehearsal page's prose and gaps),
// `wording_rehearsal_chrome` (18, that page's controls and markers, task 2.11 C),
// `wording_authoring` (23, the two shared authoring sections, task 2.11 C).
//
// Two rules live here.
//
// **Which templates must keep their facts.** A template edited to drop a
// placeholder is well-formed text AND a sentence with the fact removed — "You
// linked this to  · they'll use it against us.", or a collapsed section reading
// "said  times ·  gaps". Nothing downstream can detect either, because a template
// with no placeholder renders perfectly. So the write path checks this table
// before storing, and the refusal NAMES what is missing.
//
// **How a template is filled.** One scan of the template, emitting values as it
// goes, so a value substituted early is never re-scanned by a later pass.
//
// ## Why there is exactly ONE table, spanning all three key lists
//
// It is the one place the settings write path looks. A per-module table would be
// a per-module lookup that can silently miss — and a key absent from the lookup is
// silently unconstrained, which is the precise failure this table exists to
// prevent.

use crate::domain::settings::{parse_text, SettingError};
use crate::domain::wording as curation;
use crate::domain::wording_accusation as accusation;
use crate::domain::wording_authoring as authoring;
use crate::domain::wording_rehearsal as rehearsal;
use crate::domain::wording_rehearsal_chrome as chrome;
use crate::domain::wording_scan as scan;

/// The placeholders a stored string MUST still contain after a human edits it.
///
/// ## Why this is enforced rather than trusted
///
/// `link_summary_template` reads "You linked this to {allegations} · {cut}." An
/// edit that dropped `{allegations}` would produce "You linked this to  · they'll
/// use it against us." — a grammatical sentence with the FACT removed, on a
/// surface a lawyer reads. Nothing downstream could detect that, because a
/// template with no placeholder renders perfectly well.
///
/// So the write path checks this table before storing, and refuses with the
/// missing names. Keys absent from this table have no required placeholders,
/// which is most of them.
pub const REQUIRED_PLACEHOLDERS: &[(&str, &[&str])] = &[
    (curation::KEY_SHOW_ALL_LABEL, &["{count}"]),
    (curation::KEY_SUMMARY_TEMPLATE, &["{allegations}", "{cut}"]),
    (curation::KEY_PROGRESS_TEMPLATE, &["{linked}", "{total}"]),
    (curation::KEY_SAVE_FAILED_TEMPLATE, &["{detail}"]),
    // A confirmation that does not name what it is about is not a confirmation.
    (curation::KEY_FACT_REMOVE_CONFIRM, &["{code}"]),
    (curation::KEY_FACT_REMOVE_FAILED, &["{detail}"]),
    // A pile that cannot say how big it is reads as an empty one.
    (curation::KEY_FACT_BACKGROUND_COUNT, &["{count}"]),
    // A failure that names neither the fact nor the cause strands the human on a
    // surface where several rows look alike.
    (curation::KEY_FACT_TIER_SAVE_FAILED, &["{code}", "{reason}"]),
    (
        curation::KEY_FACT_ORDER_SAVE_FAILED,
        &["{code}", "{reason}"],
    ),
    // A card that vanishes without naming itself is the defect this prevents.
    (curation::KEY_FACT_BG_MOVE_NOTICE, &["{code}"]),
    // The old footer called folded facts "shown"; only two numbers are honest.
    (curation::KEY_FACT_FOOTER, &["{shown}", "{background}"]),
    // ── Task 2.11: the accusation section's templates ────────────────────────
    //
    // Listed HERE rather than in a second table beside their own keys, because
    // this table is the ONE place the write path looks (`validate_wording_
    // candidate`). A second table would be a second lookup that can silently
    // miss — and a key absent from the lookup is silently unconstrained, which is
    // precisely the failure this table exists to prevent.
    //
    // "Said  times, in  documents." is a grammatical sentence with both facts
    // removed, and nothing downstream could tell.
    (accusation::KEY_COUNT_TEMPLATE, &["{times}", "{documents}"]),
    // "None marked" and "none marked, out of forty-six waiting" are different
    // states of the same scenario, and only the second says what to do next.
    (accusation::KEY_NO_INSTANCES_NOTICE, &["{included}"]),
    // A gap that cannot name which fact it is about is useless on a list of
    // forty-six that look alike — and the design calls this list the single most
    // useful thing on the page.
    (accusation::KEY_GAP_NO_ANSWER, &["{code}"]),
    (accusation::KEY_GAP_ACCUSATION_REMOVED, &["{code}"]),
    (accusation::KEY_GAP_ANSWER_REMOVED, &["{code}"]),
    (accusation::KEY_SAVE_FAILED_TEMPLATE, &["{detail}"]),
    // ── Task 2.11 B2: the rehearsal page's templates ─────────────────────────
    //
    // Same one-table rule as above. The collapsed-section headers are the ones
    // that matter most: "said  times ·  gaps" is a prep list reporting no work,
    // on the surface whose whole promise is that its counts are honest.
    (rehearsal::KEY_POSITION_TEMPLATE, &["{n}", "{total}"]),
    (rehearsal::KEY_NOT_READY, &["{code}"]),
    (rehearsal::KEY_ACCUSATION_HEADER, &["{times}", "{gaps}"]),
    (rehearsal::KEY_TIMELINE_HEADER, &["{entries}"]),
    (rehearsal::KEY_POINTS_HEADER, &["{shown}", "{cap}"]),
    (rehearsal::KEY_WATCH_HEADER, &["{count}"]),
    // A gap that cannot say WHICH statement is useless: this surface never names
    // a fact by an internal handle, so who/when/where is the only way to find it.
    (
        rehearsal::KEY_GAP_NO_ANSWER,
        &["{who}", "{when}", "{where}"],
    ),
    (rehearsal::KEY_GAP_ANSWER_REMOVED, &["{who}", "{when}"]),
    (rehearsal::KEY_TIMELINE_GAP, &["{undated}", "{total}"]),
    (rehearsal::KEY_SOURCE_LABEL, &["{document}", "{page}"]),
    // ── Task 2.11 C: authorship, and the two authoring sections ──────────────
    //
    // "Written in plain words by  · " attributes nothing while looking like an
    // attribution — the worst of both, on the one line whose entire job is to say
    // a human wrote this sentence rather than a machine deriving it.
    (chrome::KEY_WHAT_ATTRIBUTION, &["{who}", "{when}"]),
    (chrome::KEY_ACCUSATION_ATTRIBUTION, &["{who}", "{when}"]),
    // The cap is a stored number. "her own words · up to " states a limit and
    // then withholds it, and "That is already  points" refuses without saying
    // what the ceiling is — a control that refuses opaquely reads as broken.
    (authoring::KEY_POINTS_SECTION_META, &["{cap}"]),
    (authoring::KEY_POINTS_CAP_REACHED, &["{cap}"]),
    // Without {n} every editing box in the list announces itself identically to a
    // screen reader, which is the same as none of them being labelled.
    (authoring::KEY_POINTS_FIELD_LABEL, &["{n}"]),
    // ── Task 2.15 Tier 2: the scan's own words ───────────────────────────────
    //
    // The conservation line IS its five numbers. An edit that dropped one would
    // leave a sentence claiming to reconcile a pool while withholding the term
    // that does not add up — worse than no line at all, because it looks checked.
    (
        scan::KEY_CONSERVATION_LINE,
        &[
            "{pool}",
            "{collapsed}",
            "{excluded}",
            "{judged}",
            "{relevant}",
        ],
    ),
    // A destructive confirmation that cannot name WHICH run is being destroyed is
    // the mis-click guard failing quietly on a list of four look-alike rows.
    (scan::KEY_HISTORY_DELETE_CONFIRM, &["{run}"]),
    // "Browse the raw evidence pool" without its size hides how much is behind
    // the control — which is the whole reason it is behind one.
    (curation::KEY_QUEUE_RAW_POOL_TOGGLE, &["{count}"]),
];

/// Which required placeholders a candidate value is missing, for one key.
///
/// Returns an empty `Vec` for a key with no requirements and for a value that
/// satisfies all of them — so the caller's check is `is_empty()` either way, with
/// no special case for "this key is unconstrained".
///
/// ## Rust Learning: returning owned `&'static str`s, not a bool
///
/// A `bool` would force the caller to re-derive WHICH placeholder was missing in
/// order to say so, and the refusal has to name them (Standing Rule 1: a failure
/// a reader cannot diagnose is incomplete). The items are `&'static str` because
/// they come from the table above, which lives for the whole program — no
/// lifetime annotation is needed on the signature for the same reason.
pub fn missing_placeholders(key: &str, value: &str) -> Vec<&'static str> {
    REQUIRED_PLACEHOLDERS
        .iter()
        .find(|(k, _)| *k == key)
        .map(|(_, required)| {
            required
                .iter()
                .filter(|token| !value.contains(**token))
                .copied()
                .collect()
        })
        .unwrap_or_default()
}

/// Check a proposed wording value: non-blank, and still carrying its facts.
///
/// The write path's whole rule for a `text` parameter, in one place. Lives here
/// rather than in the settings store because both halves are statements about
/// WORDS — what a label may not be, and what a template must keep — and because
/// the store is at its module-size limit.
///
/// # Errors
/// Returns [`SettingError::Blank`] for an empty value, or
/// [`SettingError::MissingPlaceholders`] naming every placeholder an edit dropped.
pub fn validate_wording_candidate(key: &str, candidate: &str) -> Result<(), SettingError> {
    let text = parse_text(key, candidate)?;

    // The rule a parse cannot catch: "You linked this to {allegations}" edited to
    // "You linked this to it" is well-formed text AND a sentence with the fact
    // removed. Nothing downstream could tell, because a template with no
    // placeholder renders perfectly.
    let missing = missing_placeholders(key, &text);
    if !missing.is_empty() {
        return Err(SettingError::MissingPlaceholders {
            key: key.to_string(),
            missing: missing.join(", "),
        });
    }
    Ok(())
}

/// Fill a stored template's `{placeholders}` from a list of (name, value) pairs.
///
/// ## Why this is a plain scan and not a templating crate
///
/// The whole vocabulary is three placeholders across two templates. A dependency
/// with an expression language would let a stored string reach for data this
/// module never meant to expose, which is a surface no configuration value should
/// have. Substring replacement of an exact `{name}` cannot.
///
/// ## What happens to a placeholder nobody supplied
///
/// Nothing — it stays on screen as `{name}`, visibly wrong. That is deliberate,
/// and it is the honest end of a chain whose other end is
/// [`missing_placeholders`]: a template can only lose a placeholder through an
/// edit the write path refuses, so a `{name}` reaching a screen means the store
/// was edited around the API (a `psql` UPDATE), and showing the token is how a
/// reader finds out. Substituting an empty string instead would produce a
/// confident sentence with a hole in it.
/// ## Rust Learning: why this is ONE scan and not a `replace` per placeholder
///
/// The obvious implementation is `for (name, value) { out = out.replace(…) }`.
/// It has a real bug: a value substituted by an early pass is re-scanned by every
/// later one. An accusation whose own text contains `{cut}` — arbitrary human
/// prose from a complaint — would have that chewed out of it. Walking the
/// TEMPLATE once and emitting values as it goes means a substituted value is
/// never looked at again.
pub fn render(template: &str, values: &[(&str, &str)]) -> String {
    let mut out = String::with_capacity(template.len());
    let mut rest = template;

    while let Some(open) = rest.find('{') {
        out.push_str(&rest[..open]);
        let after = &rest[open + 1..];

        let Some(close) = after.find('}') else {
            // An unclosed brace is not a placeholder. Emit it and stop scanning —
            // the remainder is literal text.
            out.push_str(&rest[open..]);
            return out;
        };

        let name = &after[..close];
        match values.iter().find(|(key, _)| *key == name) {
            Some((_, value)) => out.push_str(value),
            // Unknown: emit the token verbatim. See the doc above for why this is
            // deliberate rather than substituting an empty string.
            None => out.push_str(&rest[open..open + close + 2]),
        }
        rest = &after[close + 1..];
    }

    out.push_str(rest);
    out
}
