//! The loader's database and graph half.
//!
//! Every function here does I/O and nothing else decides anything: the plan was
//! settled in `plan.rs`, and this applies it. That split is what makes the dry
//! run a real dry run — it runs the whole of `plan.rs` and none of this file,
//! except the two READS that the plan itself depends on.
//!
//! ## Why each mode gets its own transaction
//!
//! A load is a runbook step an operator watches. Half a scenario's cards written
//! and half not is a deck nobody can reason about — the reader cannot tell a card
//! the machine skipped from one the loader dropped. One transaction per `--apply`
//! means the answer is always "all of it" or "none of it".

use anyhow::{bail, Context, Result};
use std::collections::HashMap;

use colossus_legal_backend::domain::fact_status::FactStatus;
use colossus_legal_backend::repositories::pipeline_repository::scenario_fact_cards::{
    write_field, FieldWrite,
};

use super::plan::{CardPlan, PlannedPicks, PlannedPoints, LOADER_AUTHOR};

/// Every accusation's SHORT id → its full graph node id.
///
/// Job B abbreviates an accusation to the hash suffix of its node id
/// (`"45984d77"`), and the card stores the full id like everything else in this
/// system. This is that map, read once.
///
/// ## Why the short id is derived here and not matched with `ENDS WITH`
///
/// A suffix match in Cypher would scan every accusation per lookup and, worse,
/// could match two — nothing guarantees the suffixes are unique in the query's
/// eyes. Deriving the key in Rust makes the collision VISIBLE: a repeated suffix
/// is refused below, by name, rather than silently resolving to whichever node
/// the database returned first. (`allegations.json` reports
/// `short_id_collisions: 0` for the current graph; this is what keeps that true.)
///
/// # Errors
/// Returns the query's failure, or the first suffix two accusations share.
pub async fn short_id_index(graph: &neo4rs::Graph) -> Result<HashMap<String, String>> {
    let mut stream = graph
        .execute(
            neo4rs::query("MATCH (a) WHERE labels(a)[0] = $label RETURN a.id AS id").param(
                "label",
                colossus_legal_backend::models::document_status::ENTITY_ALLEGATION,
            ),
        )
        .await
        .context("reading the accusation ids")?;

    let mut index: HashMap<String, String> = HashMap::new();
    while let Some(row) = stream.next().await.context("decoding an accusation row")? {
        let id: String = row.get("id").context("an accusation with no id")?;
        let short = id.rsplit(':').next().unwrap_or(&id).to_string();
        if let Some(existing) = index.insert(short.clone(), id.clone()) {
            bail!(
                "two accusations share the short id {short}: {existing} and {id}. \
                 Job B names accusations by that suffix, so this file cannot be \
                 loaded without deciding which it means"
            );
        }
    }
    Ok(index)
}

/// The settings snapshot, read from the store.
///
/// Needed only by the talking-point mode, whose service checks the stored cap.
/// Read here rather than defaulted, because a loader that used its own idea of
/// the cap would write a list the application then refuses to serve whole.
///
/// # Errors
/// Returns the read's failure, or the first parameter the store cannot supply.
pub async fn load_settings(
    pool: &sqlx::PgPool,
) -> Result<colossus_legal_backend::domain::settings::Settings> {
    colossus_legal_backend::services::settings_boot::load_settings(pool)
        .await
        .context("reading the settings the talking-point cap lives in")
}

/// The scenario id behind a code like `S-11`.
///
/// ## Why the code and not the id on the command line
///
/// `S-11` is what the runbook step says and what a human can check against the
/// file they are holding. A UUID on the command line is a value nobody can
/// verify by reading it.
///
/// # Errors
/// Returns a message naming the code when no scenario carries it.
pub async fn scenario_id_for_code(pool: &sqlx::PgPool, code: &str) -> Result<uuid::Uuid> {
    // The code is stored as its ordinal: `S-11` is `code_ordinal` 11, and the
    // `S-` is `domain::scenario_code`'s rendering of it.
    let ordinal: i32 = code
        .trim_start_matches(['S', 's', '-'])
        .parse()
        .with_context(|| format!("{code} is not a scenario code like S-11"))?;

    let id: Option<uuid::Uuid> =
        sqlx::query_scalar("SELECT scenario_id FROM scenarios WHERE code_ordinal = $1")
            .bind(ordinal)
            .fetch_optional(pool)
            .await
            .context("looking up the scenario")?;

    id.ok_or_else(|| anyhow::anyhow!("no scenario carries the code {code}"))
}

/// Write every planned field, in ONE transaction.
///
/// Returns how many fields were written — the number the caller prints and the
/// operator checks against the plan.
///
/// ## Why the loader uses the same writer the edit route does
///
/// `write_field` also appends the ledger row, so a machine draft and a human edit
/// leave the same kind of record. A second write path here would be a second
/// place the ledger could be forgotten — and the ledger is the only thing that
/// can later say a sentence started as a draft.
pub async fn apply_cards(
    pool: &sqlx::PgPool,
    scenario_id: uuid::Uuid,
    plan: &CardPlan,
) -> Result<usize> {
    let now = chrono::Utc::now();
    for field in &plan.fields {
        write_field(
            pool,
            &FieldWrite {
                scenario_id,
                graph_node_id: &field.graph_node_id,
                field: field.field,
                value: field.value.as_deref(),
                author: LOADER_AUTHOR,
                written_at: now,
            },
        )
        .await
        .with_context(|| format!("writing {} on {}", field.field.code(), field.graph_node_id))?;
    }
    Ok(plan.fields.len())
}

/// Replace each scenario's talking points with the planned ones.
///
/// ## Why this goes through the SERVICE and not the writers
///
/// `scenario_human_facts_tests::the_talking_point_writers_have_one_caller_family`
/// is a v2 §8 invariant: only the augmentation service may write C5. It caught
/// this loader on its first run, and it was RIGHT to — the service is the one
/// place that knows a replace is "delete the response row, re-insert every
/// item", and a loader that spelled that out itself would be a second definition
/// of what a talking-point list IS, free to drift from the human's.
///
/// Going through it also means the machine's three points and a human's edit
/// produce the same shape, and both are checked against the same stored cap.
///
/// ## Why REPLACE and not append
///
/// Appending would leave the machine's points BELOW whatever was there, and every
/// card's `backs_position` would then name the wrong sentence. The service
/// replaces, which is what the human path does.
pub async fn apply_points(
    pool: &sqlx::PgPool,
    planned: &[PlannedPoints],
    settings: &colossus_legal_backend::domain::settings::Settings,
) -> Result<usize> {
    use colossus_legal_backend::services::scenario_augmentation::set_talking_points;

    let mut written = 0usize;
    for scenario in planned {
        // The service takes the list in order and owns the indexes, so the
        // planned `(item_index, text)` pairs are handed over as text alone — the
        // plan's indexes are what the DRY RUN prints, and the service is what
        // decides them for real. Sorted here so the two agree.
        let mut ordered = scenario.items.clone();
        ordered.sort_by_key(|(index, _)| *index);
        let texts: Vec<String> = ordered.into_iter().map(|(_, text)| text).collect();

        let kept = set_talking_points(pool, scenario.scenario_id, &texts, LOADER_AUTHOR, settings)
            .await
            .with_context(|| format!("writing {}'s talking points", scenario.scenario_code))?;

        if kept.len() != texts.len() {
            // The stored cap silently drops the tail, and a card naming a point
            // past it would render nothing. Better to say so than to leave the
            // operator counting rows afterwards.
            bail!(
                "{} planned {} talking points and the stored cap kept {} — raise \
                 talking_points_cap or shorten the file",
                scenario.scenario_code,
                texts.len(),
                kept.len()
            );
        }
        written += kept.len();
    }
    Ok(written)
}

/// Include each pick as a fact, in its display order, with its drafted title.
///
/// Two writes per pick, in one transaction per scenario: the reference row that
/// puts the fact IN the scenario at its ordinal, and the card title that Job D
/// drafted for it.
///
/// ## Domain note: the picks are INCLUDED, not proposed
///
/// §1 says "the 10 picks become included facts in display order … Roman strikes".
/// So the status is `Included` and the ordinal is the pick's own — the human's
/// job is to remove what is wrong, not to accept what is right.
pub async fn apply_picks(pool: &sqlx::PgPool, planned: &[PlannedPicks]) -> Result<usize> {
    use colossus_legal_backend::repositories::pipeline_repository::scenario_fact_curation::set_fact_sort_ordinal;
    use colossus_legal_backend::repositories::pipeline_repository::scenario_store::upsert_fact_ref;

    let now = chrono::Utc::now();
    let mut written = 0usize;
    for scenario in planned {
        let mut tx = pool.begin().await.context("opening a transaction")?;
        for (node, ordinal, _title) in &scenario.picks {
            upsert_fact_ref(
                &mut *tx,
                scenario.scenario_id,
                node,
                None,
                FactStatus::Included,
                None,
                None,
                None,
                None,
            )
            .await
            .with_context(|| format!("including {node}"))?;
            set_fact_sort_ordinal(
                &mut *tx,
                scenario.scenario_id,
                node,
                *ordinal,
                LOADER_AUTHOR,
            )
            .await
            .with_context(|| format!("placing {node}"))?;
            written += 1;
        }
        tx.commit().await.context("committing the included facts")?;

        // The titles go through the card writer, OUTSIDE that transaction,
        // because it opens its own — see `write_pick_titles`.
        write_pick_titles(pool, scenario, now).await?;
    }
    Ok(written)
}

/// Write the drafted title onto each of one scenario's picks.
///
/// Split out of [`apply_picks`] for the function-size limit (Rule 18).
///
/// ## Why the titles are NOT in the caller's transaction
///
/// `write_field` opens its OWN transaction, because the card column and its
/// ledger row must land together — that pairing is `write_field`'s guarantee, and
/// borrowing the caller's transaction would take it away. So the placement commits
/// first and the titles follow. The failure mode that leaves is a placed fact whose
/// card has no title yet, which renders as a card awaiting a title — visible, and
/// fixed by re-running. The reverse order would leave a title on a card that is in
/// no scenario, which renders nowhere at all.
async fn write_pick_titles(
    pool: &sqlx::PgPool,
    scenario: &PlannedPicks,
    now: chrono::DateTime<chrono::Utc>,
) -> Result<()> {
    use colossus_legal_backend::domain::fact_card::CardField;

    for (node, _, title) in &scenario.picks {
        write_field(
            pool,
            &FieldWrite {
                scenario_id: scenario.scenario_id,
                graph_node_id: node,
                field: CardField::Title,
                value: Some(title),
                author: LOADER_AUTHOR,
                written_at: now,
            },
        )
        .await
        .with_context(|| format!("writing the title for {node}"))?;
    }
    Ok(())
}
