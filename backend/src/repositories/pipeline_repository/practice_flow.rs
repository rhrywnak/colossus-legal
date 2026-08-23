//! The reads and writes mockup v3 added: the flag on a question, and what one
//! sitting was.
//!
//! A sibling of [`super::practice`] rather than more of it: that module reached
//! Rule 17's 300-line limit when these arrived, and the seam is the honest one.
//! `practice` serves the DRILL — deal the deck, record an answer, close a
//! sitting. This serves what happens AROUND one: the complaint Marie files
//! against a question, and the shape of the evening she started.
//!
//! Same idioms as its sibling, and the same caveat: `sqlx::query` takes a
//! `&str`, so nothing here is checked against the schema at compile time. The
//! disk/code guard that catches a column name this code invents lives with the
//! sibling and reads BOTH files.

use sqlx::PgPool;
use uuid::Uuid;

use super::PipelineRepoError;

/// One flagged question, as the foot of Chuck's sheet prints it.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct FlaggedQuestionRecord {
    pub side: String,
    pub text: String,
    pub sort_order: i32,
    /// Never `None` — the query only returns rows that carry one.
    pub flag_note: Option<String>,
}

/// Every flagged question in this scenario's deck, in deck order.
///
/// ## Domain note: the whole DECK, not this sitting's queue
///
/// The list is headed "Flagged before the session". A question she flagged and
/// then kept out of tonight is exactly the one Roman most needs to see — so
/// filtering to what was asked would hide the complaints that mattered most.
pub async fn list_flagged(
    pool: &PgPool,
    scenario_id: Uuid,
) -> Result<Vec<FlaggedQuestionRecord>, PipelineRepoError> {
    sqlx::query_as::<_, FlaggedQuestionRecord>(
        "SELECT side, text, sort_order, flag_note FROM practice_questions \
         WHERE scenario_id = $1 AND flag_note IS NOT NULL ORDER BY sort_order",
    )
    .bind(scenario_id)
    .fetch_all(pool)
    .await
    .map_err(PipelineRepoError::from)
}

/// How many questions this sitting's stored queue holds.
///
/// `None` when the session carries no queue — every session started before flow
/// v1, and any started by a build that does not write one. The caller must treat
/// that as "unknown", never as zero: a sheet that claimed `Ended early.` because
/// it could not find a queue would be inventing a fact about her evening.
pub async fn session_queue_len(
    pool: &PgPool,
    session_id: Uuid,
) -> Result<Option<i32>, PipelineRepoError> {
    let row: Option<(Option<i32>,)> =
        sqlx::query_as("SELECT jsonb_array_length(queue) FROM practice_sessions WHERE id = $1")
            .bind(session_id)
            .fetch_optional(pool)
            .await?;
    Ok(row.and_then(|r| r.0))
}

/// Store — or clear — Marie's flag on one question.
///
/// ## Domain note: why this writes the QUESTION and not the session
///
/// Roman's ruling of 2026-08-18: a flag outlives the sitting. It is Marie
/// telling Roman and Chuck that a question is wrong, and it stands until one of
/// them changes the deck. A note scoped to an evening would be gone before
/// either of them read it.
///
/// A blank note CLEARS the flag — all three columns together, so a row can never
/// carry a `flagged_at` with nothing flagged. `who` is stored rather than
/// derived at render because the answer to "who flagged this" must survive the
/// log window.
///
/// Returns whether a row was touched, so the route can tell "stored" from "no
/// such question" rather than reporting success for a write that hit nothing.
pub async fn set_flag(
    pool: &PgPool,
    question_id: Uuid,
    note: Option<&str>,
    who: &str,
) -> Result<bool, PipelineRepoError> {
    let done = sqlx::query(
        "UPDATE practice_questions \
         SET flag_note = $2, \
             flagged_at = CASE WHEN $2::text IS NULL THEN NULL ELSE NOW() END, \
             flagged_by = CASE WHEN $2::text IS NULL THEN NULL ELSE $3 END \
         WHERE id = $1",
    )
    .bind(question_id)
    .bind(note)
    .bind(who)
    .execute(pool)
    .await?;
    Ok(done.rows_affected() == 1)
}

/// The open sitting the start card offers back, and enough of it to describe.
///
/// ## Domain note: why `answered` counts ROWS and not distinct questions
///
/// "1 of 5 answered" is about progress through a queue, and a question she asked
/// to be repeated is dealt twice and answered twice. Counting distinct questions
/// would make the line stall at 4 of 5 on a sitting she is working hardest at.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct OpenSessionRecord {
    pub id: Uuid,
    pub who: String,
    pub started_at: chrono::DateTime<chrono::Utc>,
    /// Was it started TODAY, in the case's own timezone?
    ///
    /// Computed by Postgres rather than in Rust, because Rust here has no tz
    /// database and Postgres has the whole one. See `row_statuses` for the
    /// defect that made this necessary.
    pub started_today: bool,
    pub answered: i64,
    /// The stored queue's length. `None` on a session opened before flow v1 —
    /// the line then says how many she answered and refuses to invent a total.
    pub queue_len: Option<i32>,
}

/// The newest sitting THIS USER has open on this scenario, or `None`.
///
/// ## Why `user_id` and not "any open sitting"
///
/// It offered anybody's. Chuck opening the page to edit the deck would be shown
/// Marie's half-finished sitting and could Resume into it, or press Start over
/// and close it — on her work, under his login. A sitting belongs to whoever
/// opened it, and sittings opened before 2026-08-19 carry no user at all, so
/// they match nobody and are simply never offered back. That is the honest
/// outcome: nobody can say whose they were.
///
/// ## Domain note: newest, and the rest are closed when she chooses
///
/// Before Section B nothing ever set `ended_at` on a sitting she walked away
/// from, so a scenario can carry several open sessions. Showing all of them
/// would be an inbox, not an offer. The newest is the one she was last in;
/// [`close_open_sessions_except`] retires the others the moment she presses
/// Resume or Start over, which is the first point at which she has said what
/// the older ones were.
pub async fn newest_open_session(
    pool: &PgPool,
    scenario_id: Uuid,
    user_id: &str,
    timezone: &str,
) -> Result<Option<OpenSessionRecord>, PipelineRepoError> {
    sqlx::query_as::<_, OpenSessionRecord>(
        "SELECT s.id, s.who, s.started_at, \
                (s.started_at AT TIME ZONE $3)::date = (NOW() AT TIME ZONE $3)::date \
                    AS started_today, \
                COUNT(a.id) AS answered, \
                jsonb_array_length(s.queue) AS queue_len \
         FROM practice_sessions s \
         LEFT JOIN practice_answers a ON a.session_id = s.id \
         WHERE s.scenario_id = $1 AND s.ended_at IS NULL AND s.user_id = $2 \
         GROUP BY s.id, s.who, s.started_at, s.queue \
         ORDER BY s.started_at DESC LIMIT 1",
    )
    .bind(scenario_id)
    .bind(user_id)
    .bind(timezone)
    .fetch_optional(pool)
    .await
    .map_err(PipelineRepoError::from)
}

/// How many open sittings this scenario carries.
///
/// Read so the count can be LOGGED before any of them is closed. Section B's
/// task asks for that number on DEV, and an operator who only ever sees the
/// newest one has no way to discover it afterwards.
pub async fn open_session_count(
    pool: &PgPool,
    scenario_id: Uuid,
) -> Result<i64, PipelineRepoError> {
    let row: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM practice_sessions WHERE scenario_id = $1 AND ended_at IS NULL",
    )
    .bind(scenario_id)
    .fetch_one(pool)
    .await?;
    Ok(row.0)
}

/// Close every open sitting on this scenario except one. Returns how many.
///
/// Nothing is deleted: each closed sitting keeps its own rows and its own
/// Chuck's sheet. This is the "close the rest" half of the multi-session rule —
/// see [`newest_open_session`] for why it happens when she chooses rather than
/// on load.
pub async fn close_open_sessions_except(
    pool: &PgPool,
    scenario_id: Uuid,
    keep: Uuid,
) -> Result<u64, PipelineRepoError> {
    let done = sqlx::query(
        "UPDATE practice_sessions SET ended_at = NOW() \
         WHERE scenario_id = $1 AND ended_at IS NULL AND id <> $2",
    )
    .bind(scenario_id)
    .bind(keep)
    .execute(pool)
    .await?;
    Ok(done.rows_affected())
}

/// One sitting, as the page needs it to re-enter at its own address.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct SittingRecord {
    pub scenario_id: Uuid,
    pub who: String,
    /// The dealt question ids in order, as stored. `None` on a session opened
    /// before flow v1, which cannot be resumed and says so.
    pub queue: Option<serde_json::Value>,
    pub ended_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// The sitting behind `…/session/:sessionId`, or `None` if there is no such row.
pub async fn get_sitting(
    pool: &PgPool,
    session_id: Uuid,
) -> Result<Option<SittingRecord>, PipelineRepoError> {
    sqlx::query_as::<_, SittingRecord>(
        "SELECT scenario_id, who, queue, ended_at FROM practice_sessions WHERE id = $1",
    )
    .bind(session_id)
    .fetch_optional(pool)
    .await
    .map_err(PipelineRepoError::from)
}

/// The questions this sitting has already dealt, in the order she answered them.
///
/// ## Domain note: a `skipped` row counts as dealt
///
/// She was shown the question and said it did not fit. Resuming to it would
/// deal it again, which is the one thing the control she pressed asked not to
/// happen. This is the derived-resume rule chosen in .401, unchanged.
pub async fn answered_question_ids(
    pool: &PgPool,
    session_id: Uuid,
) -> Result<Vec<Uuid>, PipelineRepoError> {
    let rows: Vec<(Uuid,)> = sqlx::query_as(
        "SELECT question_id FROM practice_answers \
         WHERE session_id = $1 ORDER BY answered_at, id",
    )
    .bind(session_id)
    .fetch_all(pool)
    .await?;
    Ok(rows.into_iter().map(|r| r.0).collect())
}

/// The newest attempt at one question, and how many attempts there have been.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct RowStatusRecord {
    pub question_id: Uuid,
    pub mark: String,
    pub answered_at: chrono::DateTime<chrono::Utc>,
    /// Was this attempt made TODAY, in the case's own timezone?
    ///
    /// ## The defect this closes
    ///
    /// It was compared in UTC. Marie practises in the evening in Michigan, so at
    /// 20:00 EDT every answer she had just given flipped from `answered today`
    /// to `last: Wed 19 Aug` — four hours before her day ended. Postgres does
    /// the comparing because it carries the tz database and this backend does
    /// not; the zone is a settings row (`practice_case_timezone`).
    pub answered_today: bool,
    pub attempts: i64,
}

/// The newest attempt at each of this scenario's questions.
///
/// ## Why this is one read over the whole scenario and not one per row
///
/// The start card renders every row's status at once. A read per question would
/// be ten round trips for a screen that is one payload by design — and the deck
/// payload is fetched once on mount precisely so a witness never waits.
///
/// ## Domain note: THIS USER's sittings, and every one of them
///
/// `answered today · repeat` is a report on what SHE did. Counting Chuck's test
/// answers into it would tell Marie she had answered a question she has never
/// seen. Sittings recorded before 2026-08-19 carry no user, so they match
/// nobody — an answer from before today shows no status until it is answered
/// again, which is stated in the report rather than papered over.
///
/// Every sitting of hers counts, not just tonight's
///
/// `attempt 2` means the second time she has ever answered this question, and
/// `last: Tue 18 Aug` names a sitting that is over. Scoping this to the open
/// session would make both of them say something else.
pub async fn row_statuses(
    pool: &PgPool,
    scenario_id: Uuid,
    user_id: &str,
    timezone: &str,
) -> Result<Vec<RowStatusRecord>, PipelineRepoError> {
    // DISTINCT ON keeps the first row of each question under the ORDER BY, which
    // is the newest attempt. The window function is evaluated over the whole
    // partition BEFORE the distinct runs, so `attempts` counts every attempt and
    // not just the surviving one.
    sqlx::query_as::<_, RowStatusRecord>(
        "SELECT DISTINCT ON (a.question_id) \
                a.question_id, a.mark, a.answered_at, \
                (a.answered_at AT TIME ZONE $3)::date = (NOW() AT TIME ZONE $3)::date \
                    AS answered_today, \
                COUNT(*) OVER (PARTITION BY a.question_id) AS attempts \
         FROM practice_answers a JOIN practice_sessions s ON s.id = a.session_id \
         WHERE s.scenario_id = $1 AND s.user_id = $2 \
         ORDER BY a.question_id, a.answered_at DESC, a.id DESC",
    )
    .bind(scenario_id)
    .bind(user_id)
    .bind(timezone)
    .fetch_all(pool)
    .await
    .map_err(PipelineRepoError::from)
}

/// How many practice ANSWERS this scenario holds, across every sitting.
///
/// Read before a scenario is deleted. See the delete handler for why the answer
/// decides whether the delete happens at all.
pub async fn answer_count_for_scenario(
    pool: &PgPool,
    scenario_id: Uuid,
) -> Result<i64, PipelineRepoError> {
    let row: (i64,) = sqlx::query_as(
        "SELECT COUNT(a.id) FROM practice_answers a \
         JOIN practice_sessions s ON s.id = a.session_id \
         WHERE s.scenario_id = $1",
    )
    .bind(scenario_id)
    .fetch_one(pool)
    .await?;
    Ok(row.0)
}

/// The mark on the NEWEST answer this sitting holds for one question, if any.
///
/// `None` = this sitting has never answered it. `Some("repeat")` = she asked for
/// it again, so answering it a second time is the feature rather than a
/// duplicate. Anything else is a settled row — see the caller for the two-tabs
/// case this exists for.
pub async fn last_mark_in_session(
    pool: &PgPool,
    session_id: Uuid,
    question_id: Uuid,
) -> Result<Option<String>, PipelineRepoError> {
    let row: Option<(String,)> = sqlx::query_as(
        "SELECT mark FROM practice_answers \
         WHERE session_id = $1 AND question_id = $2 \
         ORDER BY answered_at DESC, id DESC LIMIT 1",
    )
    .bind(session_id)
    .bind(question_id)
    .fetch_optional(pool)
    .await?;
    Ok(row.map(|r| r.0))
}

/// The answer that stands for one question right now.
///
/// Deliberately thinner than [`RowStatusRecord`]: no `mark`, no `attempts`, no
/// "today" arithmetic. The one-page deck row says `Answered on 22 Aug` or says
/// nothing, and the marks it used to carry (`fine` / `repeat` / `attempt 2`) are
/// retired from the interface by CC_TASK_PRACTICE_ONE_PAGE §3.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct CurrentAnswerRecord {
    pub question_id: Uuid,
    /// Her words, as typed. Read here rather than fetched again per question:
    /// the same row feeds `Print answers` and practice mode's reveal.
    pub answer_text: String,
    pub answered_at: chrono::DateTime<chrono::Utc>,
}

/// The current answer to each of a scenario's questions.
///
/// ## Domain note: EVERY user's answers, and why that is the change
///
/// Its neighbour [`row_statuses`] filters `s.user_id = $2`, and had to: it
/// composed `answered today · repeat` — a report on what SHE did — and counting
/// Chuck's test answers into it would have told Marie she had answered a question
/// she had never seen.
///
/// This one carries no filter, and that is a deliberate consequence of the page
/// becoming ONE PAGE FOR TWO PEOPLE. Chuck opens it to read Marie's answers and
/// to print them; scoped to the requester, `Print answers` would hand him blank
/// paper and every row would claim to be unanswered. The answer belongs to the
/// question, not to whoever is looking at it.
///
/// ## Rust Learning: `DISTINCT ON`, a Postgres extension
///
/// `DISTINCT ON (a.question_id)` keeps the FIRST row of each question under the
/// `ORDER BY` — so the ordering is not cosmetic, it is what selects the row. The
/// leading `ORDER BY` column must match the `DISTINCT ON` expression, and the
/// columns after it (`answered_at DESC, id DESC`) are what "first" then means:
/// newest, and for two answers in the same microsecond, the later id. Without
/// that `id` tiebreak the winner would be whichever the planner happened to
/// return, which is not a thing to leave to a planner.
pub async fn current_answers(
    pool: &PgPool,
    scenario_id: Uuid,
) -> Result<Vec<CurrentAnswerRecord>, PipelineRepoError> {
    sqlx::query_as::<_, CurrentAnswerRecord>(
        "SELECT DISTINCT ON (a.question_id) \
                a.question_id, a.answer_text, a.answered_at \
         FROM practice_answers a JOIN practice_sessions s ON s.id = a.session_id \
         WHERE s.scenario_id = $1 \
         ORDER BY a.question_id, a.answered_at DESC, a.id DESC",
    )
    .bind(scenario_id)
    .fetch_all(pool)
    .await
    .map_err(PipelineRepoError::from)
}

/// The session an answer written from the question page belongs to.
///
/// ## Domain note: the sitting is INVISIBLE PLUMBING, not a feature
///
/// `CC_TASK_PRACTICE_ONE_PAGE` retires the sitting apparatus from the
/// INTERFACE — no Start, no counts, no sides to choose, no resume, no end. It
/// does not retire it from the schema, and could not:
/// `practice_answers.session_id` is `NOT NULL REFERENCES practice_sessions(id)`,
/// so every answer must belong to one. Roman's ruling of 2026-08-23 was to keep
/// the row and hide the concept.
///
/// **So: "no sittings" is true of the interface and false of the database.**
/// That sentence needs to survive outside the conversation it was ruled in, and
/// this is where a reader of the code will be standing when they need it.
///
/// Reuses the newest UNENDED session for this scenario and user if there is one,
/// so a witness answering ten questions over an afternoon writes ten answers
/// into one row rather than opening ten sittings nobody will ever look at.
/// Returns `None` when there is none to reuse — the caller then opens one.
pub async fn open_session_for_answers(
    pool: &PgPool,
    scenario_id: Uuid,
    user_id: &str,
) -> Result<Option<Uuid>, PipelineRepoError> {
    let row: Option<(Uuid,)> = sqlx::query_as(
        "SELECT s.id FROM practice_sessions s \
         WHERE s.scenario_id = $1 AND s.user_id = $2 AND s.ended_at IS NULL \
         ORDER BY s.started_at DESC, s.id DESC LIMIT 1",
    )
    .bind(scenario_id)
    .bind(user_id)
    .fetch_optional(pool)
    .await?;
    Ok(row.map(|r| r.0))
}

/// One answer in a question's history, newest first.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct AnswerVersionRecord {
    pub answer_id: Uuid,
    pub answer_text: String,
    pub answered_at: chrono::DateTime<chrono::Utc>,
}

/// Every version of one question's answer, newest first.
///
/// ## Domain note: scenario-wide, like its siblings
///
/// No user filter, for the reason `current_answers` carries none: the page is
/// one page for two people, and an answer belongs to the question rather than
/// to whoever is looking at it.
///
/// ## Why the WHOLE history and not a count
///
/// The question page shows "▸ 2 earlier versions" as one quiet line she never
/// has to open — but when she does open it, the words must already be there. A
/// count now and a fetch on expand would put a spinner inside a disclosure
/// triangle, which is a loading state for something she opened out of idle
/// curiosity. Answers are short and there are rarely more than three.
pub async fn answer_versions(
    pool: &PgPool,
    question_id: Uuid,
) -> Result<Vec<AnswerVersionRecord>, PipelineRepoError> {
    sqlx::query_as::<_, AnswerVersionRecord>(
        "SELECT a.id AS answer_id, a.answer_text, a.answered_at \
         FROM practice_answers a \
         WHERE a.question_id = $1 \
         ORDER BY a.answered_at DESC, a.id DESC",
    )
    .bind(question_id)
    .fetch_all(pool)
    .await
    .map_err(PipelineRepoError::from)
}

/// The answer that stands for one question, if there is one — id and words.
///
/// Lighter than [`answer_versions`] and used on the write path: the answer
/// handler needs to know whether what she just typed is byte-identical to what
/// already stands, and loading a whole history to compare one string would be a
/// read that grows with every version she writes.
pub async fn current_answer_for(
    pool: &PgPool,
    question_id: Uuid,
) -> Result<Option<(Uuid, String)>, PipelineRepoError> {
    let row: Option<(Uuid, String)> = sqlx::query_as(
        "SELECT a.id, a.answer_text FROM practice_answers a \
         WHERE a.question_id = $1 \
         ORDER BY a.answered_at DESC, a.id DESC LIMIT 1",
    )
    .bind(question_id)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}
