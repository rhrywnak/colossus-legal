//! Gathering one read's payload from the store — and deciding what an absence MEANS.
//!
//! The impure half of [`super::practice_read_payload`]. Split out of
//! `api::practice_answers` in T1, for the reason `api::practice_fences` was split
//! out of it in August: the handler is the sequence of acts that record an
//! answer, and this is the question of what the model is told — which grew from
//! four fields to nine and took the file past the 300-line limit with it.
//!
//! ## THE RULE THIS MODULE EXISTS TO ENFORCE
//!
//! A missing value means one of two completely different things, and telling them
//! apart is the difference between a working feature and one that abstains
//! forever:
//!
//! | Absence | Means | This module |
//! |---|---|---|
//! | `tactic IS NULL` | every Chuck question — a direct or redirect carries none | named absence, the read PROCEEDS |
//! | no sworn pair | every Chuck question — **[measured: 20 of the 30 live rows]** | named absence, the read PROCEEDS |
//! | no receipt for a point | nobody has paired or seeded one; design §7.3 expects it | named absence, the read PROCEEDS |
//! | no points authored | a scenario whose talking points are unwritten | named absence, the read PROCEEDS |
//! | `list_points` FAILED | the database did not answer | **ABSTAIN** |
//! | `list_point_receipts` FAILED | the database did not answer | **ABSTAIN** |
//! | tactic set, vocabulary too short to name it | a settings row somebody trimmed | **ABSTAIN** |
//!
//! Roman ruled this table on 2026-08-20 against measured deck data. Getting the
//! top four wrong makes two thirds of the deck — every question Chuck asks —
//! abstain on every answer, permanently. Getting the bottom three wrong is the
//! blind-read defect: a read composed against material that silently failed to
//! load, stored looking exactly like a good one.
//!
//! ## Why the failures ABSTAIN rather than degrade
//!
//! Before T1, a failed `list_points` logged an error and carried on with an empty
//! vector. The model then judged her answer against a question and a watch-for
//! alone and returned a sentence that looked like every other sentence — green
//! rail, `read_error` NULL, nothing on the row recording that the read was made
//! blind. Two operationally distinct states, one observable, which is Standing
//! Rule 1 exactly.

use uuid::Uuid;

use crate::domain::settings::Settings;
use crate::repositories::pipeline_repository::practice::{
    list_point_receipts, list_points, PracticeQuestionRecord,
};
use crate::services::practice_page::{point_receipt, tactic_name};
use crate::services::practice_read_payload::{Keyed, PointsTo, ReadPayload, Tactic};
use crate::state::AppState;

/// Why no payload could honestly be built.
///
/// ## Rust Learning: a typed error whose `Display` is the OPERATOR's sentence
///
/// Each variant carries what a log reader needs to act — which scenario, which
/// card number, which underlying cause. Marie never sees these: she reads the
/// stored abstain line, and this text goes to `read_error` and
/// `read_abstain_reason`. That split is the Rule 1 shape the read path has kept
/// since v0 — one fixed observable on the screen, a specific one in the log.
#[derive(Debug, thiserror::Error)]
pub enum PayloadFailure {
    #[error("her points could not be read for scenario {scenario_id}: {source}")]
    Points {
        scenario_id: Uuid,
        #[source]
        source: anyhow::Error,
    },

    #[error(
        "the receipts behind her points could not be read for scenario {scenario_id}: {source}"
    )]
    Receipts {
        scenario_id: Uuid,
        #[source]
        source: anyhow::Error,
    },

    #[error(
        "this question carries tactic card {card}, which the stored vocabulary \
         (practice_tactic_names) has no name for — the row is shorter than the deck's \
         numbering; restore the seven comma-separated card names to practice_tactic_names"
    )]
    TacticUnnamed { card: i16 },
}

impl PayloadFailure {
    /// The plain-English half, for `read_abstain_reason`.
    ///
    /// Deliberately not the `Display` text: that one names a scenario id and a
    /// settings key because an operator needs them, and this one is the sentence
    /// T4 will render on the amber block. Code-owned rather than a settings row,
    /// for the reason the skip marker gives — a value composed FROM AN OBSERVED
    /// FAILURE must not be editable after the fact by the person investigating it.
    pub fn plain_reason(&self) -> &'static str {
        match self {
            PayloadFailure::Points { .. } => "her talking points could not be loaded",
            PayloadFailure::Receipts { .. } => "the receipts behind her points could not be loaded",
            PayloadFailure::TacticUnnamed { .. } => {
                "this question's tactic card has no name in the stored vocabulary"
            }
        }
    }
}

/// What she said she would point to, as the payload's three-way fact.
///
/// `None` and `Some([])` are DIFFERENT and stay different — see [`PointsTo`].
fn points_to_of(picked: Option<&Vec<String>>) -> PointsTo {
    match picked {
        None => PointsTo::NeverOpened,
        Some(list) if list.is_empty() => PointsTo::OpenedAndPickedNothing,
        Some(list) => PointsTo::Picked(list.clone()),
    }
}

/// The tactic, or the reason there is no honest way to send one.
fn tactic_of(settings: &Settings, card: Option<i16>) -> Result<Tactic, PayloadFailure> {
    let Some(card) = card else {
        // The column's own comment: NULL and "none" are not the same thing, and
        // only one of them is representable. A Chuck question HAS no tactic; it
        // does not have a tactic called none.
        return Ok(Tactic::NoneByDesign);
    };
    match tactic_name(settings, Some(card)) {
        Some(name) => Ok(Tactic::Named(name)),
        // The card number is real (the column's CHECK constrains it to 1–7) and
        // the vocabulary cannot name it. Sending "none" here would tell the model
        // a CROSS question carries no tactic — a false statement about the very
        // question it is judging, and the live defect T1 fixes.
        None => Err(PayloadFailure::TacticUnnamed { card }),
    }
}

/// Her points and their receipts, keyed `P1…Pn` and `R1…Rn`.
///
/// ## Domain note: the receipt precedence is the SCREEN's, deliberately reused
///
/// [`point_receipt`] states the order — a human's pairing first, the seeded
/// stand-in second — and the read calls that function rather than restating it.
/// A second copy of the precedence is how the model comes to cite a phrase Marie
/// is not looking at, and it would go wrong precisely when Roman's `exhibit`
/// backfill lands, since **[measured 2026-08-20]** every receipt on both live
/// scenarios comes from the seeded table today and nothing would diverge until
/// then.
async fn points_and_receipts(
    state: &AppState,
    scenario_id: Uuid,
) -> Result<(Vec<Keyed>, Vec<Keyed>), PayloadFailure> {
    let points = list_points(&state.pipeline_pool, scenario_id)
        .await
        .map_err(|e| PayloadFailure::Points {
            scenario_id,
            source: anyhow::Error::new(e),
        })?;
    let seeded = list_point_receipts(&state.pipeline_pool, scenario_id)
        .await
        .map_err(|e| PayloadFailure::Receipts {
            scenario_id,
            source: anyhow::Error::new(e),
        })?;

    let keyed_points = points
        .iter()
        .map(|p| Keyed::new(format!("P{}", p.position), Some(p.text.clone())))
        .collect();
    let keyed_receipts = points
        .iter()
        .map(|p| Keyed::new(format!("R{}", p.position), point_receipt(p, &seeded)))
        .collect();

    Ok((keyed_points, keyed_receipts))
}

/// Everything the model is told about one answer, or why it cannot be told.
///
/// ## Rust Learning: `Result` here, and a plain struct one layer up
///
/// [`super::practice_read::read_answer`] returns an outcome and never a `Result`,
/// because every arm of it maps to the same database row. This one DOES return a
/// `Result`, because its two arms are genuinely different acts: build a payload,
/// or do not call the model at all. Collapsing them would put the abstain
/// decision inside the caller as a field check, which is the shape that let the
/// blind read ship.
///
/// # Errors
/// [`PayloadFailure`] when an input the read is judged against could not be
/// loaded, or when a tactic the question really carries cannot be named.
pub async fn gather_payload(
    state: &AppState,
    scenario_id: Uuid,
    question: &PracticeQuestionRecord,
    answer_text: &str,
    points_to: Option<&Vec<String>>,
) -> Result<ReadPayload, PayloadFailure> {
    let settings = state.settings.current();
    let tactic = tactic_of(&settings, question.tactic)?;
    let (points, receipts) = points_and_receipts(state, scenario_id).await?;

    // Half a sworn pair is a DATA defect, not a load failure (Roman, A3): the
    // seed writes both or neither, no constraint enforces it, and an answer is
    // still judgeable against the half that exists. It is logged because nothing
    // else would ever notice — no screen renders a half pair as wrong.
    if question.pair_said.is_some() != question.pair_admitted.is_some() {
        tracing::warn!(
            question = %question.id,
            has_said = question.pair_said.is_some(),
            has_admitted = question.pair_admitted.is_some(),
            "practice read: half a sworn pair — the seed writes both or neither"
        );
    }

    let side = if question.side == "george" {
        settings.practice_wording.pill_george.clone()
    } else {
        settings.practice_wording.pill_chuck.clone()
    };

    Ok(ReadPayload {
        question: question.text.clone(),
        side,
        kind: question.kind.clone(),
        tactic,
        answer: answer_text.to_string(),
        points,
        receipts,
        said: question.pair_said.clone(),
        admitted: question.pair_admitted.clone(),
        points_to: points_to_of(points_to),
        watch_for: question.watch_for.clone(),
        always: settings.practice_wording.always_line.clone(),
    })
}

#[cfg(test)]
#[path = "practice_read_gather_tests.rs"]
mod tests;
