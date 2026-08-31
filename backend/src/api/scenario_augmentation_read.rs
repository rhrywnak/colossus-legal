//! Reading the augmentation panel (task 1.4; split out in task 2.11 C).
//!
//! `GET /cases/:slug/scenarios/:scenario_id/augmentation` → the whole panel in
//! one read: C1 identity, C4 human facts, the watch-list, and C5 talking points.
//!
//! Split from `api::scenario_augmentation` for Rule 17, exactly the way
//! `scenario_accusation_read` was split from its handlers, and for the same
//! reason: ruling C4b added two write handlers next door and the module went past
//! the limit. The seam is the one that was always there — the writes and their
//! fence stay together, and the READ plus the display shaping lands here.
//!
//! ## What is composed on this side of the wire, and why
//!
//! The date label ("Around April 2009") is a claim about PRECISION, and the
//! authored tag is this content's only provenance. Both are case vocabulary, and
//! the language law puts every such string on this side — a browser assembling
//! either would be a browser composing prose about the record.
//!
//! ## CRITICAL — the pipeline pool
//!
//! Every table read here lives in `colossus_legal_v2`, so the reads use
//! `&state.pipeline_pool`, NOT `state.pg_pool`.

use axum::{
    extract::{Path, State},
    Json,
};

use crate::{
    auth::AuthUser,
    domain::human_authored::{authored_tag, DateType, HumanFactKind},
    domain::scenario_code::scenario_code,
    domain::wording_authoring::AuthoringWording,
    dto::scenario_augmentation::{
        AugmentationPanelDto, AuthoringWordingDto, HumanFactDto, ScenarioIdentityDto,
        TalkingPointDto,
    },
    dto::scenario_authoring_wording::identity_wording,
    error::AppError,
    repositories::pipeline_repository::{get_scenario, ScenarioHumanFactRecord, ScenarioRecord},
    services::scenario_augmentation::{human_facts, talking_points},
    state::AppState,
};

use super::scenario_augmentation::augmentation_error_to_app_error;
use super::scenario_facts::{ensure_scenario_in_case, parse_scenario_id};

/// Compose one human fact for display.
///
/// The date label and the authored tag are built HERE, not in the browser: the
/// qualifier ("Around …") is a claim about precision, and the tag is this
/// content's only provenance. Both are case vocabulary, and the language law puts
/// every such string on this side of the wire.
fn to_fact_dto(record: &ScenarioHumanFactRecord) -> HumanFactDto {
    let date_label = record.occurred_on.map(|date| {
        let rendered = date.to_string();
        match record.date_type.as_deref().map(DateType::try_from) {
            // A stored type this build cannot read renders as the bare date
            // rather than guessing a qualifier — understating precision is the
            // safe direction.
            Some(Ok(kind)) => kind.describe(&rendered),
            _ => rendered,
        }
    });

    HumanFactDto {
        id: record.id.to_string(),
        text: record.text.clone(),
        date_label,
        person_refs: record.person_refs.clone().unwrap_or_default(),
        // Always false before task B0: these are names a human typed, not
        // resolved entities, and the panel says so.
        person_refs_are_linked: false,
        authored_tag: authored_tag(&record.authored_by),
        // Equal stamps mean untouched since it was written (see the insert).
        edited: record.updated_at != record.created_at,
    }
}

/// Compose the C1 identity block.
fn to_identity_dto(record: &ScenarioRecord) -> ScenarioIdentityDto {
    ScenarioIdentityDto {
        code: scenario_code(record.code_ordinal),
        name: record.name.clone(),
        direction: record.direction.clone(),
        // Straight off the record — no derivation, because the control that
        // renders this also WRITES it, and a header showing a computed status
        // could disagree with what the toggle is about to send.
        status: record.status.clone(),
        theme_statement: record.theme_statement.clone(),
        motivation: record.motivation.clone(),
        // Read out of the opaque definition body. A definition that is `{}` or a
        // retired shape simply yields `None` — the panel shows no attack line
        // rather than failing, because C1 editing must work on a half-authored
        // scenario.
        attack_text: record
            .definition
            .get("attack_text")
            .and_then(|v| v.as_str())
            .map(str::to_string),
    }
}

/// `GET …/augmentation` — the whole panel in one read.
///
/// Open read (`Option<AuthUser>`), matching the sibling scenario reads.
#[tracing::instrument(skip(state, user), fields(slug = %slug, scenario_id = %scenario_id))]
pub async fn get_augmentation_panel(
    user: Option<AuthUser>,
    State(state): State<AppState>,
    Path((slug, scenario_id)): Path<(String, String)>,
) -> Result<Json<AugmentationPanelDto>, AppError> {
    if let Some(ref u) = user {
        tracing::info!("{} GET augmentation panel for {}", u.username, scenario_id);
    }

    let id = parse_scenario_id(&scenario_id)?;
    ensure_scenario_in_case(&state, id, &slug).await?;

    let record = get_scenario(&state.pipeline_pool, id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, scenario_id = %id, "failed to read scenario for panel");
            AppError::Internal {
                message: "failed to load the scenario".to_string(),
            }
        })?
        .ok_or_else(|| AppError::NotFound {
            message: "scenario not found".to_string(),
        })?;

    let facts = human_facts(&state.pipeline_pool, id)
        .await
        .map_err(augmentation_error_to_app_error)?;

    let points = talking_points(&state.pipeline_pool, id)
        .await
        .map_err(augmentation_error_to_app_error)?;

    // One snapshot for the whole panel: the cap and the words it is spoken with
    // must come from the same read, or a change between two reads would show a
    // limit the sentence beside it contradicts.
    let settings = state.settings.current();

    Ok(Json(AugmentationPanelDto {
        identity: to_identity_dto(&record),
        human_facts: facts
            .iter()
            .filter(|f| f.kind == HumanFactKind::Fact.code())
            .map(to_fact_dto)
            .collect(),
        watch_list: facts
            .iter()
            .filter(|f| f.kind == HumanFactKind::WatchList.code())
            .map(to_fact_dto)
            .collect(),
        talking_points: points
            .into_iter()
            .map(|item| TalkingPointDto {
                text: item.text,
                // 1-based, matching the pill the human reads and the route the
                // editor calls. See `TalkingPointDto::position`.
                position: usize::try_from(item.item_index).unwrap_or(0) + 1,
                authored_tag: item.authored_by.as_deref().map(authored_tag),
            })
            .collect(),
        // From the store, not the browser and not a constant: it is a tunable,
        // and a client that baked in "3" would show the wrong limit the moment
        // Roman changes it on the Settings page (v2 §2b).
        talking_points_cap: settings.talking_points_cap,
        wording: authoring_wording(&settings.authoring_wording),
        identity_wording: identity_wording(&settings.scenario_authoring_wording),
    }))
}

#[cfg(test)]
#[path = "scenario_augmentation_read_tests.rs"]
mod tests;

/// The two authoring sections' words, for the wire.
///
/// A field-for-field copy rather than serializing the domain struct directly:
/// the domain type is this build's shape and the DTO is the CONTRACT, and
/// collapsing them would make every future field addition a silent API change.
/// The same separation `RehearsalWordingDto` keeps.
fn authoring_wording(w: &AuthoringWording) -> AuthoringWordingDto {
    AuthoringWordingDto {
        points_section_heading: w.points_section_heading.clone(),
        points_section_meta_template: w.points_section_meta_template.clone(),
        points_empty_notice: w.points_empty_notice.clone(),
        points_no_exhibit_notice: w.points_no_exhibit_notice.clone(),
        points_add_label: w.points_add_label.clone(),
        points_edit_label: w.points_edit_label.clone(),
        points_save_label: w.points_save_label.clone(),
        points_saving_label: w.points_saving_label.clone(),
        points_cancel_label: w.points_cancel_label.clone(),
        points_cap_reached_notice: w.points_cap_reached_notice.clone(),
        points_field_label_template: w.points_field_label_template.clone(),
        points_authoring_note: w.points_authoring_note.clone(),
        points_save_failed_notice: w.points_save_failed_notice.clone(),
        watch_section_heading: w.watch_section_heading.clone(),
        watch_section_meta: w.watch_section_meta.clone(),
        watch_field_label: w.watch_field_label.clone(),
        watch_add_label: w.watch_add_label.clone(),
        watch_save_label: w.watch_save_label.clone(),
        watch_edit_label: w.watch_edit_label.clone(),
        watch_cancel_label: w.watch_cancel_label.clone(),
        watch_remove_label: w.watch_remove_label.clone(),
        watch_edited_suffix: w.watch_edited_suffix.clone(),
        watch_save_failed_notice: w.watch_save_failed_notice.clone(),
    }
}
