//! Restate workflow step handlers.
//!
//! Each step is a thin async function called from within a `ctx.run()`
//! closure in the [`DocumentPipeline`](crate::pipeline::workflow)
//! workflow. Steps take `&Arc<AppContext>` and a document id, perform
//! their work against the database and external services, and return a
//! short summary string that Restate journals for replay.
//!
//! ## Why a sibling module to `steps/`
//!
//! `steps/` holds the legacy `colossus-pipeline`-driven steps that
//! implement `Step<DocProcessing>`. Those carry a `CancellationToken` /
//! `ProgressReporter` and a `Step::execute` shape that does not match
//! what Restate's `ctx.run` closures want. Rather than make every legacy
//! step also satisfy the Restate shape (or vice versa), the Restate
//! step handlers live here as plain `pub async fn` and delegate their
//! shared work to extracted helpers in `steps/` (e.g.
//! [`crate::pipeline::steps::extract_text::extract_text_to_db`]). This
//! keeps both paths thin: the legacy step wraps the helper with cancel
//! checks and progress reporting; the Restate handler wraps the same
//! helper with the journal-summary formatting and Restate's
//! `HandlerError` / `TerminalError` error classification.
//!
//! ## Idempotency
//!
//! Every step in this module checks whether its work has already been
//! done before proceeding. Restate replays the `ctx.run` closure on
//! workflow recovery, so each closure body must be safe to run more
//! than once against the same database state — the idempotency check
//! is what makes that safe. The DB layer is friendly to this pattern
//! (`document_text` upserts on `(document_id, page_number)`), but the
//! step body still short-circuits when prior work is observable, both
//! to save the work itself and to keep the journal entry small.

pub mod extract_text;
