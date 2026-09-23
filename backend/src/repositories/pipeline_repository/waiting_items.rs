//! What is waiting for one person, across any set of decks (CC_TASK_FOR_YOU_v1 L0).
//!
//! ONE predicate, three shapes over it: the list a page prints, the count per
//! scenario a card prints, and the single total a menu badge prints. They read
//! the same body, so the badge and the list can never disagree about what is
//! waiting — the argument `review_cursor::ANSWER_WAITING` already makes about a
//! count and its date, asked of four callers instead of two.
//!
//! ## Domain note: the two sides are read from two different places
//!
//! WHOSE list a viewer is served — the reviewers' or the witness's — is decided
//! in Rust by `services::review_permission::may_review`, so that an unlisted
//! administrator is still served the reviewers' list. WHOSE WORK does not wait
//! is decided here in SQL by membership of the stored
//! `practice_reviewer_usernames` list, because SQL cannot see Authentik groups.
//! That split is ruling R2 of CC_TASK_REVIEW_PERMISSION_v1, carried forward
//! unchanged; this module receives both as parameters and invents neither.
//!
//! ## CRITICAL — the pipeline pool
//!
//! Every table read here lives in `colossus_legal_v2`: `&state.pipeline_pool`.

use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use super::PipelineRepoError;

/// The deck changes that are worth somebody's attention.
// STRUCTURAL: this IS which acts on a deck count as work waiting. Adding,
// moving, hiding and unhiding are bookkeeping about the deck's SHAPE, and the
// question still reads as it did. Not a threshold and not a deployment value —
// changing it changes what "waiting" MEANS, which is a ruling plus a code change.
const WAITING_CHANGE_KINDS: [&str; 2] = ["reworded", "edited"];

/// ...and the one FIELD whose change the witness is waiting on.
///
/// ## Domain note: measured, not assumed (2026-09-22, on a copy of DEV)
///
/// The kinds above are not enough on their own. DEV's 146 deck changes are
/// `reworded`/`text` (25), `edited`/`stronger` (26) and `edited`/`tactic` (25),
/// plus `added`, `moved` and `hidden`. `stronger` is the model answer Chuck
/// keeps beside a question and `tactic` is his note on how it will be asked —
/// both are the REVIEWER's craft, neither is the question the witness reads.
///
/// Without this clause the first render of this page put fifty-one rows in
/// front of her saying "the question changed", quoting values like `4` — the
/// new `tactic` code — under a sentence claiming her question had been
/// reworded. The rule is one sentence: a deck change waits when it changed the
/// question's TEXT.
// STRUCTURAL: the column value the editor writes when it rewrites a question,
// not a tunable. See `practice_editor`.
const WAITING_CHANGE_FIELD: &str = "text";

/// Which list a viewer is being served.
///
/// ## Rust Learning: an enum in place of a boolean parameter
///
/// `waiting_items(pool, ids, user, reviewers, true)` reads as nothing at the
/// call site, and the day a third side appears the boolean has to be replaced
/// everywhere at once. The enum names both sides, makes the call site say which
/// one it means, and gives [`WaitingSide::audience`] one place to keep the SQL
/// fragment that defines it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WaitingSide {
    /// Chuck and whoever else reviews: answers, and notes from anyone who is not
    /// a listed reviewer.
    Reviewers,
    /// Marie: notes from a listed reviewer, and changes to her deck.
    Witness,
}

impl WaitingSide {
    /// The SQL predicate deciding whether an item belongs to this side's list.
    ///
    /// ## Domain note: the audience table, in one place
    ///
    /// | item                            | waits for     |
    /// |---------------------------------|---------------|
    /// | an answer                       | the reviewers |
    /// | a note by a listed reviewer     | the witness   |
    /// | a note by anybody else          | the reviewers |
    /// | a reworded or edited question   | the witness   |
    ///
    /// An unlisted administrator's note therefore waits for the listed
    /// reviewers, which is ruling R2's consequence stated plainly: SQL cannot
    /// see his group, so it reads him as "not a reviewer".
    ///
    /// The NULL author case is named in the note leg rather than left to
    /// `NOT (author = ANY($3))`, because `NULL = ANY(…)` is NULL and a NULL in a
    /// WHERE drops the row. A note with no author id (written before 2026-08-19)
    /// must read as "not by a reviewer" — and the L0 migration has already
    /// marked every one of those seen for everybody, so it waits for nobody in
    /// practice. This clause is what keeps that true rather than accidental.
    fn audience(self) -> &'static str {
        match self {
            WaitingSide::Reviewers => {
                "i.kind = 'answer' \
                 OR (i.kind = 'note' AND (i.author IS NULL OR NOT (i.author = ANY($3))))"
            }
            WaitingSide::Witness => {
                "(i.kind = 'note' AND i.author IS NOT NULL AND i.author = ANY($3)) \
                 OR i.kind = 'change'"
            }
        }
    }
}

/// Which of the two tabs is being asked for.
///
/// ## Domain note: ONE predicate, two readings (the page's two tabs)
///
/// "Unread" is what waits; "Everything" is the same list with the read rows
/// still in it, so a person can go back to a note they have already opened.
/// They are one query with one clause different, rather than two queries that
/// could disagree about what belongs on the page at all.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WaitingScope {
    /// Only items this viewer has not seen.
    Unseen,
    /// Every item on this side, seen or not.
    Everything,
}

impl WaitingScope {
    /// The clause that narrows the list to the unread half.
    ///
    /// The seen record is LEFT JOINed either way, so a row always carries the
    /// moment this person first read it (or `NULL`). This clause is the only
    /// difference between the two tabs — an anti-join for one, nothing for the
    /// other.
    fn clause(self) -> &'static str {
        match self {
            WaitingScope::Unseen => "AND v.seen_at IS NULL ",
            WaitingScope::Everything => "",
        }
    }
}

/// Who is asking, about which decks, and on whose side.
///
/// ## Rust Learning: a borrowed parameter struct
///
/// Four parameters that always travel together, three of them `&[…]` or `&str`.
/// Bundling them borrows rather than copies — the `'a` says the struct may not
/// outlive the slices it points at — and it stops the call sites from putting
/// `viewer` and a reviewer login in the wrong order, which two bare `&str`
/// parameters invite and the compiler cannot catch.
#[derive(Debug, Clone, Copy)]
pub struct WaitingQuery<'a> {
    /// The decks to ask about. One row comes back per id in [`waiting_counts`],
    /// including the ones holding nothing.
    pub scenario_ids: &'a [Uuid],
    /// The signed-in person's stable login. Their own work never waits for them.
    pub viewer: &'a str,
    /// `practice_reviewer_usernames`, as stored. Never a display name.
    pub reviewers: &'a [String],
    /// Which list this viewer is served — decided by `may_review`, not here.
    pub side: WaitingSide,
    /// Unread only, or everything on this side.
    pub scope: WaitingScope,
}

/// One item waiting, with everything a row on the page needs to name it.
#[derive(Debug, Clone, PartialEq, sqlx::FromRow)]
pub struct WaitingItemRow {
    /// `answer`, `note` or `change`.
    pub kind: String,
    /// The id of the answer, note or change itself.
    pub item_id: Uuid,
    pub scenario_id: Uuid,
    /// The scenario's number, for the `S-7` a screen prints. `None` for a
    /// scenario minted before codes existed.
    pub code_ordinal: Option<i32>,
    pub scenario_name: String,
    /// `None` only for a note written about a whole scenario.
    pub question_id: Option<Uuid>,
    pub question_text: Option<String>,
    /// When the item was written — answered, noted or changed.
    pub at: DateTime<Utc>,
    /// Who wrote it, or `None` for a row from before attribution existed.
    pub author: Option<String>,
    /// When this viewer first read it, or `None` while it is still unread.
    /// Always `None` under [`WaitingScope::Unseen`], by construction.
    pub seen_at: Option<DateTime<Utc>>,
    /// What the item SAYS: the answer's text, the note's text, or the
    /// question's new wording. `None` only for a deck change that recorded no
    /// new value, which the composer renders as a change with no quotation
    /// rather than as an empty one.
    pub body: Option<String>,
    /// The note this one REPLIES to, as its text (CC_TASK_FOR_YOU_v1 L3).
    /// `None` for every other kind, and for a note that answers nothing.
    ///
    /// ## Domain note: the parent's WORDS, not its id
    ///
    /// A row has one line for a body, and "Yes, that's right" on its own says
    /// nothing. The composer needs what was asked to render what was answered,
    /// and the alternative — carrying an id and reading the parent back per row
    /// — would be one query per reply on a page built to cost two.
    pub reply_to: Option<String>,
    /// For a note left on one ATTEMPT, when that attempt was answered — the
    /// date the witness's own byline names ("on your answer of Mon 21 Sep").
    /// `None` for every other kind, and for a note on the question itself.
    pub subject_at: Option<DateTime<Utc>>,
}

/// How much one deck holds for this viewer, and since when.
#[derive(Debug, Clone, PartialEq, sqlx::FromRow)]
pub struct WaitingCountRow {
    pub scenario_id: Uuid,
    /// Items waiting. Zero is a row, not a missing row.
    pub waiting: i64,
    /// When the OLDEST of them was written; `None` when nothing waits.
    pub oldest: Option<DateTime<Utc>>,
}

/// The shared body: every item that waits for this viewer, as the CTE `waiting`.
///
/// ## Rust Learning: `format!` assembling SQL from CONSTANTS only
///
/// Everything interpolated here is a compile-time `&'static str` chosen by an
/// enum — never a value from a request. The logins, the ids and the change kinds
/// all arrive as binds ($1–$4), so no caller-supplied text ever reaches the
/// statement.
///
/// ## The three legs, and what each one leaves out
///
/// - `cur` is the CURRENT answer per visible question, the same `DISTINCT ON`
///   shape three existing queries share. A superseded answer is not an item:
///   re-answering replaces what is waiting rather than adding to it.
/// - notes must still STAND (`struck_at IS NULL`) — striking is how a note is
///   withdrawn — and a note pinned to an answer only counts while that answer is
///   the current one.
/// - changes count only in the kinds [`WAITING_CHANGE_KINDS`] names.
/// - hidden questions never appear on any leg.
fn waiting_cte(side: WaitingSide, scope: WaitingScope) -> String {
    let audience = side.audience();
    let unread_only = scope.clause();
    format!(
        "WITH cur AS ( \
            SELECT DISTINCT ON (a.question_id) a.question_id, a.id AS answer_id, \
                   a.answered_at, a.answer_text, s.user_id AS author_id, q.scenario_id \
            FROM practice_answers a \
            JOIN practice_sessions s ON s.id = a.session_id \
            JOIN practice_questions q ON q.id = a.question_id \
            WHERE q.scenario_id = ANY($1) AND q.hidden_at IS NULL \
            ORDER BY a.question_id, a.answered_at DESC, a.id DESC), \
         seen AS ( \
            SELECT 'answer'::text AS kind, answer_id AS item_id, seen_at FROM practice_item_seen \
             WHERE user_id = $2 AND answer_id IS NOT NULL \
            UNION ALL SELECT 'note', note_id, seen_at FROM practice_item_seen \
             WHERE user_id = $2 AND note_id IS NOT NULL \
            UNION ALL SELECT 'change', change_id, seen_at FROM practice_item_seen \
             WHERE user_id = $2 AND change_id IS NOT NULL), \
         items AS ( \
            SELECT 'answer'::text AS kind, cur.answer_id AS item_id, cur.scenario_id, \
                   cur.question_id, cur.answered_at AS at, cur.author_id AS author, \
                   cur.answer_text AS body, NULL::timestamptz AS subject_at, \
                   NULL::text AS reply_to \
              FROM cur \
            UNION ALL \
            SELECT 'note', n.id, n.scenario_id, n.question_id, n.created_at, n.author_id, \
                   n.text, CASE WHEN n.answer_id IS NOT NULL THEN cur.answered_at END, \
                   parent.text \
              FROM practice_notes n \
              LEFT JOIN practice_questions q ON q.id = n.question_id \
              LEFT JOIN cur ON cur.question_id = n.question_id \
              LEFT JOIN practice_notes parent ON parent.id = n.answers_note_id \
             WHERE n.scenario_id = ANY($1) AND n.struck_at IS NULL \
               AND (n.question_id IS NULL OR q.hidden_at IS NULL) \
               AND (n.answer_id IS NULL OR n.answer_id = cur.answer_id) \
            UNION ALL \
            SELECT 'change', d.id, d.scenario_id, d.question_id, d.changed_at, d.changed_by_id, \
                   d.after_value, NULL::timestamptz, NULL::text \
              FROM practice_deck_changes d \
              JOIN practice_questions q ON q.id = d.question_id \
             WHERE d.scenario_id = ANY($1) AND q.hidden_at IS NULL \
               AND d.change_kind = ANY($4) AND d.field = '{WAITING_CHANGE_FIELD}'), \
         waiting AS ( \
            SELECT i.*, v.seen_at FROM items i \
             LEFT JOIN seen v ON v.kind = i.kind AND v.item_id = i.item_id \
             WHERE ({audience}) \
               AND i.author IS DISTINCT FROM $2 \
               {unread_only}) "
    )
}

/// The change kinds as a bindable array.
fn change_kinds() -> Vec<String> {
    WAITING_CHANGE_KINDS
        .iter()
        .map(|k| (*k).to_string())
        .collect()
}

/// The items waiting for this viewer, newest first, at most `limit` of them.
///
/// ## Domain note: an item's own author is never waiting for it
///
/// `i.author IS DISTINCT FROM $2` is applied to every kind, including a change
/// (ruled 2026-09-22, Q3). Until this shipped, the war room's amber count
/// included a question the witness had reworded herself, which renders on a
/// per-person list as "Marie changed this — waiting for Marie". `IS DISTINCT
/// FROM` rather than `<>` because a row from before attribution carries no
/// author, and `NULL <> 'docmarie'` is NULL — which would drop exactly the rows
/// that most need to be counted.
///
/// ## Rust Learning: `Option<i64>` as a LIMIT
///
/// `LIMIT NULL` is Postgres's spelling of `LIMIT ALL`, so `None` binds to a
/// statement with no cap at all and `Some(n)` caps it — one statement, no
/// branch, and no invented number standing in for "everything". This page is
/// one person's inbox across the whole case; silently truncating it would hide
/// work, which is the one thing it exists not to do.
///
/// # Errors
/// [`PipelineRepoError::Database`] for a failed statement.
pub async fn waiting_items(
    pool: &PgPool,
    query: &WaitingQuery<'_>,
    limit: Option<i64>,
) -> Result<Vec<WaitingItemRow>, PipelineRepoError> {
    // The tie-break on `item_id` makes the order total: two items written in the
    // same millisecond would otherwise come back in whatever order the plan
    // happened to produce, and a page that reorders itself between two loads
    // looks broken even when the set is identical.
    sqlx::query_as::<_, WaitingItemRow>(&format!(
        "{}SELECT w.kind, w.item_id, w.scenario_id, sc.code_ordinal, \
                 sc.name AS scenario_name, w.question_id, q.text AS question_text, \
                 w.at, w.author, w.seen_at, w.body, w.subject_at, w.reply_to \
            FROM waiting w \
            JOIN scenarios sc ON sc.scenario_id = w.scenario_id \
            LEFT JOIN practice_questions q ON q.id = w.question_id \
           ORDER BY w.at DESC, w.item_id \
           LIMIT $5",
        waiting_cte(query.side, query.scope)
    ))
    // The bind map, spelled out because the CTE above is composed by
    // `waiting_cte` and this call site's own literal therefore shows only some
    // of the statement's placeholders: $1 the scenario ids, $2 the viewer,
    // $3 the reviewer list, $4 the change kinds, $5 the row limit.
    // It is also what `sql_invariants::every_sqlx_statement_binds_one_value_per_placeholder`
    // reads to check this chain's arity — without it the guard sees a statement
    // shorter than the one Postgres will run.
    .bind(query.scenario_ids)
    .bind(query.viewer)
    .bind(query.reviewers)
    .bind(change_kinds())
    .bind(limit)
    .fetch_all(pool)
    .await
    .map_err(PipelineRepoError::from)
}

/// How much waits for this viewer on each deck asked about, and since when.
///
/// Starts `FROM unnest($1)` for the reason `war_room_status` gives: one row per
/// scenario ASKED FOR, so a deck holding nothing comes back as a zero instead of
/// vanishing from the result and leaving the caller to guess whether it was
/// empty or never queried.
///
/// # Errors
/// [`PipelineRepoError::Database`] for a failed statement.
pub async fn waiting_counts(
    pool: &PgPool,
    query: &WaitingQuery<'_>,
) -> Result<Vec<WaitingCountRow>, PipelineRepoError> {
    sqlx::query_as::<_, WaitingCountRow>(&format!(
        "{}SELECT ids.scenario_id, count(w.item_id) AS waiting, MIN(w.at) AS oldest \
            FROM unnest($1::uuid[]) AS ids(scenario_id) \
            LEFT JOIN waiting w ON w.scenario_id = ids.scenario_id \
           GROUP BY ids.scenario_id",
        waiting_cte(query.side, query.scope)
    ))
    // The bind map, spelled out because the CTE above is composed by
    // `waiting_cte` and this call site's own literal therefore shows only some
    // of the statement's placeholders: $1 the scenario ids, $2 the viewer,
    // $3 the reviewer list, $4 the change kinds.
    // It is also what `sql_invariants::every_sqlx_statement_binds_one_value_per_placeholder`
    // reads to check this chain's arity — without it the guard sees a statement
    // shorter than the one Postgres will run.
    .bind(query.scenario_ids)
    .bind(query.viewer)
    .bind(query.reviewers)
    .bind(change_kinds())
    .fetch_all(pool)
    .await
    .map_err(PipelineRepoError::from)
}

/// The ONE number the menu badge prints: everything waiting, across these decks.
///
/// Its own statement rather than a sum of [`waiting_counts`] in Rust, so the
/// badge costs one round trip on every page — and, more to the point, so the
/// badge and the list are two readings of the same predicate rather than two
/// implementations of it.
///
/// # Errors
/// [`PipelineRepoError::Database`] for a failed statement.
pub async fn waiting_total(
    pool: &PgPool,
    query: &WaitingQuery<'_>,
) -> Result<i64, PipelineRepoError> {
    let row: (i64,) = sqlx::query_as(&format!(
        "{}SELECT count(*) FROM waiting",
        waiting_cte(query.side, query.scope)
    ))
    // The bind map, spelled out because the CTE above is composed by
    // `waiting_cte` and this call site's own literal therefore shows only some
    // of the statement's placeholders: $1 the scenario ids, $2 the viewer,
    // $3 the reviewer list, $4 the change kinds.
    // It is also what `sql_invariants::every_sqlx_statement_binds_one_value_per_placeholder`
    // reads to check this chain's arity — without it the guard sees a statement
    // shorter than the one Postgres will run.
    .bind(query.scenario_ids)
    .bind(query.viewer)
    .bind(query.reviewers)
    .bind(change_kinds())
    .fetch_one(pool)
    .await?;
    Ok(row.0)
}

#[cfg(test)]
#[path = "waiting_items_live_tests.rs"]
mod live_tests;

#[cfg(test)]
#[path = "waiting_items_item_live_tests.rs"]
mod item_live_tests;

#[cfg(test)]
#[path = "waiting_items_seen_live_tests.rs"]
mod seen_live_tests;

#[cfg(test)]
#[path = "waiting_items_reply_live_tests.rs"]
mod reply_live_tests;
