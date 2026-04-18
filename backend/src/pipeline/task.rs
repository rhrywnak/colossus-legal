//! backend/src/pipeline/task.rs
//!
//! The `DocProcessing` task enum — the application-defined Task implementation
//! that drives the Phase 4 pipeline FSM through 5 steps:
//! ExtractText → LlmExtract → Ingest → Index → Completeness.
//!
//! This file defines the enum and its [`colossus_pipeline::Task`] impl.
//! The per-step logic lives in individual files under `pipeline/steps/`
//! and lands incrementally via tasks P4-3 through P4-7. This task (P4-2)
//! establishes the dispatch skeleton with `todo!()` placeholders for
//! `execute_current` and `on_cancel_current` so that:
//!   - the enum compiles,
//!   - `on_delete_current` is fully functional (dispatches to cleanup_all
//!     for every variant),
//!   - P4-3..P4-7 each replace one arm of execute/on_cancel without
//!     touching this file's structure.
//!
//! ## Rust Learning: tuple variants wrapping Step structs
//!
//! `DocProcessing::ExtractText(ExtractText)` is a tuple variant that wraps
//! the per-step struct. This lets the Task enum dispatch via the Step trait
//! implementation on each struct: `step.execute(...)`. The pattern scales
//! cleanly — adding a 6th step just means adding a 6th variant and a 6th
//! match arm, with zero change to how dispatch works.

use std::error::Error;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

use colossus_pipeline::cancel::CancellationToken;
use colossus_pipeline::progress::ProgressReporter;
use colossus_pipeline::step::step_name_of;
use colossus_pipeline::{PipelineError, Step, StepResult, Task};

use crate::pipeline::context::AppContext;
use crate::pipeline::steps::{
    cleanup::cleanup_all, completeness::Completeness, extract_text::ExtractText, index::Index,
    ingest::Ingest, llm_extract::LlmExtract,
};

/// The colossus-legal document-processing pipeline task.
///
/// Each variant wraps a concrete step struct. The variant order mirrors
/// the normal execution order; the FSM is enforced at runtime via
/// `validate_transition`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum DocProcessing {
    ExtractText(ExtractText),
    LlmExtract(LlmExtract),
    Ingest(Ingest),
    Index(Index),
    Completeness(Completeness),
}

/// Permitted (from, to) transitions for the DocProcessing FSM.
///
/// ExtractText → LlmExtract → Ingest → Index → Completeness.
/// No branches, no backward edges, no self-loops.
const VALID_TRANSITIONS: &[(&str, &str)] = &[
    ("ExtractText", "LlmExtract"),
    ("LlmExtract", "Ingest"),
    ("Ingest", "Index"),
    ("Index", "Completeness"),
];

#[async_trait]
impl Task for DocProcessing {
    type Context = AppContext;

    fn current_step_name(&self) -> &'static str {
        match self {
            DocProcessing::ExtractText(_) => step_name_of::<ExtractText>(),
            DocProcessing::LlmExtract(_) => step_name_of::<LlmExtract>(),
            DocProcessing::Ingest(_) => step_name_of::<Ingest>(),
            DocProcessing::Index(_) => step_name_of::<Index>(),
            DocProcessing::Completeness(_) => step_name_of::<Completeness>(),
        }
    }

    fn validate_transition(current: &str, next: &str) -> Result<(), PipelineError> {
        if VALID_TRANSITIONS
            .iter()
            .any(|(c, n)| *c == current && *n == next)
        {
            Ok(())
        } else {
            Err(PipelineError::InvalidTransition {
                from: current.to_string(),
                to: next.to_string(),
            })
        }
    }

    async fn execute_current(
        self,
        db: &PgPool,
        context: &Self::Context,
        cancel: &CancellationToken,
        progress: &ProgressReporter,
    ) -> Result<StepResult<Self>, Box<dyn Error + Send + Sync>> {
        // Each arm is replaced by its owning task:
        //   ExtractText  → P4-3
        //   LlmExtract   → P4-4
        //   Ingest       → P4-5  (landed)
        //   Index        → P4-6
        //   Completeness → P4-7
        match self {
            DocProcessing::ExtractText(_) => todo!("P4-3: ExtractText::execute"),
            DocProcessing::LlmExtract(_) => todo!("P4-4: LlmExtract::execute"),
            DocProcessing::Ingest(step) => step.execute(db, context, cancel, progress).await,
            DocProcessing::Index(_) => todo!("P4-6: Index::execute"),
            DocProcessing::Completeness(_) => todo!("P4-7: Completeness::execute"),
        }
    }

    async fn on_cancel_current(
        self,
        db: &PgPool,
        context: &Self::Context,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        // Each arm's per-step cancel rollback is filled in by P4-3..P4-7.
        // Until then, cancellation dispatch panics with a clear message
        // naming the owning task.
        match self {
            DocProcessing::ExtractText(_) => todo!("P4-3: ExtractText::on_cancel"),
            DocProcessing::LlmExtract(_) => todo!("P4-4: LlmExtract::on_cancel"),
            DocProcessing::Ingest(step) => step.on_cancel(db, context).await,
            DocProcessing::Index(_) => todo!("P4-6: Index::on_cancel"),
            DocProcessing::Completeness(_) => todo!("P4-7: Completeness::on_cancel"),
        }
    }

    async fn on_delete_current(
        self,
        db: &PgPool,
        context: &Self::Context,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        // Delete wipes everything the pipeline produced, regardless of
        // which step the job was parked at. cleanup_all's saga semantics
        // attempt all three subsystems and report composite failures.
        let document_id = match &self {
            DocProcessing::ExtractText(s) => s.document_id.clone(),
            DocProcessing::LlmExtract(s) => s.document_id.clone(),
            DocProcessing::Ingest(s) => s.document_id.clone(),
            DocProcessing::Index(s) => s.document_id.clone(),
            DocProcessing::Completeness(s) => s.document_id.clone(),
        };
        cleanup_all(&document_id, db, context)
            .await
            .map(|_report| ())
            .map_err(|e| Box::new(e) as Box<dyn Error + Send + Sync>)
    }
}

// ─────────────────────────────────────────────────────────────────────────
// Unit tests
// ─────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    const DOC_ID: &str = "test-doc";
    const STEP_NAMES: [&str; 5] = [
        "ExtractText",
        "LlmExtract",
        "Ingest",
        "Index",
        "Completeness",
    ];

    fn make_extract_text() -> DocProcessing {
        DocProcessing::ExtractText(ExtractText {
            document_id: DOC_ID.to_string(),
        })
    }
    fn make_llm_extract() -> DocProcessing {
        DocProcessing::LlmExtract(LlmExtract {
            document_id: DOC_ID.to_string(),
        })
    }
    fn make_ingest() -> DocProcessing {
        DocProcessing::Ingest(Ingest {
            document_id: DOC_ID.to_string(),
        })
    }
    fn make_index() -> DocProcessing {
        DocProcessing::Index(Index {
            document_id: DOC_ID.to_string(),
        })
    }
    fn make_completeness() -> DocProcessing {
        DocProcessing::Completeness(Completeness {
            document_id: DOC_ID.to_string(),
        })
    }

    #[test]
    fn current_step_name_returns_last_component_for_each_variant() {
        let cases: [(DocProcessing, &'static str); 5] = [
            (make_extract_text(), "ExtractText"),
            (make_llm_extract(), "LlmExtract"),
            (make_ingest(), "Ingest"),
            (make_index(), "Index"),
            (make_completeness(), "Completeness"),
        ];
        for (variant, expected) in cases {
            let got = variant.current_step_name();
            assert_eq!(got, expected, "variant should return bare short name");
            assert!(
                !got.contains("::"),
                "step_name_of must strip module path, got: {got}"
            );
        }
    }

    #[test]
    fn validate_transition_accepts_all_forward_edges() {
        for (from, to) in VALID_TRANSITIONS {
            let result = DocProcessing::validate_transition(from, to);
            assert!(
                result.is_ok(),
                "forward edge {from} -> {to} should be valid, got {result:?}"
            );
        }
    }

    #[test]
    fn validate_transition_rejects_self_loop() {
        for name in STEP_NAMES {
            let result = DocProcessing::validate_transition(name, name);
            assert!(
                matches!(result, Err(PipelineError::InvalidTransition { .. })),
                "self-loop {name} -> {name} should be rejected, got {result:?}"
            );
        }
    }

    #[test]
    fn validate_transition_rejects_backward_edges() {
        let backward: [(&str, &str); 3] = [
            ("LlmExtract", "ExtractText"),
            ("Completeness", "Index"),
            ("Ingest", "LlmExtract"),
        ];
        for (from, to) in backward {
            let result = DocProcessing::validate_transition(from, to);
            assert!(
                matches!(result, Err(PipelineError::InvalidTransition { .. })),
                "backward edge {from} -> {to} should be rejected, got {result:?}"
            );
        }
    }

    #[test]
    fn validate_transition_rejects_step_skip() {
        let result = DocProcessing::validate_transition("ExtractText", "Ingest");
        assert!(
            matches!(result, Err(PipelineError::InvalidTransition { .. })),
            "step-skip ExtractText -> Ingest should be rejected, got {result:?}"
        );
    }

    /// Compile-time exhaustiveness check. If a 6th variant is ever added to
    /// `DocProcessing`, this match (and every other match in task.rs) will
    /// fail to compile, forcing the author to update dispatch everywhere.
    /// The function is never called — its presence alone is the assertion.
    #[allow(dead_code)]
    fn exhaustive_match_check(t: DocProcessing) -> &'static str {
        match t {
            DocProcessing::ExtractText(_) => "et",
            DocProcessing::LlmExtract(_) => "le",
            DocProcessing::Ingest(_) => "in",
            DocProcessing::Index(_) => "ix",
            DocProcessing::Completeness(_) => "cp",
        }
    }

    #[test]
    fn current_step_name_match_is_exhaustive() {
        // The real assertion happens at compile time via
        // `exhaustive_match_check` above. This runtime test just touches the
        // helper so dead-code lints don't silently strip the check from
        // release builds.
        let _ = exhaustive_match_check(make_extract_text());
    }
}
