//! `--update`: bringing a stored deck into line with its file, without losing a row.
//!
//! The second way `seed_practice_deck` can be run. [`super::seed`] writes a deck
//! ONCE and refuses to touch one that already exists; this brings an existing
//! deck up to date with the file, which is what makes Chuck's Thursday review
//! and the architect's re-ordering able to meet in the middle.
//!
//! ## The law this path obeys, stated once
//!
//! - It matches by `deck_key`, never by text. Text is what a re-wording changes,
//!   and matching on it is how an edit becomes a duplicate row.
//! - It NEVER deletes. A stored question is cited by `practice_answers` under
//!   `ON DELETE RESTRICT`, and Chuck's sheet is the record of what Marie was
//!   actually asked.
//! - It never touches `practice_answers`, and never touches a row's flag: the
//!   flag is Marie's complaint about a question, not something the file authors.
//! - Rows the file no longer mentions are LEFT ALONE and listed in the report.
//!   Hiding a question is an act with an author (the deck editor); a seed run
//!   silently retiring one is not.
//!
//! ## The one-time text match, and why it is one-time
//!
//! S-5's ten rows were seeded before `deck_key` existed. There is exactly one
//! honest way to give them their keys: match each stored row to a file question
//! by its EXACT text, once, and refuse the whole run if any stored row cannot be
//! matched or if two rows share a text. After that pass they carry keys and this
//! path never looks at text again.

use sqlx::{PgPool, Row};
use uuid::Uuid;

use super::deck_file::DeckFile;
use super::seed::{resolve_refs, SeedError};
use super::seed_rows::{insert_question, set_deck_key, update_question};
use super::sources::read_sources;

/// Why an `--update` run could not finish.
///
/// Separate from [`SeedError`] rather than more variants on it: these are the
/// refusals of a DIFFERENT act. A first seed refuses because the file cannot be
/// used; an update refuses because the file and the stored deck cannot be
/// reconciled without guessing — and guessing here means Chuck's edits landing
/// on a question he did not mean.
#[derive(Debug, thiserror::Error)]
pub enum UpdateError {
    #[error("{source}")]
    Seed {
        #[source]
        source: SeedError,
    },

    #[error(
        "question {position} in the file has no `key`; --update matches on keys and cannot run \
         without them — add `key: g1` (and so on) to every question. Nothing was written"
    )]
    FileQuestionHasNoKey { position: usize },

    #[error(
        "the stored question “{text}” carries no deck_key and no question in the file has that \
         exact text, so this run cannot tell which key it should get. Fix the file's text to \
         match, or give the stored row its key by hand. Nothing was written"
    )]
    StoredRowUnmatched { text: String },

    #[error(
        "two questions share the exact text “{text}”, so matching the un-keyed stored rows by \
         text would be a guess. Nothing was written"
    )]
    AmbiguousText { text: String },

    #[error("the database refused the update: {source}")]
    Database {
        #[source]
        source: sqlx::Error,
    },
}

/// One stored deck row, as the plan needs to see it.
struct StoredQuestion {
    id: Uuid,
    deck_key: Option<String>,
    text: String,
}

/// What one `--update` run did, in the numbers the report prints.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct UpdateReport {
    pub scenario_code: String,
    pub scenario_id: Uuid,
    /// `key → the stored text it was matched to`, for rows that had no key.
    pub keyed_by_text: Vec<(String, String)>,
    /// Keys whose stored row this run rewrote.
    pub updated: Vec<String>,
    /// Keys the file carries that the deck did not.
    pub inserted: Vec<String>,
    /// Stored questions the file no longer mentions. Left exactly as they are.
    pub untouched: Vec<String>,
    pub written: bool,
}

/// Plan an update, prove it, and — with `apply` — perform it in one transaction.
///
/// # Errors
/// Any [`UpdateError`]. Nothing is written unless the return is `Ok` with
/// `written: true`.
pub async fn run_update(
    pool: &PgPool,
    deck: &DeckFile,
    apply: bool,
) -> Result<UpdateReport, UpdateError> {
    let code = deck.scenario_code.trim().to_string();
    let sources = read_sources(pool, &code)
        .await
        .map_err(|source| UpdateError::Seed { source })?;
    // Every ref is resolved before a transaction opens, for the reason the first
    // seed gives: a refusal found half-way through the writes rolls back
    // correctly, but the operator reads "the database refused" for what is a
    // fixable typo in a file.
    let refs =
        resolve_refs(deck, &sources, &code).map_err(|source| UpdateError::Seed { source })?;

    let keys = file_keys(deck)?;
    let stored = read_stored(pool, sources.scenario_id).await?;
    let assignments = match_unkeyed(&stored, deck, &keys)?;
    // With the assignments applied, every stored row's key is known — so the
    // plan below is decided on keys alone, which is the whole point of the pass.
    let key_of = keys_after(&stored, &assignments);

    let mut report = plan(&code, sources.scenario_id, &keys, &key_of, &assignments);

    if !apply {
        return Ok(report);
    }

    apply_update(
        pool,
        sources.scenario_id,
        deck,
        &refs,
        &assignments,
        &key_of,
    )
    .await?;
    report.written = true;
    Ok(report)
}

/// What this run WOULD do, decided on keys alone.
///
/// Split from [`run_update`] so that function reads as the five steps it is —
/// resolve, key, plan, stop-if-dry, write. This half touches no database and no
/// transaction, which is also what makes the plan a unit test rather than a run
/// against DEV.
fn plan(
    code: &str,
    scenario_id: Uuid,
    keys: &[&str],
    key_of: &[(Uuid, String)],
    assignments: &[(Uuid, &str, String)],
) -> UpdateReport {
    let mut report = UpdateReport {
        scenario_code: code.to_string(),
        scenario_id,
        keyed_by_text: assignments
            .iter()
            .map(|(_, key, text)| ((*key).to_string(), text.clone()))
            .collect(),
        ..Default::default()
    };
    for key in keys {
        match key_of.iter().find(|(_, held)| held == key) {
            Some(_) => report.updated.push((*key).to_string()),
            None => report.inserted.push((*key).to_string()),
        }
    }
    // Rows the file no longer mentions are LEFT ALONE and listed. Hiding a
    // question is an act with an author (the deck editor); a seed run silently
    // retiring one is not.
    report.untouched = key_of
        .iter()
        .filter(|(_, held)| !keys.iter().any(|key| key == held))
        .map(|(_, held)| held.clone())
        .collect();
    report
}

/// Every stored row's key, once the one-time text match has been applied.
///
/// Rows that already carried one keep it; rows the match just named get theirs.
/// A row that is in neither list simply has no key and is therefore in neither
/// the update plan nor the "left as they are" list — which cannot happen, since
/// [`match_unkeyed`] refuses the run rather than leaving one unnamed.
fn keys_after(
    stored: &[StoredQuestion],
    assignments: &[(Uuid, &str, String)],
) -> Vec<(Uuid, String)> {
    let mut out: Vec<(Uuid, String)> = stored
        .iter()
        .filter_map(|row| row.deck_key.clone().map(|key| (row.id, key)))
        .collect();
    out.extend(
        assignments
            .iter()
            .map(|(id, key, _)| (*id, (*key).to_string())),
    );
    out
}

/// Every file question's key, in file order, or the first one that has none.
///
/// A separate pass because `--update` cannot do anything at all without them:
/// discovering the fifth question is un-keyed after four writes would leave a
/// half-updated deck, and discovering it before any read is a sentence an
/// operator can act on.
fn file_keys(deck: &DeckFile) -> Result<Vec<&str>, UpdateError> {
    deck.questions
        .iter()
        .enumerate()
        .map(|(i, question)| {
            question
                .key
                .as_deref()
                .map(str::trim)
                .ok_or(UpdateError::FileQuestionHasNoKey { position: i + 1 })
        })
        .collect()
}

/// The scenario's stored deck, as the plan needs it.
async fn read_stored(pool: &PgPool, scenario_id: Uuid) -> Result<Vec<StoredQuestion>, UpdateError> {
    let rows = sqlx::query(
        "SELECT id, deck_key, text FROM practice_questions \
         WHERE scenario_id = $1 ORDER BY sort_order, id",
    )
    .bind(scenario_id)
    .fetch_all(pool)
    .await
    .map_err(|source| UpdateError::Database { source })?;

    rows.into_iter()
        .map(|row| {
            Ok(StoredQuestion {
                id: row
                    .try_get("id")
                    .map_err(|source| UpdateError::Database { source })?,
                deck_key: row
                    .try_get("deck_key")
                    .map_err(|source| UpdateError::Database { source })?,
                text: row
                    .try_get("text")
                    .map_err(|source| UpdateError::Database { source })?,
            })
        })
        .collect()
}

/// Which key each un-keyed stored row should receive, matched by exact text.
///
/// Returns `(stored id, key, the stored text)` for every row that needs one.
///
/// # Errors
/// [`UpdateError::AmbiguousText`] when two file questions share a text — the
/// match would then be a coin toss — and [`UpdateError::StoredRowUnmatched`]
/// when a stored row's text appears in no file question. Both refuse the WHOLE
/// run: a partial keying would leave the deck in a state where a second run
/// behaved differently, which is the worst thing a re-runnable tool can do.
fn match_unkeyed<'a>(
    stored: &[StoredQuestion],
    deck: &DeckFile,
    keys: &[&'a str],
) -> Result<Vec<(Uuid, &'a str, String)>, UpdateError> {
    let mut out = Vec::new();
    for row in stored.iter().filter(|row| row.deck_key.is_none()) {
        let wanted = row.text.trim();
        let matches: Vec<usize> = deck
            .questions
            .iter()
            .enumerate()
            .filter(|(_, q)| q.text.trim() == wanted)
            .map(|(i, _)| i)
            .collect();
        match matches.as_slice() {
            [] => {
                return Err(UpdateError::StoredRowUnmatched {
                    text: row.text.clone(),
                })
            }
            [only] => out.push((row.id, keys[*only], row.text.clone())),
            _ => {
                return Err(UpdateError::AmbiguousText {
                    text: row.text.clone(),
                })
            }
        }
    }
    Ok(out)
}

/// Write the plan: the one-time keys, then every update and insert, in one
/// transaction.
///
/// One transaction for the reason the first seed gives: a half-updated deck is
/// worse than an un-updated one, because the page renders it confidently.
async fn apply_update(
    pool: &PgPool,
    scenario_id: Uuid,
    deck: &DeckFile,
    refs: &[Option<String>],
    assignments: &[(Uuid, &str, String)],
    key_of: &[(Uuid, String)],
) -> Result<(), UpdateError> {
    let mut tx = pool
        .begin()
        .await
        .map_err(|source| UpdateError::Database { source })?;

    for (id, key, _) in assignments {
        set_deck_key(&mut tx, *id, key)
            .await
            .map_err(|source| UpdateError::Database { source })?;
    }

    for (i, question) in deck.questions.iter().enumerate() {
        let key = question.key.as_deref().map(str::trim).unwrap_or_default();
        let sort_order = i32::try_from(i + 1).unwrap_or(i32::MAX);
        let existing = key_of
            .iter()
            .find(|(_, held)| held == key)
            .map(|(id, _)| *id);
        let written = match existing {
            Some(id) => {
                update_question(&mut tx, id, question, refs[i].as_deref(), sort_order).await
            }
            None => {
                insert_question(
                    &mut tx,
                    scenario_id,
                    question,
                    refs[i].as_deref(),
                    sort_order,
                )
                .await
            }
        };
        written.map_err(|source| UpdateError::Database { source })?;
    }

    tx.commit()
        .await
        .map_err(|source| UpdateError::Database { source })
}

/// Render the count proof the operator reads and the report file holds.
///
/// One function for both, so the file cannot say something the terminal did not
/// (the one-shot family's law).
pub fn render_update_report(report: &UpdateReport) -> String {
    let verdict = if report.written {
        "UPDATED the deck"
    } else {
        "DRY RUN — nothing was written"
    };
    let list = |label: &str, values: &[String]| {
        if values.is_empty() {
            format!("{label:<22}(none)\n")
        } else {
            format!("{label:<22}{}\n", values.join(", "))
        }
    };
    format!(
        "seed_practice_deck --update — {code}\n\
         =============================================\n\
         scenario_id           {id}\n\
         {keyed}\
         {updated}\
         {inserted}\
         {untouched}\
         \n\
         {verdict}\n",
        code = report.scenario_code,
        id = report.scenario_id,
        keyed = list(
            "keyed by text",
            &report
                .keyed_by_text
                .iter()
                .map(|(key, text)| format!("{key} ← “{}…”", head(text)))
                .collect::<Vec<_>>()
        ),
        updated = list("updated", &report.updated),
        inserted = list("inserted", &report.inserted),
        untouched = list("left as they are", &report.untouched),
    )
}

/// The first few words of a question, for a report line that must stay one line.
fn head(text: &str) -> String {
    text.split_whitespace()
        .take(6)
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
#[path = "seed_update_tests.rs"]
mod tests;
