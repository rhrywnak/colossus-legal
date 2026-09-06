//! The fact-card half of the card payload's assembly (FACT_CARD_v2 §2).
//!
//! Three reads and one composition, split out of `api::scenario_cards` because
//! that handler was already at the 300-line module limit before this task and
//! because the seam is real: everything here is about the five sentences a HUMAN
//! wrote for a scenario, while the handler is about the pool the graph returns.
//!
//! ## Why these are here and not in `services`
//!
//! All three perform I/O. `services::fact_card_render` is the pure half — it
//! composes a stored row into finished lines and touches no database — and this
//! module is what feeds it. The same split `scenario_card_assembly` documents:
//! pure shaping in services, reads beside the handler.

use std::collections::HashMap;

use uuid::Uuid;

use crate::domain::settings::Settings;
use crate::dto::scenario_card::ScenarioCardsResponse;
use crate::error::AppError;
use crate::repositories::allegation_options_repository::fetch_allegation_options;
use crate::repositories::pipeline_repository::scenario_fact_cards::{
    list_cards_for_scenario, CardRecord,
};
use crate::repositories::pipeline_repository::{
    list_items_for_response, sole_response_for_scenario,
};
use crate::services::fact_card_render::{attach_fact_cards, AllegationLabel, RenderContext};
use crate::state::AppState;

/// Read this scenario's stored cards and attach them to the response.
///
/// ## Why a failure here REFUSES rather than degrading
///
/// Almost every other read on this path degrades: a missing page text renders a
/// bare quote, a missing reason renders no reason. This one does not. If the
/// cards cannot be read, every card renders as though nobody had drafted it — a
/// deck of 59 prepared answers would look like a deck of none, and there is no
/// way to show that as a gap in a list whose absence IS the gap. The request
/// fails and the page says so.
///
/// The three reads it needs — the cards, the case's accusations, and the
/// scenario's talking points — are each ONE query for the whole deck.
pub(crate) async fn attach_scenario_fact_cards(
    state: &AppState,
    scenario_id: Uuid,
    subject_id: &str,
    settings: &Settings,
    response: &mut ScenarioCardsResponse,
) -> Result<(), AppError> {
    let cards = list_cards_for_scenario(&state.pipeline_pool, scenario_id)
        .await
        .map_err(|e| {
            tracing::error!(
                error = %e,
                %scenario_id,
                "could not read this scenario's fact cards — the deck is refused rather \
                 than rendered, because a deck with every card missing looks exactly \
                 like a deck nobody has drafted"
            );
            // Named by scenario, so the 500 body identifies which deck failed.
            AppError::Internal {
                message: format!(
                    "failed to read the fact cards of scenario {scenario_id} — the \
                     deck is refused rather than shown incomplete; reload to retry"
                ),
            }
        })?;
    if cards.is_empty() {
        return Ok(());
    }
    let by_node: HashMap<String, CardRecord> = cards
        .into_iter()
        .map(|c| (c.graph_node_id.clone(), c))
        .collect();

    let allegations = load_allegation_labels(state, subject_id).await?;
    let talking_points = load_talking_points(state, scenario_id).await?;

    attach_fact_cards(
        response,
        &by_node,
        RenderContext {
            allegations: &allegations,
            talking_points: &talking_points,
            wording: &settings.fact_card_wording,
        },
    );
    Ok(())
}

/// The case's accusations, indexed by graph id, in the shape a card needs.
///
/// Reuses `fetch_allegation_options` — the same catalogue the link panel reads —
/// rather than a second query, so a card and a link chip name one accusation
/// identically. The `count_tag` is composed HERE, once, from the count's number
/// and title: the browser must not join them (the language law).
async fn load_allegation_labels(
    state: &AppState,
    subject_id: &str,
) -> Result<HashMap<String, AllegationLabel>, AppError> {
    let options = fetch_allegation_options(&state.graph, subject_id)
        .await
        .map_err(|e| {
            tracing::error!(
                error = %e,
                %subject_id,
                "failed to read the accusation catalogue needed to label fact cards"
            );
            // Named by subject, like its sibling above: the 500 body should say
            // which read failed without an operator first correlating a
            // timestamp against the log.
            AppError::Internal {
                message: format!(
                    "failed to read the accusations for subject {subject_id} — the \
                     count tags cannot be composed; reload the scenario to retry"
                ),
            }
        })?;

    Ok(options
        .into_iter()
        .map(|row| {
            let count_tag = match (row.count_number, row.count_name.as_deref()) {
                // Both halves or nothing: "Count 1" with no cause of action named
                // is not a tag a lawyer can use, and a bare title does not say
                // which count it is.
                (Some(number), Some(name)) => Some(format!("Count {number} — {name}")),
                _ => None,
            };
            (
                row.allegation_id,
                AllegationLabel {
                    paragraph: row.paragraph,
                    // The complaint's own sentence, falling back to the node's
                    // title. Same preference `accusation_label` makes one module
                    // over, so the two surfaces read alike.
                    text: row.summary.or(row.title),
                    count_tag,
                },
            )
        })
        .collect())
}

/// The scenario's talking points, by their 1-based position.
///
/// ## Why a read failure here is a WARNING and not a refusal
///
/// Unlike the cards, this one degrades honestly: a card whose Backs row cannot be
/// composed renders the stored em dash, which is the same thing it renders when
/// the card backs no point. Nothing is claimed falsely, and four other rows are
/// unaffected — so failing the whole deck to protect one line would be the wrong
/// trade. The absence stays observable in the log.
///
/// `item_index` is 0-based in storage and the card's `backs_position` is 1-based
/// — the number PRINTED beside the point. The `+ 1` is that translation, in the
/// one place that needs it.
async fn load_talking_points(
    state: &AppState,
    scenario_id: Uuid,
) -> Result<HashMap<i32, String>, AppError> {
    let Some(response) = sole_response_for_scenario(&state.pipeline_pool, scenario_id)
        .await
        .unwrap_or_else(|e| {
            tracing::warn!(
                error = %e,
                %scenario_id,
                "could not read this scenario's talking points; every card's Backs row \
                 renders as the stored em dash. The rest of each card is unaffected"
            );
            None
        })
    else {
        return Ok(HashMap::new());
    };

    let items = list_items_for_response(&state.pipeline_pool, response.id)
        .await
        .unwrap_or_else(|e| {
            tracing::warn!(
                error = %e,
                %scenario_id,
                "could not read the talking-point items; every card's Backs row renders \
                 as the stored em dash"
            );
            Vec::new()
        });

    Ok(items
        .into_iter()
        .map(|item| (item.item_index + 1, item.text))
        .collect())
}
