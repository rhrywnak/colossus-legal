//! The exit-code scheme every one-shot maintenance tool answers on.
//!
//! Ruled 2026-08-14 for `rekey_evidence`, and extended by the party-merge
//! addendum to the whole family. These are the runbook's machine-readable answer
//! to "did it do the whole job?", so they are a CONTRACT: a step that branches on
//! `3` must mean the same thing whichever binary produced it.
//!
//! ## The distinction the scheme exists to make
//!
//! `3` and `5` are both non-zero and they are not interchangeable:
//!
//! - `3` — the tool ran to completion and REFUSED to write one unit of work,
//!   because the counts it measured did not match the counts it planned. The
//!   tool is fine; the DATA needs investigating.
//! - `5` — the tool broke part-way through. The TOOL needs investigating.
//!
//! An operator who cannot tell those apart from the exit code alone has to read
//! a log to know whether to page anyone, which is the situation the scheme was
//! written to end.
//!
//! ## What does NOT earn a non-zero code
//!
//! A planned refusal. `rekey_evidence` is expected to refuse 42 twin nodes on
//! every run until the merge session happens, and `merge_evidence_twins` is
//! expected to send the 7 doubly-curated pairs to the human queue on every run.
//! A tool that exited non-zero for doing exactly what it was designed to do
//! would train an operator to ignore its code — which is worse than having no
//! code at all.
//!
//! ## Rust Learning: `u8` constants and `ExitCode::from`
//!
//! `std::process::ExitCode` has no const constructor, so the codes are plain
//! `u8` and each binary does `ExitCode::from(EXIT_OK)`. Keeping them as `u8`
//! also lets a unit test assert their values and their uniqueness, which a test
//! could not do with `ExitCode` (it implements neither `PartialEq` nor `Debug`
//! in a form a test can compare).

/// Ran to completion; every planned write applied; nothing aborted.
///
/// A dry run also exits `0` — it completed the job it was asked to do, which was
/// to plan and prove and write nothing.
pub const EXIT_OK: u8 = 0;

/// Bad arguments or bad input file, or the report could not be written.
pub const EXIT_BAD_INPUT: u8 = 1;

/// Could not connect to Neo4j or to Postgres. The error log names which.
///
/// The two stores share one code deliberately: no runbook step branches on which
/// store refused, and the log already names it.
pub const EXIT_CONNECTION: u8 = 2;

/// Ran to completion, but one or more units of work ABORTED on a count mismatch
/// and were rolled back.
pub const EXIT_UNIT_ABORTED: u8 = 3;

/// The plan is unsafe and nothing was written.
///
/// "Unsafe" means the tool proved, before touching anything, that executing the
/// plan would leave the stores in a state it cannot justify — two nodes sharing
/// an id, a survivor that is also a member, a ruling naming a node that is not
/// there.
pub const EXIT_UNSAFE_PLAN: u8 = 4;

/// Execution failed part-way through; the report names the last good unit.
pub const EXIT_EXECUTION_FAILED: u8 = 5;

/// Every code in the scheme, with its runbook meaning.
///
/// Exists so a test can assert the set is complete and collision-free, and so
/// `--help` text can be generated from the same place the codes are defined
/// rather than hand-copied into four module headers.
pub const ALL_EXIT_CODES: &[(u8, &str)] = &[
    (
        EXIT_OK,
        "completed; nothing aborted (a dry run exits here too)",
    ),
    (
        EXIT_BAD_INPUT,
        "bad arguments or input file, or unwritable report",
    ),
    (EXIT_CONNECTION, "could not connect to Neo4j or Postgres"),
    (
        EXIT_UNIT_ABORTED,
        "completed, but a unit of work was rolled back on a count mismatch",
    ),
    (EXIT_UNSAFE_PLAN, "the plan is unsafe; nothing was written"),
    (EXIT_EXECUTION_FAILED, "execution failed part-way through"),
];

/// Render the scheme for a binary's `--help` epilogue.
///
/// One definition, four `--help` outputs. A code added here appears in every
/// tool's help without anyone remembering to update four doc comments.
pub fn help_text() -> String {
    let mut out = String::from("EXIT CODES:\n");
    for (code, meaning) in ALL_EXIT_CODES {
        out.push_str(&format!("  {code}  {meaning}\n"));
    }
    out
}

#[cfg(test)]
#[path = "exit_tests.rs"]
mod tests;
