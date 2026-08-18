//! Writing one scenario's deck, once, with a count proof.
//!
//! The execution half of `seed_practice_deck`, following the one-shot family's
//! law (`oneshot`): dry run is the default, the unit of work is verified before
//! and after, and the proof is real output.
//!
//! ## Why this tool REFUSES rather than overwrites
//!
//! `practice_answers.question_id` is `ON DELETE RESTRICT`. Chuck's sheet is the
//! record of what Marie was asked, and a question deleted out from under an
//! answer would leave a row nobody can read. So a scenario that already carries a
//! deck gets one of two outcomes and never a third: if the stored deck is the
//! same deck, the run is a no-op and says so (this is what makes it idempotent);
//! if it differs, the run refuses with the plan unwritten and names the
//! difference. Editing a live deck is a page in v1, not a re-seed.
//!
//! ## Rust Learning: `sqlx::query` with a transaction, and why the whole seed is one
//!
//! Ten `INSERT`s outside a transaction is ten chances to leave a half-deck
//! behind — and a half-deck is worse than none, because the page renders it
//! confidently. `pool.begin()` yields a `Transaction`; every statement runs
//! against `&mut *tx`; a dropped transaction rolls back, so an early `?` cannot
//! commit anything.

use sqlx::{PgPool, Row};
use uuid::Uuid;

use super::deck_file::{DeckError, DeckFile, DeckQuestion, DeckSourceKind};
use super::sources::{read_sources, ScenarioSources};

/// Why a seed run could not finish.
#[derive(Debug, thiserror::Error)]
pub enum SeedError {
    #[error("the deck file could not be read from {path}: {source}")]
    Unreadable {
        path: String,
        #[source]
        source: std::io::Error,
    },

    #[error("the deck file at {path} is not valid YAML for a deck: {source}")]
    Unparseable {
        path: String,
        #[source]
        source: serde_yaml::Error,
    },

    #[error("{source}")]
    Invalid {
        #[source]
        source: DeckError,
    },

    #[error("no scenario in the pipeline database has the code {code}")]
    NoSuchScenario { code: String },

    #[error(
        "question {position} names {kind} {index}, but scenario {code} has only {available} of them \
         — seed the scenario's evidence first, or fix the deck; nothing was written"
    )]
    SourceOutOfRange {
        position: usize,
        kind: &'static str,
        index: usize,
        code: String,
        available: usize,
    },

    #[error(
        "scenario {code} already carries a deck of {stored} questions that differs from this file \
         ({incoming} questions) — nothing was written. A stored question cannot be replaced while \
         an answer cites it; edit the deck on the page instead"
    )]
    DeckDiffers {
        code: String,
        stored: usize,
        incoming: usize,
    },

    #[error("the database refused the seed: {source}")]
    Database {
        #[source]
        source: sqlx::Error,
    },
}

/// What one run did, in the numbers the report prints.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SeedReport {
    pub scenario_code: String,
    pub scenario_id: Uuid,
    /// Rows in the deck BEFORE this run.
    pub questions_before: usize,
    /// Rows in the deck AFTER it (equal to `questions_before` on a dry run).
    pub questions_after: usize,
    /// How many the file asked for.
    pub questions_planned: usize,
    /// The scenario's ruled instances and talking points, as counted.
    pub instances_available: usize,
    pub points_available: usize,
    /// False on a dry run, and on an already-seeded no-op.
    pub written: bool,
    /// True when the stored deck already matched the file.
    pub already_seeded: bool,
}

/// Read and validate a deck file.
///
/// # Errors
/// [`SeedError::Unreadable`], [`SeedError::Unparseable`] or [`SeedError::Invalid`],
/// each naming the path and what was wrong.
pub fn load_deck(path: &std::path::Path) -> Result<DeckFile, SeedError> {
    let raw = std::fs::read_to_string(path).map_err(|source| SeedError::Unreadable {
        path: path.display().to_string(),
        source,
    })?;
    let deck: DeckFile = serde_yaml::from_str(&raw).map_err(|source| SeedError::Unparseable {
        path: path.display().to_string(),
        source,
    })?;
    deck.validate()
        .map_err(|source| SeedError::Invalid { source })?;
    Ok(deck)
}

/// Resolve one question's `source_ref` against the scenario's own record.
///
/// ## Domain note: this is the whole of "nothing is invented"
///
/// A question claims to come from the third thing Phillips said. This function
/// is where that claim is checked against the scenario's ruled instances, and
/// where it becomes a stored id rather than a promise. A file naming an instance
/// the scenario does not have is a REFUSAL — never a row written with a NULL ref
/// and a receipt still asserting a source.
///
/// # Errors
/// [`SeedError::SourceOutOfRange`], naming the question, the kind and the count.
fn resolve_ref(
    question: &DeckQuestion,
    position: usize,
    sources: &ScenarioSources,
    code: &str,
) -> Result<Option<String>, SeedError> {
    let index = match question.source_index {
        None => return Ok(None), // validated: only a manual question reaches this
        Some(index) => index,
    };
    let (available, resolved) = match question.source_kind {
        DeckSourceKind::Instance => (
            sources.instances.len(),
            sources.instances.get(index - 1).cloned(),
        ),
        DeckSourceKind::Point => (
            sources.points.len(),
            sources.points.get(index - 1).map(|id| id.to_string()),
        ),
        DeckSourceKind::Manual => (0, None),
    };
    resolved.map(Some).ok_or(SeedError::SourceOutOfRange {
        position,
        kind: question.source_kind.as_column(),
        index,
        code: code.to_string(),
        available,
    })
}

/// Plan the deck, prove it, and — with `apply` — write it in one transaction.
///
/// # Errors
/// Any [`SeedError`]. Nothing is written unless the return is `Ok` with
/// `written: true`.
pub async fn run(pool: &PgPool, deck: &DeckFile, apply: bool) -> Result<SeedReport, SeedError> {
    let code = deck.scenario_code.trim().to_string();
    let sources = read_sources(pool, &code).await?;

    // Resolve EVERY ref before a transaction is opened. A refusal found half-way
    // through the writes would roll back correctly, but the operator would read
    // "the database refused" for what is a fixable typo in a file.
    let mut refs: Vec<Option<String>> = Vec::with_capacity(deck.questions.len());
    for (i, question) in deck.questions.iter().enumerate() {
        refs.push(resolve_ref(question, i + 1, &sources, &code)?);
    }

    let before = count_questions(pool, sources.scenario_id).await?;
    if before > 0 {
        return finish_already_seeded(&code, &sources, deck, before);
    }

    let mut report = SeedReport {
        scenario_code: code,
        scenario_id: sources.scenario_id,
        questions_before: before,
        questions_after: before,
        questions_planned: deck.questions.len(),
        instances_available: sources.instances.len(),
        points_available: sources.points.len(),
        written: false,
        already_seeded: false,
    };
    if !apply {
        return Ok(report);
    }

    write_deck(pool, sources.scenario_id, deck, &refs).await?;
    report.questions_after = count_questions(pool, sources.scenario_id).await?;
    report.written = true;
    Ok(report)
}

/// The already-seeded arm: a no-op when the counts agree, a refusal when they
/// do not. See the module header for why this is never an overwrite.
fn finish_already_seeded(
    code: &str,
    sources: &ScenarioSources,
    deck: &DeckFile,
    before: usize,
) -> Result<SeedReport, SeedError> {
    if before != deck.questions.len() {
        return Err(SeedError::DeckDiffers {
            code: code.to_string(),
            stored: before,
            incoming: deck.questions.len(),
        });
    }
    Ok(SeedReport {
        scenario_code: code.to_string(),
        scenario_id: sources.scenario_id,
        questions_before: before,
        questions_after: before,
        questions_planned: deck.questions.len(),
        instances_available: sources.instances.len(),
        points_available: sources.points.len(),
        written: false,
        already_seeded: true,
    })
}

/// How many questions the scenario's deck holds right now.
async fn count_questions(pool: &PgPool, scenario_id: Uuid) -> Result<usize, SeedError> {
    let row = sqlx::query("SELECT COUNT(*) AS n FROM practice_questions WHERE scenario_id = $1")
        .bind(scenario_id)
        .fetch_one(pool)
        .await
        .map_err(|source| SeedError::Database { source })?;
    let n: i64 = row
        .try_get("n")
        .map_err(|source| SeedError::Database { source })?;
    Ok(usize::try_from(n).unwrap_or(0))
}

/// Insert the whole deck, or none of it.
async fn write_deck(
    pool: &PgPool,
    scenario_id: Uuid,
    deck: &DeckFile,
    refs: &[Option<String>],
) -> Result<(), SeedError> {
    let mut tx = pool
        .begin()
        .await
        .map_err(|source| SeedError::Database { source })?;

    for (i, question) in deck.questions.iter().enumerate() {
        sqlx::query(
            "INSERT INTO practice_questions \
             (scenario_id, side, text, tactic, braid_rows, source_kind, source_ref, receipt, \
              watch_for, stronger, stronger_lean, pair_said, pair_admitted, sort_order, created_by) \
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15)",
        )
        .bind(scenario_id)
        .bind(question.side.as_column())
        .bind(question.text.trim())
        .bind(question.tactic)
        .bind(question.braid_rows.as_deref().map(str::trim))
        .bind(question.source_kind.as_column())
        .bind(refs[i].as_deref())
        .bind(question.receipt.as_deref().map(str::trim))
        .bind(question.watch_for.as_deref().map(str::trim))
        .bind(question.stronger.as_deref().map(str::trim))
        .bind(question.stronger_lean.as_deref().map(str::trim))
        .bind(question.pair_said.as_deref().map(str::trim))
        .bind(question.pair_admitted.as_deref().map(str::trim))
        .bind(i32::try_from(i + 1).unwrap_or(i32::MAX))
        .bind("seed_practice_deck")
        .execute(&mut *tx)
        .await
        .map_err(|source| SeedError::Database { source })?;
    }

    tx.commit()
        .await
        .map_err(|source| SeedError::Database { source })
}

/// Render the count proof the operator reads and the report file holds.
///
/// One function for both, so the file cannot say something the terminal did not
/// (the one-shot family's law).
pub fn render_report(report: &SeedReport) -> String {
    let verdict = match (report.written, report.already_seeded) {
        (true, _) => "WROTE the deck",
        (false, true) => "NO-OP — this scenario already carries this deck",
        (false, false) => "DRY RUN — nothing was written",
    };
    format!(
        "seed_practice_deck — {code}\n\
         =============================================\n\
         scenario_id           {id}\n\
         ruled instances       {instances}\n\
         talking points        {points}\n\
         questions in file     {planned}\n\
         questions before      {before}\n\
         questions after       {after}\n\
         \n\
         {verdict}\n",
        code = report.scenario_code,
        id = report.scenario_id,
        instances = report.instances_available,
        points = report.points_available,
        planned = report.questions_planned,
        before = report.questions_before,
        after = report.questions_after,
    )
}

#[cfg(test)]
#[path = "seed_tests.rs"]
mod tests;
