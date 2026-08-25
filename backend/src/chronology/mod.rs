//! The case chronology's one-shot seed (CASE_CHRONOLOGY_DESIGN_v2 §6).
//!
//! Sits beside `practice`, `rekey`, `remap`, `partymerge` and `twinmerge` — the
//! one-shot maintenance family — and follows their shape: the DECISIONS are pure
//! and unit-tested (`seed`), the WRITING is a thin transactional layer
//! (`seed_execute`), the PROOF an operator reads is rendered separately
//! (`seed_report`), and the binary in `src/bin/` only parses arguments and
//! translates errors into the family's exit codes.
//!
//! ## What lives here and what does not
//!
//! The seed only. The chronology's READ path is ordinary application code —
//! `repositories::pipeline_repository::chronology*`, `services::chronology_read`,
//! `api::timeline` — and none of it imports this module. When the file has been
//! loaded once and `timeline.json` is deleted (design R15), this whole module
//! becomes history and can be removed without touching a line the app serves.

pub mod seed;
pub mod seed_execute;
pub mod seed_report;

#[cfg(test)]
#[path = "guard_tests.rs"]
mod guard_tests;
