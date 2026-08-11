//! Gathering what one ready scenario's rehearsal render needs (task 2.11 B2).
//!
//! The I/O half. Everything it reads goes straight into
//! `services::rehearsal_render`, which is pure — so the composition is testable
//! without a database and this module has nothing in it but reads and their
//! failure handling.
//!
//! Split from `scenario_readiness` for Rule 17 and for a clearer seam: that
//! module owns the READY GATE, which is a rule about who may see a scenario at
//! all; this one owns what a scenario that passed the gate is made of.
//!
//! ## CRITICAL — the pipeline pool, and one graph read
//!
//! `scenarios`, `scenario_fact_refs`, `scenario_human_facts`, `scenario_responses`
//! and `response_items` all live in `colossus_legal_v2`: `&state.pipeline_pool`.
//! The statements themselves come from Neo4j, and ONLY the placed ones — see
//! `fetch_rehearsal_facts` for why that query projects so little.

use std::collections::HashMap;

use sqlx::PgPool;

use crate::domain::human_authored::HumanFactKind;
use crate::domain::scenario_code::scenario_code;
use crate::domain::settings::Settings;
use crate::dto::rehearsal::{RehearsalPoint, RehearsalScenario, RehearsalWatchItem};
use crate::repositories::pipeline_repository::{
    list_candidate_ordinals, list_fact_refs_for_scenario, list_human_facts_for_scenario,
    list_items_for_response, sole_response_for_scenario, PipelineRepoError, ScenarioRecord,
};
use crate::repositories::scenario_accusation_repository::{
    fetch_anchor_paragraphs, fetch_rehearsal_facts, RehearsalFactRow,
};
use crate::services::rehearsal_render::{render_scenario, Authored, ScenarioInput};
use crate::services::scenario_accusation::{derive, StoredJudgment};

/// Why a rehearsal scenario could not be assembled.
#[derive(Debug, thiserror::Error)]
pub enum AssemblyError {
    #[error("failed to read the scenario's rehearsal content: {source}")]
    Read {
        #[source]
        source: PipelineRepoError,
    },

    /// The RECORD store could not be reached.
    ///
    /// Its own variant, and not folded into `Read`: a Postgres failure means the
    /// human's judgments are unavailable, while this means the statements those
    /// judgments point AT are. Reporting either as the other sends an operator to
    /// the wrong service — and this one must never degrade to an empty page,
    /// which would read as "nothing has been marked".
    #[error("failed to read the statements from the record store: {source}")]
    Record {
        #[source]
        source: crate::repositories::scenario_card_repository::ScenarioCardRepoError,
    },

    /// A stored row carries a token this build cannot classify.
    #[error("{detail}")]
    Undecodable { detail: String },
}

/// Assemble one ready scenario for the rehearsal page.
///
/// # Errors
/// Returns [`AssemblyError`] if any read fails or a stored token is unreadable.
pub async fn assemble_scenario(
    pool: &PgPool,
    graph: &neo4rs::Graph,
    record: &ScenarioRecord,
    settings: &Settings,
) -> Result<RehearsalScenario, AssemblyError> {
    let scenario_id = record.scenario_id;

    let notes = list_human_facts_for_scenario(pool, scenario_id)
        .await
        .map_err(|source| AssemblyError::Read { source })?;
    let judgments = judgments_in(&notes, scenario_id)?;

    let included = included_ids(pool, scenario_id).await?;
    let state = derive(&judgments, &included);
    let facts = placed_facts(graph, &state).await?;
    let points = talking_points_of(pool, scenario_id, settings).await?;
    // The candidate ordinals behind the C-codes the pair card prints (task R4,
    // P3). Read here rather than passed in, because this function is where every
    // other per-scenario read for this page already lives — and the working
    // page's accusation route reads the same table the same way.
    let ordinals = list_candidate_ordinals(pool, scenario_id)
        .await
        .map_err(|source| AssemblyError::Read { source })?;
    // The paragraph behind each anchor id, for the bears-on chips (task R4,
    // P6a). Scoped to the handful of ids this scenario anchors — never the whole
    // complaint.
    let anchors = record.anchor_allegation_ids.clone().unwrap_or_default();
    let paragraphs = fetch_anchor_paragraphs(graph, &anchors)
        .await
        .map_err(|source| AssemblyError::Record { source })?;

    Ok(render_scenario(ScenarioInput {
        code: scenario_code(record.code_ordinal),
        // The address the page's editors write to (ruling C1). It renders
        // nowhere; `code` is the only identifier a reader ever sees.
        scenario_id: scenario_id.to_string(),
        title: &record.name,
        what_this_is: record.theme_statement.as_deref(),
        what_authored: Authored {
            by: record.theme_authored_by.as_deref(),
            at: record.theme_authored_at,
        },
        // The AUTHORED accusation, never `definition->>'attack_text'` — that
        // column holds a verbatim first-person quote from the record, and
        // rendering it here promoted one piece of evidence into the summary of
        // all of it. That was the defect this whole task exists to end, and this
        // line is where it stays ended.
        accusation_text: record.accusation_text.as_deref(),
        accusation_authored: Authored {
            by: record.accusation_text_authored_by.as_deref(),
            at: record.accusation_text_authored_at,
        },
        state: &state,
        facts: &facts,
        points,
        watch_for: watch_items(&notes),
        // The prep page's identity line, its foldable full attack, and its
        // bears-on chips (task R3). All three come off the record the assembly
        // already loaded — no second read.
        direction_label: direction_label(&record.direction, settings),
        attack_text: attack_text_of(&record.definition),
        bears_on: bears_on_codes(&anchors, &paragraphs),
        ordinals: &ordinals,
        settings,
    }))
}

/// The watch-list notes, each with the address the edit route needs.
///
/// Filtered on `kind` rather than on anything about the text: a watch item and a
/// human fact live in one table, and this block shows only the first. Each row
/// carries its own id because this page can now EDIT one (ruling C4b), and
/// `PUT …/human-facts/:fact_id` has to know which — see the DTO's module header
/// for why an address is not §10-excluded content.
fn watch_items(
    notes: &[crate::repositories::pipeline_repository::ScenarioHumanFactRecord],
) -> Vec<RehearsalWatchItem> {
    notes
        .iter()
        .filter(|n| n.kind == HumanFactKind::WatchList.code())
        .map(|n| RehearsalWatchItem {
            id: n.id.to_string(),
            text: n.text.clone(),
        })
        .collect()
}

// CONST: mirrors the `scenario_fact_refs.status` vocabulary — a schema-coupling
// invariant, not a deployment knob.
const INCLUDED_STATUS: &str = "included";

/// The graph node ids currently INCLUDED in the scenario.
///
/// Compared as the stored TOKEN rather than decoded into `FactStatus`: this path
/// only asks "is it in?", and a status this build cannot read is not in — which is
/// the safe direction here, because a judgment pointing at it then surfaces as a
/// NAMED gap rather than as content nobody vetted.
async fn included_ids(
    pool: &PgPool,
    scenario_id: uuid::Uuid,
) -> Result<std::collections::HashSet<String>, AssemblyError> {
    Ok(list_fact_refs_for_scenario(pool, scenario_id)
        .await
        .map_err(|source| AssemblyError::Read { source })?
        .iter()
        .filter(|r| r.status == INCLUDED_STATUS)
        .map(|r| r.graph_node_id.clone())
        .collect())
}

/// The statements a human PLACED, by graph node id.
///
/// Only the marked instances and the items paired as our answers. The included
/// pool is deliberately not read: this page renders nothing nobody put on it, and
/// a query that fetched more would be loading data the page may not show.
async fn placed_facts(
    graph: &neo4rs::Graph,
    state: &crate::services::scenario_accusation::AccusationState,
) -> Result<HashMap<String, RehearsalFactRow>, AssemblyError> {
    let mut placed: Vec<String> = Vec::new();
    for instance in &state.instances {
        placed.push(instance.anchor_graph_node_id.clone());
        if let Some(answer) = &instance.answers_graph_node_id {
            placed.push(answer.clone());
        }
    }
    // Sorted and de-duplicated: one item can be the answer to two instances, and
    // asking the record store for it twice would be two rows for one statement.
    placed.sort_unstable();
    placed.dedup();

    Ok(fetch_rehearsal_facts(graph, &placed)
        .await
        .map_err(|source| AssemblyError::Record { source })?
        .into_iter()
        .map(|row| (row.graph_node_id.clone(), row))
        .collect())
}

/// The stored ANCHORED judgments, in the narrow shape the derivation needs.
///
/// Same three cases and the same reasoning as
/// `api::scenario_accusation_read::judgments_in`: a row with no anchor is prose
/// belonging to another block and is skipped; a row WITH an anchor this build
/// cannot classify is somebody's judgment about a specific statement and refuses,
/// because rendering the page without it would drop a human's decision silently.
fn judgments_in(
    notes: &[crate::repositories::pipeline_repository::ScenarioHumanFactRecord],
    scenario_id: uuid::Uuid,
) -> Result<Vec<StoredJudgment>, AssemblyError> {
    let mut out = Vec::new();
    for note in notes {
        let Some(anchor) = note.anchor_graph_node_id.as_deref() else {
            continue;
        };

        let kind = HumanFactKind::try_from(note.kind.as_str()).map_err(|e| {
            tracing::error!(
                error = %e,
                fact_id = %note.id,
                %scenario_id,
                anchor,
                "an anchored row carries a kind token this build does not define"
            );
            AssemblyError::Undecodable {
                detail: e.to_string(),
            }
        })?;

        if !kind.is_anchored() {
            tracing::error!(
                fact_id = %note.id,
                %scenario_id,
                anchor,
                kind = kind.code(),
                "a prose kind carries an anchor; the row's shape and its kind disagree"
            );
            return Err(AssemblyError::Undecodable {
                detail: format!("a {} row carries an anchor", kind.code()),
            });
        }

        out.push(StoredJudgment {
            kind,
            anchor_graph_node_id: anchor.to_string(),
            answers_graph_node_id: note.answers_graph_node_id.clone(),
        });
    }
    Ok(out)
}

/// One scenario's talking points, ordered and capped.
///
/// ## Why `scenario_responses.status` is NOT consulted — read before changing
///
/// Ruled 2026-08-01 and carried forward verbatim from the code this replaces: the
/// SCENARIO's readiness is the only gate. Every 1.4 write path sets that column to
/// `'draft'`, so filtering on it would show an empty block forever with no error —
/// the silent-empty failure this codebase keeps removing.
async fn talking_points_of(
    pool: &PgPool,
    scenario_id: uuid::Uuid,
    settings: &Settings,
) -> Result<Vec<RehearsalPoint>, AssemblyError> {
    // The guarded read (task R1 Piece 6). This site had no multi-row warning at
    // all until .390, and it is the one that feeds a witness: a second response
    // row would have silently rehearsed the older row's points.
    let response = sole_response_for_scenario(pool, scenario_id)
        .await
        .map_err(|source| AssemblyError::Read { source })?;

    let Some(response) = response else {
        return Ok(Vec::new());
    };

    let items = list_items_for_response(pool, response.id)
        .await
        .map_err(|source| AssemblyError::Read { source })?;

    Ok(items
        .into_iter()
        // The cap is a display law as well as a write law: a list that grew past
        // it — through a direct write, or a lowered cap — must still rehearse as
        // the few points a witness can hold.
        .take(settings.talking_points_cap)
        .map(|item| RehearsalPoint {
            // The STORED index, not the position in this iteration: the two agree
            // today, and would stop agreeing the moment the cap trimmed from the
            // front or a row went missing. Editing point 3 must address the row
            // the store calls 3, or the edit lands on the wrong sentence.
            //
            // ## Rust Learning: `usize::try_from` on an `i32`
            //
            // `item_index` is `i32` because Postgres `int` is signed. A negative
            // value cannot exist (the writer only ever inserts from `enumerate`),
            // but the compiler does not know that, so the conversion is fallible.
            // Falling back to 0 rather than panicking keeps a corrupted row from
            // taking down the page — and 0 is a position no route matches, so the
            // edit is refused rather than mis-applied.
            position: usize::try_from(item.item_index).unwrap_or(0) + 1,
            text: item.text,
            // The pairing editor is tracker task 3.9 and does not exist; measured
            // on DEV, `response_item_fact_refs` holds zero rows. Deriving a label
            // from the record instead would put words in the witness's mouth.
            exhibit: None,
            exhibit_notice: settings
                .rehearsal_chrome_wording
                .point_no_exhibit_notice
                .clone(),
        })
        .collect())
}

#[cfg(test)]
#[path = "rehearsal_assembly_tests.rs"]
mod tests;

// STRUCTURAL: the `scenarios.direction` CHECK vocabulary. Schema-coupling, not a
// deployment knob — changing either needs a migration, and the pair is repeated
// nowhere else in this module.
const DIRECTION_OFFENSE: &str = "offense";
const DIRECTION_DEFENSE: &str = "defense";

/// Offense or defense, in the stored word rather than the token.
///
/// The token is a schema value (`"offense"` / `"defense"`); the word is what a
/// reader sees. Translating it in the browser would put this vocabulary in two
/// places, which is the defect .391 spent a migration closing one layer up.
fn direction_label(direction: &str, settings: &Settings) -> String {
    let w = &settings.rehearsal_wording;
    match direction {
        DIRECTION_OFFENSE => w.direction_offense_label.clone(),
        DIRECTION_DEFENSE => w.direction_defense_label.clone(),
        // A token the column's CHECK should make impossible. Falling back is
        // right — a witness's page must not fail over a vocabulary surprise —
        // but falling back SILENTLY would state the wrong side of the argument
        // on the surface where being on the wrong side matters most. So the
        // fallback names the scenario and the token it did not recognise.
        other => {
            tracing::warn!(
                direction = other,
                "unexpected scenario direction — rendering the defense wording; \
                 check scenarios.direction for this scenario"
            );
            w.direction_defense_label.clone()
        }
    }
}

/// The attack in their own words, out of the stored definition.
///
/// `None` for a blank one as well as a missing one — a definition holding `""`
/// and one holding nothing are the same thing to a reader, and the page renders
/// no fold control for either rather than an empty drawer.
fn attack_text_of(definition: &serde_json::Value) -> Option<String> {
    definition
        .get("attack_text")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

/// The complaint paragraphs this scenario bears on, as A-codes.
///
/// ## Domain note: why the code and not the paragraph number
/// The bears-on chips: `A-41`, the handle every other surface uses.
///
/// ## What was wrong before (task R4, P6a)
///
/// The old version derived the number from the id, by taking its last
/// `:`-segment. An anchor id is `doc-…:allegation:<hash>`, so that segment is
/// the HASH — every chip on the prep page read `A-<hash>`: a node-id fragment
/// wearing the prefix of a paragraph number, on the surface where a reader is
/// least equipped to notice. The paragraph is not in the id at all. It is a
/// property of the Allegation node, which is why this now takes a map that was
/// READ rather than a string it can pick apart.
///
/// ## Why an unknown id renders AS the id
///
/// Dropping it would make the chip row quietly shorter than the scenario's
/// actual anchors — the count would disagree with the data and nothing on screen
/// would say why. The id is a poor label but an HONEST one: it names something
/// that exists and can be looked up. Same rule, and same reason, as the working
/// page's `labelForAllegationId`.
fn bears_on_codes(anchors: &[String], paragraphs: &HashMap<String, String>) -> Vec<String> {
    anchors
        .iter()
        .map(|id| match paragraphs.get(id) {
            Some(paragraph) => crate::domain::scenario_code::allegation_code(paragraph),
            None => id.clone(),
        })
        .collect()
}

#[cfg(test)]
mod prep_helpers_tests {
    use super::*;
    use crate::domain::settings::Settings;

    #[test]
    fn the_direction_is_stated_in_words_not_in_its_token() {
        let s = Settings::for_test();
        assert_eq!(
            direction_label(DIRECTION_OFFENSE, &s),
            "We are pressing this"
        );
        assert_eq!(
            direction_label(DIRECTION_DEFENSE, &s),
            "We are answering this"
        );
    }

    /// An unexpected token renders the defense wording AND warns.
    ///
    /// The fallback itself is right — a witness's page must not fail over a
    /// vocabulary surprise — but it states the wrong side of the argument on the
    /// surface where being on the wrong side matters most, so it cannot be
    /// silent. The warn is asserted by the architecture gate's reading rather
    /// than here; this pins that the fallback does not panic and does not invent
    /// a third label.
    #[test]
    fn an_unexpected_direction_falls_back_rather_than_panicking() {
        assert_eq!(
            direction_label("sideways", &Settings::for_test()),
            "We are answering this"
        );
    }

    #[test]
    fn the_verbatim_attack_is_absent_when_nobody_wrote_one() {
        assert_eq!(attack_text_of(&serde_json::json!({})), None);
        assert_eq!(
            attack_text_of(&serde_json::json!({ "attack_text": "" })),
            None,
            "a blank one is the same as none — the page renders no fold control \
             for either rather than an empty drawer"
        );
        assert_eq!(
            attack_text_of(&serde_json::json!({ "attack_text": "   " })),
            None
        );
    }

    #[test]
    fn the_verbatim_attack_is_trimmed_when_present() {
        assert_eq!(
            attack_text_of(&serde_json::json!({ "attack_text": "  they did not cooperate  " })),
            Some("they did not cooperate".to_string())
        );
    }

    /// The chips read the PARAGRAPH the graph holds, never the id's tail.
    ///
    /// ## Why the test this replaces was green over the bug for months
    ///
    /// It asserted `bears_on_codes(["41", "46"]) == ["A-41", "A-46"]` — passing
    /// BARE paragraph numbers as anchor ids. A real anchor id is
    /// `doc-…:allegation:<hash>`, and the old function took the last
    /// `:`-segment: for that fixture the number itself, for real data the hash.
    /// The test exercised a shape the database never produces, so it could not
    /// fail while the page rendered `A-<hash>`.
    ///
    /// This one uses the real shape, and the paragraph arrives from the map the
    /// assembly READ rather than from any string the id can be cut into.
    #[test]
    fn the_chips_read_the_paragraph_and_not_the_ids_tail() {
        let anchors = vec![
            "doc-complaint:allegation:9f2c1a".to_string(),
            "doc-complaint:allegation:7b4e02".to_string(),
        ];
        let paragraphs: HashMap<String, String> = [
            (
                "doc-complaint:allegation:9f2c1a".to_string(),
                "41".to_string(),
            ),
            (
                "doc-complaint:allegation:7b4e02".to_string(),
                "92".to_string(),
            ),
        ]
        .into_iter()
        .collect();

        assert_eq!(bears_on_codes(&anchors, &paragraphs), vec!["A-41", "A-92"]);

        // The regression, stated as its own assertion: no chip may carry a
        // fragment of the id it came from.
        for chip in bears_on_codes(&anchors, &paragraphs) {
            assert!(!chip.contains("9f2c1a"), "{chip}");
            assert!(!chip.contains("7b4e02"), "{chip}");
        }
    }

    /// A paragraph the graph cannot supply renders AS the id, never dropped.
    ///
    /// Dropping it would make the chip row quietly shorter than the scenario's
    /// actual anchors: the count would disagree with the data and nothing on
    /// screen would say why.
    #[test]
    fn an_unresolvable_anchor_keeps_its_place_in_the_row() {
        let anchors = vec![
            "doc-complaint:allegation:9f2c1a".to_string(),
            "doc-complaint:allegation:missing".to_string(),
        ];
        let paragraphs: HashMap<String, String> = [(
            "doc-complaint:allegation:9f2c1a".to_string(),
            "41".to_string(),
        )]
        .into_iter()
        .collect();

        let chips = bears_on_codes(&anchors, &paragraphs);
        assert_eq!(chips.len(), 2, "a chip was dropped: {chips:?}");
        assert_eq!(chips[0], "A-41");
        assert_eq!(chips[1], "doc-complaint:allegation:missing");
    }

    #[test]
    fn no_anchors_is_an_empty_chip_row_rather_than_a_guess() {
        assert!(bears_on_codes(&[], &HashMap::new()).is_empty());
    }
}
