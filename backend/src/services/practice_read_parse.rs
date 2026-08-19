//! The one-sentence read: what is sent, and what is accepted back.
//!
//! Pure. No provider, no database, no clock — so every rule below is a unit test
//! instead of a call that costs money to observe.
//!
//! ## The two rules that make this a READ and not a chatbot
//!
//! 1. ONE line. The reply's first non-empty line is the whole of it; anything
//!    after is discarded, and a reply that needs more than a line is REFUSED.
//! 2. A LENGTH. A cap when it names a tactic, and a shorter one after the OK
//!    word. A witness reading feedback between reps can hold one sentence.
//!
//! ## Why the caps and the OK word ARRIVE rather than being written here
//!
//! They are settings rows, and this module cannot reach the store — so the only
//! way a number gets into the rule is for a caller to have read one. That is the
//! structural half of the same lesson the theme scan learned on 2026-08-09, when
//! a compiled-in 512 truncated 7 of 104 verdicts and nobody could change it
//! without a build. The OK word has a second reason: it is COUPLED to the prompt
//! file, which teaches the model to produce it, and both halves must be editable
//! by the same person in the same sitting.
//!
//! ## Why an over-long reply is a FAILURE and not a truncation
//!
//! Truncating would put half a sentence on a witness-prep screen — and half a
//! sentence about testimony can invert its meaning ("You corrected the premise,
//! but then you…"). The design says the read is never a paragraph and never a
//! score; the honest response to a model that wrote one is the same one used
//! when the model is unreachable: `read_text = null`, the screen says "no system
//! read this time", and the boxes stand.

/// A read that was accepted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReadLine {
    /// The one sentence, exactly as it will be stored and shown.
    pub text: String,
    /// True when the read said the answer was fine. Drives the green/red rule
    /// on screen and the `read_ok` column.
    pub ok: bool,
}

/// Why a reply was not usable.
///
/// These become `read_error` on the answer row — the log-side half of the
/// honesty: the screen shows one fixed line, this column says which failure it
/// was, so a wave of refusals is distinguishable from a wave of timeouts.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ReadRejection {
    #[error("the model replied with nothing")]
    Empty,

    #[error("the read was {words} words; the limit is {limit} — refused rather than truncated")]
    TooLong { words: usize, limit: usize },
}

/// The rules a reply is judged against, read from the store by the caller.
///
/// ## Rust Learning: borrowed fields and a lifetime
///
/// `fine_token: &'a str` borrows the settings snapshot rather than cloning a
/// `String` on every answer. The `'a` is what tells the compiler the struct may
/// not outlive the snapshot it points into — which is exactly the guarantee
/// wanted here, since a stale token would judge against wording the store no
/// longer holds.
#[derive(Debug, Clone, Copy)]
pub struct ReadRules<'a> {
    /// The most words a read may use when it names a tactic (task §5: 25).
    pub max_words: usize,
    /// The most words that may follow the OK word (task §5: six).
    pub max_words_after_fine: usize,
    /// The exact word the prompt reserves for "nothing wrong with that answer".
    pub fine_token: &'a str,
}

/// Take a model reply and decide whether it is a read.
///
/// ## Rust Learning: `trim_matches` over a slice of characters
///
/// Models sometimes wrap a one-line answer in quotation marks — straight or
/// curly, depending on the model and the prompt. `trim_matches(&['"', '“', '”'][..])`
/// strips any of them from BOTH ends in one call. The `[..]` turns the array into
/// a slice, which is the form `Pattern` is implemented for.
///
/// # Errors
/// [`ReadRejection::Empty`] for a blank reply, [`ReadRejection::TooLong`] for one
/// over its cap. Both are recorded as an absent read, never as a shortened one.
pub fn parse_read(raw: &str, rules: ReadRules<'_>) -> Result<ReadLine, ReadRejection> {
    let line = raw
        .lines()
        .map(str::trim)
        .find(|l| !l.is_empty())
        .ok_or(ReadRejection::Empty)?
        .trim_matches(&['"', '\u{201c}', '\u{201d}'][..])
        .trim();

    if line.is_empty() {
        return Err(ReadRejection::Empty);
    }

    // "Fine." and "Fine. Short, and yours." are the same verdict with different
    // amounts of encouragement; both are the OK arm, and only the tail is capped.
    if let Some(tail) = line.strip_prefix(rules.fine_token) {
        let words = tail.split_whitespace().count();
        if words > rules.max_words_after_fine {
            return Err(ReadRejection::TooLong {
                words,
                limit: rules.max_words_after_fine,
            });
        }
        return Ok(ReadLine {
            text: line.to_string(),
            ok: true,
        });
    }

    let words = line.split_whitespace().count();
    if words > rules.max_words {
        return Err(ReadRejection::TooLong {
            words,
            limit: rules.max_words,
        });
    }
    Ok(ReadLine {
        text: line.to_string(),
        ok: false,
    })
}

/// Everything the model is told about one answer.
///
/// A struct rather than seven `&str` arguments: six of them are strings and two
/// transpositions would silently swap her answer with the question, producing a
/// read that is confidently about the wrong text.
#[derive(Debug, Clone)]
pub struct ReadInputs<'a> {
    pub question: &'a str,
    /// The tactic's name, or `None` on a Chuck question — which has no trap.
    pub tactic: Option<&'a str>,
    pub side: &'a str,
    /// `cross`, `direct` or `redirect`. Domain note: prompt v2 judges the three
    /// by different rules — on cross the right answer is the short counter plus
    /// one named receipt and a paragraph is `That's redirect — save it for
    /// Chuck.`; on direct and redirect there is no length fault at all. `side`
    /// cannot carry that, because two of the three kinds are Chuck's.
    pub kind: &'a str,
    pub answer: &'a str,
    /// Her three points, in her words.
    pub points: &'a [String],
    pub watch_for: Option<&'a str>,
    /// The ALWAYS card, read from the same store the screen reads it from.
    pub always: &'a str,
}

/// Compose the user message.
///
/// ## Domain note: what is NOT in here
///
/// No case summary, no document, no other scenario, no graph. The model sees one
/// question, one answer, three sentences she wrote and one watch-for — which is
/// the whole of "nothing reads the whole graph" (design §5) expressed as an
/// LLM input rather than as a query.
pub fn build_user_message(inputs: &ReadInputs<'_>) -> String {
    let tactic = inputs.tactic.unwrap_or("none — this is a direct question");
    let watch = inputs
        .watch_for
        .unwrap_or("(no watch-for was written for this question)");
    let points = if inputs.points.is_empty() {
        "(none recorded)".to_string()
    } else {
        inputs
            .points
            .iter()
            .enumerate()
            .map(|(i, p)| format!("{}. {p}", i + 1))
            .collect::<Vec<_>>()
            .join("\n")
    };

    format!(
        "THE QUESTION ({side}): {question}\n\
         THE KIND: {kind}\n\
         THE TACTIC: {tactic}\n\n\
         HER ANSWER, verbatim:\n{answer}\n\n\
         HER THREE POINTS:\n{points}\n\n\
         THE WATCH-FOR: {watch}\n\n\
         THE ALWAYS CARD: {always}\n",
        side = inputs.side,
        kind = inputs.kind,
        question = inputs.question,
        answer = inputs.answer,
        always = inputs.always,
    )
}

#[cfg(test)]
#[path = "practice_read_parse_tests.rs"]
mod tests;
