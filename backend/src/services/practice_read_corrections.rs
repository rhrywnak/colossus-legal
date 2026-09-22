//! The read prompt file, cut in two: the system prompt, and the corrections a
//! re-request sends.
//!
//! Pure. No provider, no database, no disk — [`split_prompt`] is handed the
//! file's text by `practice_read_setup`, so every rule below is a unit test.
//!
//! ## Why the corrections live in the PROMPT FILE (GO ruling, 2026-09-21)
//!
//! Before v2.2.1 a rejected reply was simply asked for again, with the same
//! message, and a model that had named `R2` in its prose had no way to know
//! that was the fault — so the second attempt tended to repeat the first. Now
//! attempt 2 carries the model's own rejected reply and ONE sentence naming what
//! was wrong. Those sentences are words said to a MODEL, which makes them prompt
//! material: version-controlled, diffable and md5-verified on push, exactly like
//! the system prompt beside them — never a code literal, and never a settings row
//! an operator edits in a browser with no review.
//!
//! ## Why the file is CUT, and the cut is code-enforced
//!
//! Everything after [`CORRECTIONS_MARKER`] is a different audience: the model
//! sees each correction only when a reply was sent back, and never as part of
//! the standing instructions. CLAUDE.md Rule 1 asks for audience boundaries that
//! are enforced by code rather than by convention — so this module splits the
//! file, and `practice_read_setup` sends only the half before the line.
//!
//! ## A file without the marker is legitimate
//!
//! v4 has none. Pointing `practice_read_prompt_file` back at v4 is the one-UPDATE
//! rollback the migration promises, and it must keep working: with no marker the
//! read re-requests plainly, exactly as it did before v5, and says so in the log.
//! A file WITH the marker but missing a section is a broken deploy, and is
//! refused by name.

use crate::services::practice_read_parse::ReplyRejection;

/// The line that divides the system prompt from the corrections.
///
/// STRUCTURAL: it is the boundary this code enforces, so it must match the file
/// byte for byte — the same reasoning that keeps the reply's field names in code
/// (`practice_read_parse::FIELD_CALL`). `the_v5_prompt_file_carries_every_correction`
/// reads the shipped file and fails the build if the two ever disagree.
// STRUCTURAL: this string is the code-enforced boundary between the system prompt
// and the re-request corrections; it must match the prompt file byte for byte,
// exactly as FIELD_CALL must match the reply format the model is taught.
pub const CORRECTIONS_MARKER: &str = "<!-- RE-REQUEST CORRECTIONS -->";

/// Section headings below the marker, one per correction plus the frame.
///
/// STRUCTURAL for the same reason as the marker: they are how this code finds
/// each sentence in the file.
// STRUCTURAL: file-format section names; they match the corrections block in
// the prompt file byte for byte and change only when that format changes.
const SECTION_PREFIX: &str = "### ";
const SECTION_KEY_IN_PROSE: &str = "key_in_prose";
const SECTION_UNKNOWN_KEY: &str = "unknown_key";
const SECTION_UNPARSEABLE: &str = "unparseable";
const SECTION_NOTHING_SAID: &str = "nothing_said";
const SECTION_RESEND: &str = "resend";

/// Every section a file with the marker must carry.
// STRUCTURAL: the complete section list of the corrections format, checked
// against the shipped v5 file by `the_v5_prompt_file_carries_every_correction`.
pub const CORRECTION_SECTIONS: &[&str] = &[
    SECTION_KEY_IN_PROSE,
    SECTION_UNKNOWN_KEY,
    SECTION_UNPARSEABLE,
    SECTION_NOTHING_SAID,
    SECTION_RESEND,
];

/// The one-sentence corrections, and the frame that carries one back.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Corrections {
    /// `{part}` and `{token}`: the part the key was found in, and the key.
    key_in_prose: String,
    /// `{key}`: the key the model cited without having been sent it.
    unknown_key: String,
    unparseable: String,
    nothing_said: String,
    /// `{reply}` and `{correction}`: appended to the ORIGINAL user message.
    resend: String,
}

/// The prompt file, understood.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReadPrompt {
    /// What is sent as the system prompt — the file up to the marker.
    pub system: String,
    /// `None` when the file has no marker (v4 and earlier).
    pub corrections: Option<Corrections>,
}

/// Cut the prompt file at the marker and read each correction.
///
/// # Errors
/// An operator's sentence naming the section when the file carries the marker
/// but a section is missing or blank — a half-deployed v5 must not silently
/// degrade to plain re-requests.
pub fn split_prompt(file: &str) -> Result<ReadPrompt, String> {
    let Some((system, tail)) = split_at_marker(file) else {
        return Ok(ReadPrompt {
            system: file.to_string(),
            corrections: None,
        });
    };
    let section = |name: &str| {
        section_body(tail, name).ok_or_else(|| {
            format!(
                "the read prompt carries {CORRECTIONS_MARKER} but no non-empty \
                 `{SECTION_PREFIX}{name}` section below it — restore the section, \
                 or point practice_read_prompt_file at a complete file"
            )
        })
    };
    Ok(ReadPrompt {
        system: system.trim_end().to_string(),
        corrections: Some(Corrections {
            key_in_prose: section(SECTION_KEY_IN_PROSE)?,
            unknown_key: section(SECTION_UNKNOWN_KEY)?,
            unparseable: section(SECTION_UNPARSEABLE)?,
            nothing_said: section(SECTION_NOTHING_SAID)?,
            resend: section(SECTION_RESEND)?,
        }),
    })
}

/// Split at the first line that IS the marker (trimmed), not one that merely
/// mentions it — a sentence quoting the marker must not cut the prompt.
fn split_at_marker(file: &str) -> Option<(&str, &str)> {
    let mut offset = 0;
    for line in file.split_inclusive('\n') {
        if line.trim() == CORRECTIONS_MARKER {
            return Some((&file[..offset], &file[offset + line.len()..]));
        }
        offset += line.len();
    }
    None
}

/// The text under `### name`, up to the next `### ` heading, trimmed. `None`
/// when the heading is absent or its body is blank.
fn section_body(tail: &str, name: &str) -> Option<String> {
    let heading = format!("{SECTION_PREFIX}{name}");
    let mut lines = tail.lines().skip_while(|line| line.trim() != heading);
    lines.next()?;
    let body: Vec<&str> = lines
        .take_while(|line| !line.starts_with(SECTION_PREFIX))
        .collect();
    let body = body.join("\n").trim().to_string();
    (!body.is_empty()).then_some(body)
}

impl Corrections {
    /// The one sentence naming what was wrong with a rejected reply.
    ///
    /// `None` for [`ReplyRejection::Empty`]: there is no reply to send back and
    /// nothing to correct, so the re-request is the plain one. The four the GO
    /// names — a key in the prose, an unknown key, an unparseable reply, a reply
    /// that said nothing — each have their own sentence.
    ///
    /// ## Rust Learning: `str::replace` over a templating crate
    ///
    /// Four fixed tokens, filled once each. A template engine would add a crate
    /// and an escaping model for a job `replace` does exactly; the tokens are
    /// braced (`{part}`) because they are written in the prompt file, where a
    /// brace reads as "filled in" to the person editing it.
    pub fn correction_for(&self, rejection: &ReplyRejection) -> Option<String> {
        match rejection {
            ReplyRejection::Empty => None,
            ReplyRejection::Unparseable { .. } => Some(self.unparseable.clone()),
            ReplyRejection::NothingSaid => Some(self.nothing_said.clone()),
            ReplyRejection::UnknownKey { key, .. } => Some(self.unknown_key.replace("{key}", key)),
            ReplyRejection::KeyInProse { part, token } => Some(
                self.key_in_prose
                    .replace("{part}", part)
                    .replace("{token}", token),
            ),
        }
    }

    /// Attempt 2's user message: the original, then the rejected reply and the
    /// correction inside the `resend` frame.
    ///
    /// Domain note: the ORIGINAL message is kept whole rather than replaced —
    /// the model is being asked to fix a reply about THIS answer, and without
    /// the payload it would be correcting a verdict on nothing.
    ///
    /// `{correction}` is filled BEFORE `{reply}`, so a reply that happens to
    /// contain the text `{correction}` cannot have the sentence spliced into it.
    pub fn resend(&self, original_user: &str, rejected_reply: &str, correction: &str) -> String {
        let frame = self
            .resend
            .replace("{correction}", correction)
            .replace("{reply}", rejected_reply);
        format!("{original_user}\n\n{frame}")
    }
}

#[cfg(test)]
#[path = "practice_read_corrections_tests.rs"]
mod tests;
