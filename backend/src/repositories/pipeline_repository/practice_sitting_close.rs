//! Ending a sitting nobody ever ended.
//!
//! A module of its own rather than another function in
//! [`super::practice_flow`], which stands at 282 non-blank lines against Rule
//! 17's 300 — and because this is a DIFFERENT act from its neighbours there.
//! Those close a sitting because Marie CHOSE something (Resume, Start over, End)
//! and the choice is what says the older ones are over. This one closes a
//! sitting because the DAY ended, which nobody chose and nobody is present for.
//!
//! ## The defect this repairs (recorded 2026-09-16, measured 2026-09-17)
//!
//! The one-page flow reuses the newest unended sitting for a scenario and user
//! (`practice_flow::open_session_for_answers`) and never ends it. The row count
//! was never the problem — reuse caps it at one per user per scenario, and DEV
//! carried exactly one row from the one-page era. What broke is the MEANING of
//! `ended_at` to the one thing that reads it: `practice::last_ended_session`
//! feeds the deck's "what changed since your last sitting" box
//! (`api::practice_deck_read`), and with nothing ending since 2026-08-20 that
//! box measured from a month earlier — 144 deck changes on DEV, growing, and
//! unable to shrink.
//!
//! ## CRITICAL — the pipeline pool
//!
//! `practice_sessions` and `practice_answers` live in `colossus_legal_v2`.

use sqlx::PgPool;
use uuid::Uuid;

use super::PipelineRepoError;

/// Close this user's open sittings on this scenario that did not START today.
///
/// Returns how many were closed, so the caller can log a number rather than an
/// act that may or may not have happened.
///
/// ## Domain note: a sitting is a DAY's work
///
/// "Today" in the CASE's timezone (`practice_case_timezone`), not the server's
/// and not the browser's — the same rule `practice_flow::newest_open_session`
/// already applies when it decides whether a sitting `started_today`. This is a
/// rule rather than a tunable idle threshold on purpose (Standing Rule 2): there
/// is no number here to get wrong, and "the sitting you left yesterday is over"
/// is a sentence Marie would recognise. A sitting she is in RIGHT NOW is never
/// touched, however long it has been open, because she has not finished it.
///
/// ## Domain note: stamped at its own last answer, not at now
///
/// Ruled 2026-09-17. `ended_at` is read as the moment the work stopped, and the
/// 26 sittings open when this shipped stopped in August. Stamping them `NOW()`
/// would record a month of silence as a month of practice. So each is closed at
/// its own last answer, and one that holds no answers at its `started_at` —
/// which is the only honest thing left to say about a sitting where nothing
/// happened.
///
/// # Errors
/// The database's own error when the UPDATE cannot run.
pub async fn close_sittings_before_today(
    pool: &PgPool,
    scenario_id: Uuid,
    user_id: &str,
    timezone: &str,
) -> Result<u64, PipelineRepoError> {
    let done = sqlx::query(
        "UPDATE practice_sessions s \
            SET ended_at = COALESCE( \
                    (SELECT MAX(a.answered_at) FROM practice_answers a \
                      WHERE a.session_id = s.id), \
                    s.started_at) \
          WHERE s.scenario_id = $1 \
            AND s.user_id = $2 \
            AND s.ended_at IS NULL \
            AND (s.started_at AT TIME ZONE $3)::date \
                <> (NOW() AT TIME ZONE $3)::date",
    )
    .bind(scenario_id)
    .bind(user_id)
    .bind(timezone)
    .execute(pool)
    .await?;
    Ok(done.rows_affected())
}

#[cfg(test)]
#[path = "practice_sitting_close_live_tests.rs"]
mod live_tests;
