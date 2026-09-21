//! Collecting the question chat's package from the stores (the I/O half of the
//! package; [`crate::services::chat_question_context`] renders it).
//!
//! Every read either succeeds or fails the request by name — with ONE exception,
//! stated: resolving which document the question was BUILT FROM goes to the
//! graph, and if the graph cannot answer, the package is still complete (every
//! document is in it); only the "primary first" ordering is lost. That is logged
//! as a warning naming the question, never swallowed (Standing Rule 1).

use neo4rs::query;
use uuid::Uuid;

use crate::domain::scenario_code::scenario_code;
use crate::domain::settings::Settings;
use crate::repositories::pipeline_repository::chat_discussions::{
    list_messages, list_threads, load_corpus, ANCHOR_QUESTION,
};
use crate::repositories::pipeline_repository::practice::{
    get_question, list_point_receipts, list_points, PracticeQuestionRecord,
};
use crate::repositories::pipeline_repository::practice_discussions::list_thread;
use crate::repositories::pipeline_repository::practice_notes::{
    attempts_for_question, notes_for_question,
};
use crate::repositories::pipeline_repository::scenario_store::get_scenario;
use crate::services::chat_question_context::{QuestionContext, SiblingThread};
use crate::services::chat_question_error::{store, ChatRunError};
use crate::services::chat_question_text::{package_documents, PackagedDocument};
use crate::services::practice_page::point_receipt;
use crate::state::AppState;

/// CONST: the question-row side vocabulary (the column's CHECK) and how the model
/// is told who asks. Protocol of this table, not case data: the CHECK itself
/// fixes the two values.
const SIDE_CROSS: &str = "george";
const ASKER_CROSS: &str = "opposing counsel, on cross-examination";
const ASKER_DIRECT: &str = "her own lawyer";
/// CONST: the `source_kind` whose `source_ref` is a graph Evidence id.
const SOURCE_INSTANCE: &str = "instance";

/// Everything one turn needs, gathered.
pub struct Gathered {
    pub question: PracticeQuestionRecord,
    pub context: QuestionContext,
    pub documents: Vec<PackagedDocument>,
}

/// The display name for a login: the witness's name, a reviewer's bench name, or
/// — for anyone else — the login itself, which is at least never wrong.
pub fn display_name(settings: &Settings, username: &str) -> String {
    let read = &settings.practice_read;
    if username == read.witness_username {
        return settings.question_chat.witness_display_name.clone();
    }
    read.reviewer_usernames
        .iter()
        .position(|u| u == username)
        .and_then(|i| read.reviewer_display_names.get(i).cloned())
        .unwrap_or_else(|| username.to_string())
}

/// The case's participants in switcher order: the witness, then the bench.
pub fn participants(settings: &Settings) -> Vec<String> {
    let read = &settings.practice_read;
    let mut out = vec![read.witness_username.clone()];
    out.extend(read.reviewer_usernames.iter().cloned());
    out
}

/// Load the question or name its absence.
///
/// # Errors
/// [`ChatRunError::QuestionNotFound`] or a store failure.
pub async fn require_question(
    state: &AppState,
    question_id: Uuid,
) -> Result<PracticeQuestionRecord, ChatRunError> {
    get_question(&state.pipeline_pool, question_id)
        .await
        .map_err(store("get_question", question_id))?
        .ok_or(ChatRunError::QuestionNotFound(question_id))
}

/// Gather the package for `viewer`'s turn on `question_id`.
///
/// # Errors
/// Any store failure, named by operation.
pub async fn gather(
    state: &AppState,
    question_id: Uuid,
    viewer: &str,
) -> Result<Gathered, ChatRunError> {
    let settings = state.settings.current();
    let pool = &state.pipeline_pool;
    let q = require_question(state, question_id).await?;
    let scenario = get_scenario(pool, q.scenario_id)
        .await
        .map_err(store("get_scenario", question_id))?
        .ok_or(ChatRunError::QuestionNotFound(question_id))?;
    let points = list_points(pool, q.scenario_id)
        .await
        .map_err(store("list_points", question_id))?;
    let seeded = list_point_receipts(pool, q.scenario_id)
        .await
        .map_err(store("list_point_receipts", question_id))?;
    let attempts = attempts_for_question(pool, q.scenario_id, question_id)
        .await
        .map_err(store("attempts_for_question", question_id))?;
    let notes = notes_for_question(pool, question_id)
        .await
        .map_err(store("notes_for_question", question_id))?
        .into_iter()
        .filter(|n| n.struck_at.is_none())
        .collect();
    let earlier = list_thread(pool, question_id)
        .await
        .map_err(store("list_thread", question_id))?;
    let siblings = sibling_threads(state, &settings, question_id, viewer).await?;
    let corpus = load_corpus(pool)
        .await
        .map_err(store("load_corpus", question_id))?;
    let primary = primary_document(state, &q).await;

    let read = &settings.practice_read;
    let tactic_name = q
        .tactic
        .and_then(|t| usize::try_from(t).ok())
        .and_then(|t| read.tactic_names.get(t.saturating_sub(1)).cloned());
    let context = QuestionContext {
        scenario_code: scenario_code(scenario.code_ordinal),
        question_text: q.text.clone(),
        kind: q.kind.clone(),
        asker: if q.side == SIDE_CROSS {
            ASKER_CROSS
        } else {
            ASKER_DIRECT
        }
        .to_string(),
        tactic_name,
        attack: scenario.theme_statement.clone(),
        watch_for: q.watch_for.clone(),
        points: points
            .iter()
            .map(|p| (p.position, p.text.clone(), point_receipt(p, &seeded)))
            .collect(),
        pair_said: q.pair_said.clone(),
        pair_admitted: q.pair_admitted.clone(),
        attempts,
        notes,
        siblings,
        earlier,
        witness_name: settings.question_chat.witness_display_name.clone(),
        viewer_name: display_name(&settings, viewer),
        case_timezone: read.case_timezone.clone(),
    };
    let documents = package_documents(&corpus, primary.as_deref());
    Ok(Gathered {
        question: q,
        context,
        documents,
    })
}

/// Every OTHER person's thread on this question, as speaker/text lines.
async fn sibling_threads(
    state: &AppState,
    settings: &Settings,
    question_id: Uuid,
    viewer: &str,
) -> Result<Vec<SiblingThread>, ChatRunError> {
    let pool = &state.pipeline_pool;
    let anchor = question_id.to_string();
    let threads = list_threads(pool, ANCHOR_QUESTION, &anchor, viewer)
        .await
        .map_err(store("list_threads", question_id))?;
    let mut out = Vec::new();
    for t in threads.into_iter().filter(|t| t.username != viewer) {
        let owner = display_name(settings, &t.username);
        let lines = list_messages(pool, t.id)
            .await
            .map_err(store("list_messages", question_id))?
            .into_iter()
            .filter_map(|m| {
                let speaker = if m.role == "assistant" {
                    "The AI".to_string()
                } else {
                    owner.clone()
                };
                m.rendered_text.map(|text| (speaker, text))
            })
            .collect::<Vec<_>>();
        if !lines.is_empty() {
            out.push(SiblingThread {
                owner_name: owner,
                lines,
            });
        }
    }
    Ok(out)
}

/// The stored document an `instance` question was built from, via its Evidence
/// node's `CONTAINED_IN` edge — or `None`, logged when the graph could not say.
async fn primary_document(state: &AppState, q: &PracticeQuestionRecord) -> Option<String> {
    let evidence_id = match (q.source_kind.as_str(), q.source_ref.as_deref()) {
        (SOURCE_INSTANCE, Some(r)) => r.to_string(),
        _ => return None,
    };
    let cypher = query(
        "MATCH (e {id: $id})-[:CONTAINED_IN]->(d:Document) RETURN d.source_document_id AS doc LIMIT 1",
    )
    .param("id", evidence_id.clone());
    match state.graph.execute(cypher).await {
        Ok(mut rows) => match rows.next().await {
            Ok(Some(row)) => row.get::<String>("doc").ok(),
            Ok(None) => {
                tracing::warn!(question_id = %q.id, %evidence_id,
                    "question chat: the question's source Evidence node is not in the graph; no primary document");
                None
            }
            Err(e) => {
                tracing::warn!(question_id = %q.id, %evidence_id, error = %e,
                    "question chat: reading the primary document failed; the corpus is still complete");
                None
            }
        },
        Err(e) => {
            tracing::warn!(question_id = %q.id, %evidence_id, error = %e,
                "question chat: the graph could not be asked for the primary document; the corpus is still complete");
            None
        }
    }
}
