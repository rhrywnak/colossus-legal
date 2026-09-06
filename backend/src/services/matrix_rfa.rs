// =============================================================================
// backend/src/services/matrix_rfa.rs — a request for admission, on one line
// =============================================================================
//
// PROOF_MATRIX_v2 §3: "RFA/answer-only cards (card has `question`): render
// 'RFA {n} — {request text} — {answer}'; derive `n` from the card title/number
// where it exists, else omit `RFA n`."
//
// ## Why the backend composes this and the browser does not
//
// The house language law: the frontend substitutes, it never composes. Both the
// drill-down and the Word export print this line, and a sentence assembled in two
// places is a sentence that reads two ways. So the finished line is served, from
// stored templates, and the renderer prints what it is given.
//
// ## Why an answer-only card is a problem worth solving at all
//
// Measured on DEV: 367 Evidence nodes carry a `question`, and their
// `verbatim_quote` is the ANSWER alone — "Admitted", "No.", "No, not that I
// recall." A list of those quotes under an accusation is a column of bare
// affirmations with nothing to say what was affirmed. The request is the half
// that carries the meaning, and it is already on the node.
//
// ## Domain note: the number is taken from `paragraph`, and only when it says RFA
//
// The card's number lives in `Evidence.paragraph`: `"RFA 17"` on the admissions
// response, `"Q19"` on the interrogatories. §3 says to omit `RFA n` when no
// number can be derived — and printing "RFA 19" over interrogatory Q19 would be
// worse than omitting it, because a wrong RFA number in a document bound for a
// hearing is a citation to a request that says something else. So the number is
// used ONLY when the field names an RFA; an interrogatory renders through the
// unnumbered template, which is exactly the "else" branch §3 asks for.

use crate::domain::wording_matrix::MatrixWording;

/// The prefix `Evidence.paragraph` carries on a request for admission.
// CONST: an extraction vocabulary token, not a deployment value — it is what the
// pass wrote into the graph, in the same category as a relationship type.
const RFA_PREFIX: &str = "RFA";

/// The finished one-line rendering of a Q&A card, or `None` when this item is not
/// one.
///
/// Returns `None` — meaning "print the verbatim quote as usual" — for any item
/// with no `question`, and for one whose question or answer is blank. A card
/// carrying a question and an empty answer is a real state (an unanswered
/// request), and rendering "— Admit that … — " would print a template with its
/// point removed; the plain quote is the honest fallback.
///
/// ## Rust Learning: `Option<&str>` parameters instead of `&Option<String>`
///
/// The caller holds `Option<String>` fields and passes `.as_deref()`. Taking
/// `Option<&str>` means this function borrows rather than forcing a clone, and it
/// can be called from a test with plain literals.
pub fn rfa_line(
    question: Option<&str>,
    answer: Option<&str>,
    paragraph: Option<&str>,
    wording: &MatrixWording,
) -> Option<String> {
    let request = non_blank(question)?;
    let reply = non_blank(answer)?;

    match rfa_number(paragraph) {
        Some(number) => Some(
            wording
                .rfa_template
                .replace("{number}", number)
                .replace("{request}", request)
                .replace("{answer}", reply),
        ),
        None => Some(
            wording
                .rfa_unnumbered_template
                .replace("{request}", request)
                .replace("{answer}", reply),
        ),
    }
}

/// The trimmed value, or `None` when it is absent or all whitespace.
///
/// Absent and blank are collapsed HERE, deliberately and in one place: both mean
/// "there is no text to put in this slot", and the caller's decision is the same
/// for either. They stay distinguishable upstream, where the row still carries
/// whichever it was.
fn non_blank(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|v| !v.is_empty())
}

/// The request number from a `paragraph` field, when that field names an RFA.
///
/// Accepts `"RFA 17"` and `"RFA17"`, case-insensitively, and returns the digits
/// as they appear. Returns `None` for `"Q19"`, for `"RFA"` with no digits, and
/// for anything that carries trailing text after the number — a value this
/// function does not fully understand must not be half-read into a citation.
///
/// ## Rust Learning: returning a `&str` borrowed from the argument
///
/// The digits are a SLICE of the caller's string, so nothing is allocated and the
/// returned reference cannot outlive the input. Rust infers that lifetime from
/// the single reference parameter, which is why no `'a` is written.
fn rfa_number(paragraph: Option<&str>) -> Option<&str> {
    let text = non_blank(paragraph)?;
    let rest = text
        .get(..RFA_PREFIX.len())
        .filter(|head| head.eq_ignore_ascii_case(RFA_PREFIX))?;
    let digits = text[rest.len()..].trim();
    if digits.is_empty() || !digits.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    Some(digits)
}

#[cfg(test)]
#[path = "matrix_rfa_tests.rs"]
mod tests;
