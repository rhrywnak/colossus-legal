//! The proposal file — the middle step, where a human approves.
//!
//! ## Format
//!
//! ```text
//! # REMAP PROPOSAL — doc-sabrina-morris-affidavit
//! # Delete any MAP line you do not want applied, then add an APPROVED line.
//! #
//! # APPROVED <your name>
//!
//! MAP doc:evidence:old1 doc:evidence:new1
//! MAP doc:evidence:old2 doc:evidence:new2
//! ```
//!
//! Two rules do all the work:
//!
//! - **`apply` refuses a proposal with no `APPROVED` line.** The generated file
//!   carries that line COMMENTED OUT, so approving means deleting a `#` — a
//!   deliberate act on a file someone opened. A generated file cannot be executed
//!   by accident, and that is what makes "propose → approve → apply" a real
//!   three-step rather than a two-step with a ceremony in the middle.
//! - **Deleting a `MAP` line un-approves that move.** Rejecting one match needs
//!   no syntax, no flag and no second file.
//!
//! The human queue is written alongside as its own file. It is never executed —
//! it is the list of decisions this tool refused to make.

use std::fmt::Write as _;

use super::plan::{Match, RemapPlan};

/// A parsed proposal, ready to apply.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Proposal {
    pub document_id: String,
    /// Who approved it. `None` means the file was never approved and `apply`
    /// must refuse.
    pub approved_by: Option<String>,
    pub moves: Vec<(String, String)>,
}

/// Why a proposal could not be used.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ProposalError {
    #[error("line {line}: {message}")]
    Syntax { line: usize, message: String },

    #[error(
        "the proposal names no document. A remap has to know which document's \
         rows it is moving"
    )]
    NoDocument,

    #[error(
        "the proposal has no APPROVED line. Open it, delete any MAP line you \
         reject, uncomment the APPROVED line with your name, and run apply again. \
         A generated proposal is never executed unread"
    )]
    NotApproved,

    #[error(
        "the proposal has no MAP lines left. Nothing would be applied; if that \
         is what you meant, there is nothing to run"
    )]
    NoMoves,
}

/// Render the proposal file from a plan.
pub fn render(plan: &RemapPlan) -> String {
    let t = plan.totals();
    let mut out = String::new();
    let _ = writeln!(out, "# REMAP PROPOSAL — {}", plan.document_id);
    let _ = writeln!(
        out,
        "#\n# {} old node(s): {} unchanged · {} unambiguous · {} ambiguous · {} unmatched\n\
         # Yield (unchanged + unambiguous): {:.1}%\n\
         # Curated rows at risk in the queue: {}\n#",
        t.old_nodes,
        t.unchanged,
        t.unambiguous,
        t.ambiguous,
        t.unmatched,
        t.yield_percent(),
        t.curated_rows_at_risk
    );
    out.push_str(
        "# Every MAP line below moves that old node's curated rows onto the new\n\
         # node. Delete any line you do not want applied.\n\
         #\n\
         # Then DELETE THE '#' from the line below and put your name on it.\n\
         # Without it, apply refuses to run.\n\
         #\n\
         # APPROVED your-name-here\n\n",
    );
    _ = writeln!(out, "DOCUMENT {}", plan.document_id);
    for (old, new) in plan.auto_moves() {
        let _ = writeln!(out, "MAP {old} {new}");
    }
    out
}

/// Render the human queue — the decisions this tool refused to make.
pub fn render_queue(plan: &RemapPlan) -> String {
    let queue = plan.queue();
    let mut out = String::new();
    let _ = writeln!(out, "=== REMAP HUMAN QUEUE — {} ===\n", plan.document_id);
    if queue.is_empty() {
        out.push_str(
            "Every old node either kept its id or matched exactly one new node.\n\
             Nothing here needs a human.\n",
        );
        return out;
    }
    let _ = writeln!(
        out,
        "{} node(s) could not be matched safely, worst first — the number after \
         each\nid is the curated rows that node carries, which is what is actually \
         at stake.\n",
        queue.len()
    );
    for node in queue {
        let _ = writeln!(
            out,
            "── {}  ({} curated row(s))",
            node.old.id, node.old.curated_rows
        );
        let _ = writeln!(
            out,
            "   page    : {}",
            node.old
                .page
                .map(|p| p.to_string())
                .unwrap_or_else(|| "—".to_string())
        );
        if let Some(question) = &node.old.question {
            let _ = writeln!(out, "   question: {question}");
        }
        let _ = writeln!(out, "   quote   : {}", node.old.verbatim_quote);
        match &node.outcome {
            Match::Ambiguous { candidates } => {
                let _ = writeln!(out, "   AMBIGUOUS — candidates:");
                for candidate in candidates {
                    let _ = writeln!(out, "     {candidate}");
                }
            }
            Match::Unmatched => {
                let _ = writeln!(
                    out,
                    "   UNMATCHED — no new node carries this page + quote + question"
                );
            }
            // `queue()` yields only the two arms above; handled rather than
            // unwrapped so a future outcome cannot panic here.
            other => {
                let _ = writeln!(out, "   {other:?}");
            }
        }
        out.push('\n');
    }
    out
}

/// Parse a proposal file a human has been through.
pub fn parse(text: &str) -> Result<Proposal, ProposalError> {
    let mut document_id = None;
    let mut approved_by = None;
    let mut moves = Vec::new();

    for (index, raw) in text.lines().enumerate() {
        let line = index + 1;
        // Comments are stripped WHOLE-LINE only: a '#' inside an id would be
        // bizarre, but truncating on it mid-line could silently shorten one.
        let content = raw.trim();
        if content.is_empty() || content.starts_with('#') {
            continue;
        }
        let mut parts = content.split_whitespace();
        let keyword = parts.next().unwrap_or_default();
        let rest: Vec<&str> = parts.collect();
        match keyword {
            "DOCUMENT" => document_id = Some(one_value(line, "DOCUMENT", &rest)?.to_string()),
            "APPROVED" => {
                if rest.is_empty() {
                    return Err(ProposalError::Syntax {
                        line,
                        message: "APPROVED needs a name".to_string(),
                    });
                }
                approved_by = Some(rest.join(" "));
            }
            "MAP" => {
                if rest.len() != 2 {
                    return Err(ProposalError::Syntax {
                        line,
                        message: format!(
                            "MAP needs exactly two ids (old then new), got {}",
                            rest.len()
                        ),
                    });
                }
                moves.push((rest[0].to_string(), rest[1].to_string()));
            }
            other => {
                return Err(ProposalError::Syntax {
                    line,
                    message: format!(
                        "unknown directive '{other}' (expected DOCUMENT, MAP or APPROVED)"
                    ),
                })
            }
        }
    }

    let document_id = document_id.ok_or(ProposalError::NoDocument)?;
    if moves.is_empty() {
        return Err(ProposalError::NoMoves);
    }
    check_for_duplicates(&moves)?;
    Ok(Proposal {
        document_id,
        approved_by,
        moves,
    })
}

impl Proposal {
    /// The moves, if and only if a human approved them.
    pub fn approved_moves(&self) -> Result<&[(String, String)], ProposalError> {
        match self.approved_by {
            Some(_) => Ok(&self.moves),
            None => Err(ProposalError::NotApproved),
        }
    }
}

/// No id may appear twice on either side.
///
/// An old id twice would apply two different moves to one set of rows; a new id
/// twice would collapse two statements' rulings onto one node.
fn check_for_duplicates(moves: &[(String, String)]) -> Result<(), ProposalError> {
    for (side, values) in [
        ("old", moves.iter().map(|(o, _)| o).collect::<Vec<_>>()),
        ("new", moves.iter().map(|(_, n)| n).collect::<Vec<_>>()),
    ] {
        let mut seen: Vec<&String> = values.clone();
        let before = seen.len();
        seen.sort();
        seen.dedup();
        if seen.len() != before {
            return Err(ProposalError::Syntax {
                line: 0,
                message: format!("an id appears more than once on the {side} side of a MAP"),
            });
        }
    }
    Ok(())
}

/// Read a directive that takes exactly one value.
fn one_value<'a>(line: usize, keyword: &str, rest: &[&'a str]) -> Result<&'a str, ProposalError> {
    match rest {
        [value] => Ok(value),
        _ => Err(ProposalError::Syntax {
            line,
            message: format!("{keyword} needs exactly one value"),
        }),
    }
}

#[cfg(test)]
#[path = "proposal_tests.rs"]
mod tests;
