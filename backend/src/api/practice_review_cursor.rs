//! "Done reviewing" — the one act that moves a reader's cursor on a deck.
//!
//! CC_TASK_REVIEW_LOOP_v1 §1. `PUT /cases/:slug/scenarios/:scenario_id/practice/review-cursor`
//! upserts `now()` for the signed-in user. There is no delete route, no per-item
//! mark and no stored count: the War Room pill and the deck's review bar are both
//! DERIVED at read time (`review_cursor::awaiting_review`).
//!
//! ## Why the route now REFUSES an impermissible press (v1 of REVIEW_PERMISSION)
//!
//! It used to accept anybody's: a press by a non-reviewer wrote a row that
//! nothing read, so it changed nothing, and hiding the button was enough. That
//! stopped being true the moment the queue began reading EVERY cursor row
//! (`review_cursor::awaiting_review`, R1) — an ungated route would then let any
//! signed-in user clear a deck's queue for the whole team. Permission is decided
//! in ONE place, `services::review_permission::may_review`, and this route and
//! the button both ask it.
//!
//! ## Why PUT
//!
//! Pressing it twice leaves the same one row — the mark simply moves to the later
//! press. That is idempotent in the sense HTTP means, which is what PUT promises
//! and POST does not.
//!
//! ## Why this route IS case- and scenario-scoped
//!
//! Unlike the answer and note routes, its target is a scenario the caller names
//! in the path, not a server-minted row id; the case fence
//! (`ensure_scenario_in_case`) is what stops a mark landing on another case's deck.

use axum::{
    extract::{Path, State},
    Json,
};
use uuid::Uuid;

use crate::{
    auth::AuthUser,
    dto::practice_review::DeckReviewDto,
    dto::practice_review::{ReviewCursorResponse, ReviewSweepRequest, SweptItem},
    error::AppError,
    repositories::pipeline_repository::item_seen::{mark_scenario_notes_seen, mark_seen, ItemRef},
    repositories::pipeline_repository::review_cursor::{awaiting_review, AwaitingReviewRow},
    services::{
        practice_notes::attribution,
        review_permission::may_review,
        war_room_progress::{review_queue_reviewer, reviewer_display_line},
    },
    state::AppState,
};

use super::practice::repo_error;
use super::scenario_facts::{ensure_scenario_in_case, parse_scenario_id};

/// Record that this reviewer has READ the items the page showed.
///
/// ## Domain note: the press must be permitted, because it is now READ
///
/// The seen rows are keyed by `username` — the id `attribution` stamps on every
/// practice write — and they decide what the war room's pill, the deck's bar
/// and the For You page show THIS person. The 403 stays for the reason it
/// arrived: a client calling an address it was not offered must not be able to
/// write read-state at all.
///
/// ## What changed under it (CC_TASK_FOR_YOU_v1 L2)
///
/// It used to move one timestamp for the whole deck, shared across the bench.
/// It now writes one row per item, for the presser alone. Two consequences,
/// both intended: an item that arrived while the page was being read is NOT
/// swept (it is not in `items`), and another reviewer's queue is untouched —
/// their inbox is their own.
///
/// # Errors
/// 400 for an unparseable scenario id; **403 when the caller may not review**;
/// 404 for a scenario not in this case; 500 (logged) for a failed write.
pub async fn put_review_cursor(
    user: AuthUser,
    State(state): State<AppState>,
    Path((slug, scenario_id)): Path<(String, String)>,
    Json(body): Json<ReviewSweepRequest>,
) -> Result<Json<ReviewCursorResponse>, AppError> {
    let scenario_id = parse_scenario_id(&scenario_id)?;
    ensure_scenario_in_case(&state, scenario_id, &slug).await?;
    let (user_id, _) = attribution(&user);

    let settings = state.settings.current();
    if !may_review(&user_id, user.is_admin(), review_queue_reviewer(&settings)) {
        // Logged at WARN with the caller named: a refused press is either
        // somebody's mistake or a client calling an address it was not offered,
        // and both are worth seeing in the journal.
        tracing::warn!(
            %slug, %scenario_id, user = %user_id, admin = user.is_admin(),
            "practice: a press of Done reviewing was refused — not a reviewer"
        );
        return Err(AppError::Forbidden {
            // WHAT failed, WHY (which of the two doors), and WHAT TO DO. The
            // caller cannot read the server's journal, and the warn! above is
            // the operator's half of the same refusal.
            message: "you are not a reviewer on this case: your sign-in name is not \
                      among the reviewers in Settings, and you are not an \
                      administrator. Ask for your name to be added to the \
                      reviewers, or for an administrator to press it for you."
                .to_string(),
        });
    }

    let items: Vec<ItemRef> = body.items.iter().map(swept).collect();
    let shown = mark_seen(&state.pipeline_pool, &user_id, &items)
        .await
        .map_err(|e| {
            repo_error(
                "mark_seen",
                format!("{slug} scenario {scenario_id} user {user_id}: {e}"),
            )
        })?;
    // The one item no row can name — see `ReviewSweepRequest::served_at`.
    let scenario_notes =
        mark_scenario_notes_seen(&state.pipeline_pool, &user_id, scenario_id, body.served_at)
            .await
            .map_err(|e| {
                repo_error(
                    "mark_scenario_notes_seen",
                    format!("{slug} scenario {scenario_id} user {user_id}: {e}"),
                )
            })?;

    let marked = u32::try_from(shown + scenario_notes).map_err(|_| {
        repo_error(
            "mark_seen",
            format!("{slug} scenario {scenario_id} user {user_id}: the written count does not fit a u32"),
        )
    })?;
    tracing::info!(
        %slug, %scenario_id, user = %user_id,
        shown_items = items.len(), written = marked,
        "practice: a deck's shown items were marked read"
    );
    Ok(Json(ReviewCursorResponse {
        scenario_id,
        marked,
    }))
}

/// One wire item as the repository's own reference.
///
/// ## Rust Learning: two enums for one idea, and why they are not shared
///
/// `SweptItem` is the WIRE shape (it derives serde); `ItemRef` is the
/// repository's (it does not). The same split every DTO in this build makes:
/// a change to how a value is carried cannot silently change how it is stored,
/// and the compiler makes this three-line function the only place the two meet.
fn swept(item: &SweptItem) -> ItemRef {
    match item {
        SweptItem::Answer(id) => ItemRef::Answer(*id),
        SweptItem::Note(id) => ItemRef::Note(*id),
        SweptItem::Change(id) => ItemRef::Change(*id),
    }
}

/// The deck's review bar: the reviewer's backlog on this scenario, and whether
/// THIS user may press Done reviewing.
///
/// The same query as the War Room pill, asked about one scenario, so the bar and
/// the pill cannot disagree. A missing row or a count that is not a `u32` is a
/// logged 500 — never a quiet 0 that would hide the bar.
pub(super) async fn deck_review(
    state: &AppState,
    scenario_id: Uuid,
    user_id: &str,
    is_admin: bool,
) -> Result<DeckReviewDto, AppError> {
    let settings = state.settings.current();
    let reviewers = review_queue_reviewer(&settings);
    // The bar's number is THIS reader's, like every other reading of the queue
    // since L2 — see `review_cursor`'s header for why it stopped being global.
    let rows = awaiting_review(&state.pipeline_pool, &[scenario_id], user_id, reviewers)
        .await
        .map_err(|e| {
            repo_error(
                "awaiting_review",
                format!("scenario {scenario_id} viewer {user_id} reviewers {reviewers:?}: {e}"),
            )
        })?;
    let row = rows.iter().find(|r| r.scenario_id == scenario_id);
    let awaiting = usable_count(&rows, scenario_id).ok_or_else(|| {
        repo_error(
            "awaiting_review",
            format!("no usable count for {scenario_id} in {rows:?}"),
        )
    })?;
    Ok(DeckReviewDto {
        awaiting,
        can_mark_reviewed: can_mark_reviewed(user_id, is_admin, reviewers),
        reviewer_display_name: reviewer_display_line(&settings),
        // Formatted HERE, in the case's own timezone, like every other date on
        // this surface — the browser holds no date format and fills only the
        // stored clause's `{date}`. `None` when the read returned no date, which
        // is the honest shape: a deck with nothing waiting has no oldest item.
        oldest: row.and_then(|r| r.oldest).map(|at| {
            crate::services::practice_clock::local_day_month(
                at,
                &settings.practice_read.case_timezone,
            )
        }),
    })
}

/// Whether this signed-in user may press Done reviewing.
///
/// A thin wrapper over [`crate::services::review_permission::may_review`], kept
/// under this name because the route-table test and `settings_journey_tests`
/// name it: a journey that checked a copy of the rule would pass while the
/// thing it claims to prove was broken.
///
/// ## Domain note: PERMISSION, not display (ruled 2026-09-22)
///
/// `reviewers` is the `practice_reviewer_usernames` row, which is now a DISPLAY
/// list — who the war room names. An administrator may review whether or not
/// any screen names them, which is what this task exists for: Roman took himself
/// off the list to keep his name off the dashboard and lost the button with it.
pub(crate) fn can_mark_reviewed(user_id: &str, is_admin: bool, reviewers: &[String]) -> bool {
    may_review(user_id, is_admin, reviewers)
}

/// The count for `scenario_id`, if the read returned one that fits a `u32`.
///
/// `None` for a missing row or a negative/overflowing count — both are query
/// defects, and the caller turns them into a logged 500 rather than a quiet 0.
fn usable_count(rows: &[AwaitingReviewRow], scenario_id: Uuid) -> Option<u32> {
    rows.iter()
        .find(|r| r.scenario_id == scenario_id)
        .and_then(|r| u32::try_from(r.awaiting).ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(n: u128, awaiting: i64) -> AwaitingReviewRow {
        AwaitingReviewRow {
            scenario_id: Uuid::from_u128(n),
            awaiting,
            oldest: None,
        }
    }

    /// One name on the bench behaves exactly as the old equality test did.
    ///
    /// The migration seeds the bench FROM the single row it replaces, so on the
    /// day this ships every store has exactly one entry. If this behaviour had
    /// moved, it would have moved for every deployment at once.
    #[test]
    fn one_name_behaves_exactly_as_the_old_equality() {
        let one = bench(&["cpenzien"]);
        assert!(can_mark_reviewed("cpenzien", false, &one));
        assert!(!can_mark_reviewed("roman", false, &one));
        assert!(!can_mark_reviewed("docmarie", false, &one));
        // The bench comes from the row, not a literal: change it, and the
        // answer follows.
        let moved = bench(&["roman"]);
        assert!(can_mark_reviewed("roman", false, &moved));
        assert!(!can_mark_reviewed("cpenzien", false, &moved));
    }

    /// Done reviewing is offered to EVERY listed reviewer and to nobody else —
    /// membership, not equality (mutation-proved in the report: negating the
    /// predicate reds this).
    ///
    /// First, last and middle are all asserted on purpose: a predicate that
    /// compared only the first entry would pass a test that checked one name.
    #[test]
    fn can_mark_reviewed_is_membership_over_the_whole_bench() {
        let bench = bench(&["cpenzien", "roman", "jdoe"]);
        assert!(
            can_mark_reviewed("cpenzien", false, &bench),
            "the first entry"
        );
        assert!(can_mark_reviewed("roman", false, &bench), "a middle entry");
        assert!(can_mark_reviewed("jdoe", false, &bench), "the last entry");
        assert!(!can_mark_reviewed("docmarie", false, &bench), "nobody else");
    }

    /// An EMPTY bench offers the button to nobody.
    ///
    /// The boot check refuses a blank row, so this state cannot reach a running
    /// server — but `any` over an empty slice being `false` is what makes the
    /// refusal safe rather than merely early. The opposite (`all`, which is
    /// `true` when empty) would have handed the button to every signed-in user
    /// the moment a row went blank.
    #[test]
    fn an_empty_bench_offers_the_button_to_nobody() {
        assert!(!can_mark_reviewed("cpenzien", false, &[]));
        assert!(!can_mark_reviewed("", false, &[]));
    }

    /// A blank entry never matches, and neither does a near-miss.
    ///
    /// A trailing comma in the settings row is the way a blank entry gets in.
    /// Matching one would grant Done reviewing to a caller whose username the
    /// auth layer left empty, which is the worst possible reading of a typo.
    #[test]
    fn a_blank_or_near_miss_username_never_matches() {
        let bench = bench(&["cpenzien", ""]);
        assert!(
            !can_mark_reviewed("", false, &bench),
            "a blank login is not a member"
        );
        assert!(
            !can_mark_reviewed("CPENZIEN", false, &bench),
            "usernames are ids"
        );
        assert!(
            !can_mark_reviewed("cpenzien ", false, &bench),
            "and are not trimmed"
        );
    }

    /// The route REFUSES a press it should not accept, before it writes.
    ///
    /// ## Why a source scan, and what it is standing in for
    ///
    /// The handler needs an `AppState` (a live pool and a settings snapshot),
    /// which this suite cannot build — the same hole `practice_answers_tests`
    /// records. What can be checked is the SHAPE: the permission is consulted,
    /// and it is consulted BEFORE `mark_reviewed` writes. Ordering is the half
    /// that matters: a check after the write would refuse the response and keep
    /// the row, and the queue reads rows, not responses.
    ///
    /// The live 403 is proved in the journey (docmarie, and an unauthenticated
    /// call), which is the instrument this is a substitute for.
    #[test]
    fn the_put_asks_permission_before_it_writes() {
        let source = std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("src/api/practice_review_cursor.rs"),
        )
        .expect("this module is on disk");
        let from = source
            .find("pub async fn put_review_cursor(")
            .expect("the handler is declared");
        let rest = &source[from..];
        let to = rest[1..]
            .find("\n/// ")
            .map(|i| i + 1)
            .unwrap_or(rest.len());
        // Comments stripped: the doc comment above the handler TALKS about the
        // 403, and a scan that read prose would pass on a handler that does
        // nothing.
        let body: String = rest[..to]
            .lines()
            .map(|line| match line.find("//") {
                Some(at) => &line[..at],
                None => line,
            })
            .collect::<Vec<_>>()
            .join("\n");

        let asked = body.find("may_review(").expect("permission is consulted");
        let refused = body
            .find("AppError::Forbidden")
            .expect("an impermissible press is a 403");
        // `mark_seen` since L2 — the press writes per-item rows now, and this
        // test is about ORDER, so it follows the name of whatever writes.
        let wrote = body.find("mark_seen(").expect("the press is written");
        assert!(asked < wrote, "permission is asked before the write");
        assert!(refused < wrote, "and the refusal returns before the write");
        assert!(
            body.contains("user.is_admin()"),
            "the admin door is the caller's groups, not a list"
        );
    }

    /// The bench as a slice of owned names — the shape the settings row reads as.
    fn bench(names: &[&str]) -> Vec<String> {
        names.iter().map(|n| (*n).to_string()).collect()
    }

    /// The asked-for scenario's count comes back, not a neighbour's.
    #[test]
    fn the_count_is_the_asked_scenarios() {
        let rows = [row(1, 7), row(2, 4)];
        assert_eq!(usable_count(&rows, Uuid::from_u128(2)), Some(4));
    }

    /// A read that forgot the scenario is `None` — never a quiet 0.
    #[test]
    fn a_missing_scenario_is_none() {
        assert_eq!(usable_count(&[row(1, 7)], Uuid::from_u128(9)), None);
    }

    /// A negative count is refused rather than wrapped.
    #[test]
    fn a_negative_count_is_none() {
        assert_eq!(usable_count(&[row(1, -1)], Uuid::from_u128(1)), None);
    }
}
