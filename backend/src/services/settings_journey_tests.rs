//! THE JOURNEYS (Law 23) — walked end to end against a live pipeline database.
//!
//! Law 23 was written on 2026-09-20 because v2.1.14 shipped with 5,540 green
//! tests and it was IMPOSSIBLE to add a second reviewer through its own settings
//! page. Every test checked a state; none walked the task. The guard firing was
//! itself a tested, celebrated behaviour.
//!
//! So these do not check states. Each one starts from the store as it actually
//! ships, goes through the application's own writes and NOTHING else, and ends
//! at the outcome a human was promised:
//!
//!   * **J1 — "Add a second reviewer."** The release's reason to exist.
//!   * **J2 — "Remove a reviewer."**
//!   * **J3 — "Find and change any ordinary setting."** The mechanism must not
//!     leak onto rows that were never coupled.
//!
//! ## They must not be pointed at DEV's real database
//!
//! Every test here WRITES the reviewer rows and restores them afterwards. Point
//! `PIPELINE_DATABASE_URL` at a throwaway database copied from
//! `colossus_legal_v2`, exactly as `theme_scan_judge_live_tests` requires:
//!
//! ```text
//! cargo test --lib settings_journey -- --ignored --test-threads=1
//! ```
//!
//! `--test-threads=1` is not optional: they all edit the same two rows.

use sqlx::PgPool;

use crate::api::practice_review_cursor::can_mark_reviewed;
use crate::domain::practice_params::{
    KEY_PRACTICE_REVIEWER_DISPLAY_NAMES, KEY_PRACTICE_REVIEWER_USERNAMES,
};
use crate::services::settings_boot::load_settings;
use crate::services::settings_groups::group_by_id;
use crate::services::settings_handle::SettingsHandle;
use crate::services::settings_template_file::TemplateDir;
use crate::services::war_room_progress::reviewer_display_line;

use super::*;

type TestResult<T> = Result<T, Box<dyn std::error::Error>>;

/// A pool against whatever `PIPELINE_DATABASE_URL` names. Same helper shape as
/// `theme_scan_live_fixture::pipeline_pool`.
async fn pool() -> TestResult<PgPool> {
    // best-effort: a missing .env is normal when the URL comes from the shell,
    // which is how a scratch database is pointed at. The connect fails loudly
    // either way.
    dotenvy::dotenv().ok();
    let url = std::env::var("PIPELINE_DATABASE_URL")?;
    Ok(sqlx::postgres::PgPoolOptions::new()
        .max_connections(4)
        .connect(&url)
        .await?)
}

/// A live handle, so the write path swaps a real snapshot as it does in the app.
async fn handle(pool: &PgPool) -> TestResult<SettingsHandle> {
    Ok(SettingsHandle::new(load_settings(pool).await?))
}

async fn value_of(pool: &PgPool, key: &str) -> TestResult<String> {
    Ok(get_setting(pool, key)
        .await?
        .ok_or("the row must exist")?
        .value)
}

/// Put the bench back to the one reviewer every deployment ships with.
///
/// Through the pair route, because that is the only way to write these two rows
/// — which is itself the fix, and means the teardown re-proves it every run.
async fn restore_shipped_bench(pool: &PgPool, handle: &SettingsHandle) -> TestResult<()> {
    let bench = group_by_id("reviewer_bench").ok_or("declared")?;
    if value_of(pool, KEY_PRACTICE_REVIEWER_USERNAMES).await? != "cpenzien" {
        set_setting_pair(
            pool,
            handle,
            bench,
            &[vec!["cpenzien".to_string(), "Chuck".to_string()]],
            "__journey_teardown",
        )
        .await?;
    }
    Ok(())
}

/// ⚑ J1 — "ADD A SECOND REVIEWER". The release's reason to exist. (M)
///
/// Red against v2.1.14, green after. Both halves are kept: the refusal of a
/// one-row-at-a-time edit is not a scaffold to delete on green, it is step 1's
/// own requirement and stays as a permanent assertion.
#[tokio::test]
#[ignore = "needs a live pipeline database — see the module doc; point PIPELINE_DATABASE_URL at a scratch copy"]
async fn j1_add_a_second_reviewer() -> TestResult<()> {
    let pool = pool().await?;
    let handle = handle(&pool).await?;
    let bench = group_by_id("reviewer_bench").ok_or("declared")?;
    restore_shipped_bench(&pool, &handle).await?;

    // ── The shipped state, read from the store rather than assumed ──────────
    assert_eq!(
        value_of(&pool, KEY_PRACTICE_REVIEWER_USERNAMES).await?,
        "cpenzien"
    );
    assert_eq!(
        value_of(&pool, KEY_PRACTICE_REVIEWER_DISPLAY_NAMES).await?,
        "Chuck"
    );

    // ── THE DEADLOCK, still there underneath ───────────────────────────────
    //
    // The boot check is untouched by this task, and this proves it: a trial
    // holding the NEW logins beside the OLD names is refused, exactly as it was
    // on v2.1.14. That refusal is correct. It was never the bug — the bug was
    // that it was the only path a human had.
    let one_sided = trial_snapshot_many(
        &pool,
        &[(KEY_PRACTICE_REVIEWER_USERNAMES, "cpenzien,roman")],
    )
    .await;
    assert!(
        one_sided.is_err(),
        "the cross-row invariant must still refuse a half-updated store — if this \
         passes, the fix weakened the check instead of feeding it both values"
    );

    // ── Either row, saved alone, is refused BY NAME and writes nothing ─────
    for key in [
        KEY_PRACTICE_REVIEWER_USERNAMES,
        KEY_PRACTICE_REVIEWER_DISPLAY_NAMES,
    ] {
        let refused = set_setting(
            &pool,
            &handle,
            key,
            "cpenzien,roman",
            "__journey",
            &TemplateDir::new("."),
        )
        .await
        .expect_err("a coupled row cannot be saved alone");

        match refused {
            SettingsError::Coupled { key: named, group } => {
                assert_eq!(named, key);
                assert!(
                    group.contains("Done reviewing"),
                    "names the editor: {group}"
                );
            }
            other => panic!("expected a Coupled refusal for {key}, got {other:?}"),
        }
    }
    assert_eq!(
        value_of(&pool, KEY_PRACTICE_REVIEWER_USERNAMES).await?,
        "cpenzien",
        "a refused save must leave the store exactly as it was"
    );

    // ── THE JOURNEY: both reviewers, one write ─────────────────────────────
    let settings = set_setting_pair(
        &pool,
        &handle,
        bench,
        &[
            vec!["cpenzien".to_string(), "Chuck".to_string()],
            vec!["roman".to_string(), "Roman".to_string()],
        ],
        "__journey",
    )
    .await?;

    // ── The promised outcome, in the store AND in the running snapshot ─────
    assert_eq!(
        value_of(&pool, KEY_PRACTICE_REVIEWER_USERNAMES).await?,
        "cpenzien,roman"
    );
    assert_eq!(
        value_of(&pool, KEY_PRACTICE_REVIEWER_DISPLAY_NAMES).await?,
        "Chuck,Roman"
    );

    let bench_now = &settings.practice_read.reviewer_usernames;
    assert!(
        can_mark_reviewed("roman", bench_now),
        "the whole point: Roman can now press Done reviewing"
    );
    assert!(
        can_mark_reviewed("cpenzien", bench_now),
        "and Chuck still can"
    );
    assert!(!can_mark_reviewed("docmarie", bench_now));

    let line = reviewer_display_line(&settings);
    assert!(
        line.contains("Chuck") && line.contains("Roman"),
        "the queue line names both: {line}"
    );

    restore_shipped_bench(&pool, &handle).await?;
    Ok(())
}

/// J2 — "REMOVE A REVIEWER", and the empty bench that must never be reachable.
#[tokio::test]
#[ignore = "needs a live pipeline database — see the module doc; point PIPELINE_DATABASE_URL at a scratch copy"]
async fn j2_remove_a_reviewer() -> TestResult<()> {
    let pool = pool().await?;
    let handle = handle(&pool).await?;
    let bench = group_by_id("reviewer_bench").ok_or("declared")?;
    restore_shipped_bench(&pool, &handle).await?;

    // Start from two, through the app's own write.
    set_setting_pair(
        &pool,
        &handle,
        bench,
        &[
            vec!["cpenzien".to_string(), "Chuck".to_string()],
            vec!["roman".to_string(), "Roman".to_string()],
        ],
        "__journey",
    )
    .await?;

    // ── Two back down to one ───────────────────────────────────────────────
    let settings = set_setting_pair(
        &pool,
        &handle,
        bench,
        &[vec!["roman".to_string(), "Roman".to_string()]],
        "__journey",
    )
    .await?;

    assert_eq!(
        value_of(&pool, KEY_PRACTICE_REVIEWER_USERNAMES).await?,
        "roman"
    );
    assert_eq!(
        value_of(&pool, KEY_PRACTICE_REVIEWER_DISPLAY_NAMES).await?,
        "Roman"
    );
    assert!(!can_mark_reviewed(
        "cpenzien",
        &settings.practice_read.reviewer_usernames
    ));
    assert_eq!(reviewer_display_line(&settings), "Roman");

    // ── ⚑ Removing the LAST reviewer is refused ────────────────────────────
    //
    // Newly reachable: `("none","none")` satisfies the length check as 0 == 0,
    // and until this route existed nobody could submit both rows at once to get
    // there. A bench with nobody on it means the review queue can never be
    // cleared by anyone.
    let refused = set_setting_pair(&pool, &handle, bench, &[], "__journey")
        .await
        .expect_err("an empty bench must be refused");
    assert!(
        matches!(refused, SettingsError::Pair { .. }),
        "got {refused:?}"
    );
    assert!(
        refused.to_string().contains("at least one reviewer"),
        "{refused}"
    );
    assert_eq!(
        value_of(&pool, KEY_PRACTICE_REVIEWER_USERNAMES).await?,
        "roman",
        "the refusal wrote nothing"
    );

    restore_shipped_bench(&pool, &handle).await?;
    Ok(())
}

/// J3 — "FIND AND CHANGE ANY ORDINARY SETTING." The mechanism must not leak.
#[tokio::test]
#[ignore = "needs a live pipeline database — see the module doc; point PIPELINE_DATABASE_URL at a scratch copy"]
async fn j3_an_ordinary_setting_still_saves_on_its_own() -> TestResult<()> {
    let pool = pool().await?;
    let handle = handle(&pool).await?;
    let templates = TemplateDir::new(".");

    let before = value_of(&pool, "talking_points_cap").await?;
    let candidate = if before == "5" { "4" } else { "5" };

    set_setting(
        &pool,
        &handle,
        "talking_points_cap",
        candidate,
        "__journey",
        &templates,
    )
    .await?;
    assert_eq!(value_of(&pool, "talking_points_cap").await?, candidate);

    // Put it back, which exercises the ordinary path a second time.
    set_setting(
        &pool,
        &handle,
        "talking_points_cap",
        &before,
        "__journey",
        &templates,
    )
    .await?;
    assert_eq!(value_of(&pool, "talking_points_cap").await?, before);
    Ok(())
}

/// ⚑ THE ATOMICITY PROOF — the mutation this task was told to run. (M)
///
/// The instruction: *"break the pair transaction into two writes → J1's
/// atomicity assertion reds (a crash between writes may not leave a mismatched
/// store)."*
///
/// A crash cannot be staged, so the failure is staged instead: `commit_pair` is
/// handed one row that exists and one that does not. The second write matches
/// zero rows, the function returns without committing, and the transaction is
/// dropped — which rolls the FIRST row back.
///
/// Split that one transaction into two and the first row lands. This test is
/// then the only thing in the suite that notices, and what it notices is a store
/// whose two lists are different lengths — the state that refuses to boot.
#[tokio::test]
#[ignore = "needs a live pipeline database — see the module doc; point PIPELINE_DATABASE_URL at a scratch copy"]
async fn the_pair_write_is_one_transaction_or_nothing() -> TestResult<()> {
    let pool = pool().await?;
    let handle = handle(&pool).await?;
    restore_shipped_bench(&pool, &handle).await?;

    let before = value_of(&pool, KEY_PRACTICE_REVIEWER_USERNAMES).await?;

    let doomed = vec![
        (
            EncodedRow {
                key: KEY_PRACTICE_REVIEWER_USERNAMES,
                value: "cpenzien,roman".to_string(),
            },
            before.clone(),
        ),
        (
            EncodedRow {
                key: "__no_such_settings_row__",
                value: "anything".to_string(),
            },
            "anything_else".to_string(),
        ),
    ];

    let refused = commit_pair(&pool, "reviewer_bench", &doomed, "__journey")
        .await
        .expect_err("a row that matches nothing must fail the whole group");
    assert!(
        matches!(refused, SettingsError::UnknownKey { .. }),
        "got {refused:?}"
    );

    assert_eq!(
        value_of(&pool, KEY_PRACTICE_REVIEWER_USERNAMES).await?,
        before,
        "THE ATOMICITY ASSERTION: the first row must have rolled back. If this \
         reads 'cpenzien,roman', the group is no longer written in one \
         transaction and a failure part-way through leaves the two lists a \
         different length — a store that refuses to boot, after a page that said \
         it saved."
    );
    Ok(())
}
