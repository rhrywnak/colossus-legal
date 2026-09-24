//! What the question chat is told about the question — the text block after the
//! documents. Pure: it renders a [`QuestionContext`] the gather step collected.
//!
//! Sections, in order: the question · the document it rests on · the attack it
//! answers · her talking points · the sworn pair · her answers and their reads ·
//! the team's standing notes · the other threads on this question · the earlier
//! shared discussion (ADDENDUM_1). Every section says so plainly when it is
//! empty — an empty section and a missing section must not look alike to the
//! model (Standing Rule 1).
//!
//! ## Why the primary document is NAMED here and not positioned in the corpus
//!
//! Until v2.2.2 the question's source document was sorted to the front of the
//! document blocks. Those blocks are the prompt cache's prefix, so reordering them
//! per question destroyed the cache and cost 62% of the feature's spend
//! (CC_TASK_CHAT_COST_AUDIT_v1). This block sits AFTER the last cache breakpoint,
//! so a sentence here is free — and a model reads a name better than a position:
//! a document in slot one is not self-describing, a title and a date are.

use chrono::NaiveDate;

use crate::repositories::pipeline_repository::practice_discussions::DiscussionTurnRecord;
use crate::repositories::pipeline_repository::practice_notes::{AttemptRecord, NoteRecord};
use crate::services::chat_question_text::{document_date_label, translate_keys};
use crate::services::practice_clock::local_stamp;

/// What stands in for the analysis when the analysis FAILED — said as such,
/// never Marie's failure sentence passed along as though it were an analysis.
///
/// Moved here from `practice_discuss` when the dock that also used it was retired
/// (CC_TASK_MODEL_JOBS_PANEL_v1, ruling Q1).
// STRUCTURAL: model-wire vocabulary — the model-facing label for a failed
// analysis; rephrasing it is a coordinated prompt change.
pub const FAILED_ANALYSIS: &str = "no answer analysis (it failed)";

/// One other person's thread, as the model reads it.
#[derive(Debug, Clone, PartialEq)]
pub struct SiblingThread {
    pub owner_name: String,
    /// `(speaker, text)`, oldest first.
    pub lines: Vec<(String, String)>,
}

/// Everything the package is rendered from.
#[derive(Debug, Clone)]
pub struct QuestionContext {
    pub scenario_code: String,
    pub question_text: String,
    /// `cross` / `direct` / `redirect`.
    pub kind: String,
    /// Who asks: the defense lawyer or her own lawyer, in words.
    pub asker: String,
    pub tactic_name: Option<String>,
    /// The title of the document this question was built from. `None` = the graph
    /// could not resolve one; the block says so rather than staying silent.
    pub primary_title: Option<String>,
    /// That document's date, when the record holds one.
    pub primary_date: Option<NaiveDate>,
    pub attack: Option<String>,
    pub watch_for: Option<String>,
    /// `(number, text, receipt)`.
    pub points: Vec<(i32, String, Option<String>)>,
    pub pair_said: Option<String>,
    pub pair_admitted: Option<String>,
    pub attempts: Vec<AttemptRecord>,
    pub notes: Vec<NoteRecord>,
    pub siblings: Vec<SiblingThread>,
    pub earlier: Vec<DiscussionTurnRecord>,
    pub witness_name: String,
    /// Who is writing this turn, by name.
    pub viewer_name: String,
    pub case_timezone: String,
}

/// What an empty section says. STRUCTURAL: prose for the MODEL, not the screen —
/// the prompt file is where the model's instructions live; these only label data.
const NONE_RECORDED: &str = "(none recorded)";

/// Names the document a question was built from, filled with title and date.
///
/// STRUCTURAL: prose for the MODEL, the same class as `NONE_RECORDED` above — not
/// a witness-facing sentence, so not a wording-store row (ruled 2026-09-23, Q5).
/// The wording is the task's, verbatim.
const RESTS_ON: &str = "This question rests mainly on {title}, {date}.";

/// The same sentence for a document the case record holds no date for.
///
/// STRUCTURAL, as above. It exists because the dated sentence cannot be reused
/// with an empty slot: "rests mainly on X, ." would read as a formatting fault,
/// and substituting a placeholder date would state something the record does not
/// say. An undated document says so, in the same words its own document block
/// uses (`chat_question_text::UNDATED`), only mid-sentence.
const RESTS_ON_UNDATED: &str = "This question rests mainly on {title}, date not recorded.";

/// What the block says when the graph could not resolve the primary document.
///
/// STRUCTURAL, as above. It is a DIFFERENT sentence rather than an omission: a
/// question with no resolvable source and a question whose source lookup failed
/// must not read to the model as a question that simply has no source
/// (Standing Rule 1). `gather` logs the graph failure separately.
const RESTS_ON_UNKNOWN: &str =
    "The document this question was built from is not recorded. Every document in \
     the case record is above; do not guess which one it came from.";

/// Render the context block.
pub fn render_context(c: &QuestionContext) -> String {
    let mut out = String::new();
    section(&mut out, "THE QUESTION", &question_lines(c));
    section(
        &mut out,
        "THE DOCUMENT THIS QUESTION RESTS ON",
        &rests_on(c),
    );
    section(
        &mut out,
        "THE ATTACK THIS SCENARIO ANSWERS",
        &c.attack.clone().unwrap_or_else(|| NONE_RECORDED.into()),
    );
    section(
        &mut out,
        &format!("{}'S TALKING POINTS", c.witness_name.to_uppercase()),
        &points(c),
    );
    section(&mut out, "THE SWORN PAIR", &pair(c));
    section(
        &mut out,
        &format!(
            "{}'S ANSWERS AND THEIR READS, OLDEST FIRST",
            c.witness_name.to_uppercase()
        ),
        &render_attempts(c),
    );
    section(
        &mut out,
        "STANDING NOTES FROM COUNSEL AND THE TEAM",
        &notes(c),
    );
    section(&mut out, "THE OTHER THREADS ON THIS QUESTION", &siblings(c));
    section(
        &mut out,
        "THE EARLIER SHARED DISCUSSION OF THIS QUESTION",
        &earlier(c),
    );
    section(&mut out, "WHO IS WRITING NOW", &c.viewer_name);
    translate_keys(&out)
}

fn section(out: &mut String, heading: &str, body: &str) {
    out.push_str("## ");
    out.push_str(heading);
    out.push('\n');
    out.push_str(if body.trim().is_empty() {
        NONE_RECORDED
    } else {
        body
    });
    out.push_str("\n\n");
}

/// The primary-document line: the title and date of the document the question was
/// built from, or the explicit "not recorded" sentence.
///
/// ## Rust Learning: `Option::as_deref` and matching a pair
///
/// `primary_title` is an `Option<String>`; `as_deref()` borrows it as
/// `Option<&str>` so the match arms compare and format without cloning. Matching
/// the `(title, date)` pair rather than nesting two `if let`s is what makes the
/// third state — a title with no date — impossible to forget; the compiler
/// refuses the match until every combination is answered.
fn rests_on(c: &QuestionContext) -> String {
    match c.primary_title.as_deref() {
        None => RESTS_ON_UNKNOWN.to_string(),
        Some(title) => match document_date_label(c.primary_date) {
            Some(date) => RESTS_ON.replace("{title}", title).replace("{date}", &date),
            None => RESTS_ON_UNDATED.replace("{title}", title),
        },
    }
}

fn question_lines(c: &QuestionContext) -> String {
    let mut s = format!(
        "{} · {} · asked by {}\n\"{}\"",
        c.scenario_code, c.kind, c.asker, c.question_text
    );
    if let Some(t) = &c.tactic_name {
        s.push_str(&format!("\nIts tactic: {t}"));
    }
    if let Some(w) = &c.watch_for {
        s.push_str(&format!("\nWatch for: {w}"));
    }
    s
}

fn points(c: &QuestionContext) -> String {
    c.points
        .iter()
        .map(|(n, text, receipt)| match receipt {
            Some(r) => format!("{n}. {text} — backed by: {r}"),
            None => format!("{n}. {text} — no receipt named"),
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn pair(c: &QuestionContext) -> String {
    match (&c.pair_said, &c.pair_admitted) {
        (None, None) => String::new(),
        (said, admitted) => format!(
            "What they said: {}\nWhat they admitted under oath: {}",
            said.as_deref().unwrap_or(NONE_RECORDED),
            admitted.as_deref().unwrap_or(NONE_RECORDED)
        ),
    }
}

/// Her answers and their reads — also what `get_answer_history` returns.
pub(crate) fn render_attempts(c: &QuestionContext) -> String {
    c.attempts
        .iter()
        .map(|a| {
            let read = match (&a.read_text, a.read_ok) {
                // ADDENDUM_1: a failed analysis is said as such, never passed on
                // as Marie's failure sentence.
                _ if a.read_failed() => format!("Read: {FAILED_ANALYSIS}"),
                (Some(t), Some(true)) => format!("Read (marked fine): {t}"),
                (Some(t), _) => format!("Read: {t}"),
                (None, _) => "Read: none stored".to_string(),
            };
            format!(
                "{} — answered: \"{}\"\n{read}",
                local_stamp(a.answered_at, &c.case_timezone),
                a.answer_text
            )
        })
        .collect::<Vec<_>>()
        .join("\n\n")
}

fn notes(c: &QuestionContext) -> String {
    c.notes
        .iter()
        .map(|n| {
            format!(
                "{} ({}): {}",
                n.author,
                local_stamp(n.created_at, &c.case_timezone),
                n.text
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn siblings(c: &QuestionContext) -> String {
    c.siblings
        .iter()
        .map(|t| {
            let lines: Vec<String> = t
                .lines
                .iter()
                .map(|(who, text)| format!("{who}: {text}"))
                .collect();
            format!("### {}'s thread\n{}", t.owner_name, lines.join("\n"))
        })
        .collect::<Vec<_>>()
        .join("\n\n")
}

fn earlier(c: &QuestionContext) -> String {
    c.earlier
        .iter()
        .map(|t| {
            format!(
                "{} ({}): {}",
                t.author_name,
                local_stamp(t.created_at, &c.case_timezone),
                t.text
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
#[path = "chat_question_context_tests.rs"]
mod tests;
