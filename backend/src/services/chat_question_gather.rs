//! Collecting the question chat's package from the stores (the I/O half of the
//! package; [`crate::services::chat_question_context`] renders it).
//!
//! Every read either succeeds or fails the request by name — with ONE exception,
//! stated: resolving which document the question was BUILT FROM goes to the
//! graph, and if the graph cannot answer, the package is still complete (every
//! document is in it); only the "primary first" ordering is lost. That is logged
//! as a warning naming the question, never swallowed (Standing Rule 1).

use neo4rs::query;

use crate::domain::chat_params::QuestionChatParams;
use crate::neo4j::schema::CONTAINED_IN;
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

/// STRUCTURAL: the `practice_questions.kind` vocabulary (its CHECK) — schema,
/// not case data; the words each kind is SPOKEN as are settings rows.
const KIND_CROSS: &str = "cross";
const KIND_DIRECT: &str = "direct";
const KIND_REDIRECT: &str = "redirect";
/// STRUCTURAL: the `source_kind` whose `source_ref` is a graph Evidence id (the
/// column's CHECK vocabulary).
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

/// The rows the package is built from, read in one place.
struct Rows {
    scenario: crate::repositories::pipeline_repository::scenario_store::ScenarioRecord,
    points: Vec<(i32, String, Option<String>)>,
    attempts: Vec<crate::repositories::pipeline_repository::practice_notes::AttemptRecord>,
    notes: Vec<crate::repositories::pipeline_repository::practice_notes::NoteRecord>,
    earlier:
        Vec<crate::repositories::pipeline_repository::practice_discussions::DiscussionTurnRecord>,
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
    let q = require_question(state, question_id).await?;
    let rows = read_rows(state, &q).await?;
    let siblings = sibling_threads(state, &settings, question_id, viewer).await?;
    let corpus = load_corpus(&state.pipeline_pool)
        .await
        .map_err(store("load_corpus", question_id))?;
    let primary = primary_document(state, &q).await;
    let read = &settings.practice_read;
    let chat = &settings.question_chat;
    let context = QuestionContext {
        scenario_code: scenario_code(rows.scenario.code_ordinal),
        question_text: q.text.clone(),
        kind: q.kind.clone(),
        asker: asker_for(&q.kind, chat),
        tactic_name: tactic_name(q.tactic, &read.tactic_names),
        attack: rows.scenario.theme_statement.clone(),
        watch_for: q.watch_for.clone(),
        points: rows.points,
        pair_said: q.pair_said.clone(),
        pair_admitted: q.pair_admitted.clone(),
        attempts: rows.attempts,
        notes: rows.notes,
        siblings,
        earlier: rows.earlier,
        witness_name: chat.witness_display_name.clone(),
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

/// The question's scenario, points with their receipts, answers, standing notes
/// and the old dock's thread — every read named by operation.
async fn read_rows(state: &AppState, q: &PracticeQuestionRecord) -> Result<Rows, ChatRunError> {
    let pool = &state.pipeline_pool;
    let id = q.id;
    let scenario = get_scenario(pool, q.scenario_id)
        .await
        .map_err(store("get_scenario", id))?
        .ok_or(ChatRunError::QuestionNotFound(id))?;
    let points = list_points(pool, q.scenario_id)
        .await
        .map_err(store("list_points", id))?;
    let seeded = list_point_receipts(pool, q.scenario_id)
        .await
        .map_err(store("list_point_receipts", id))?;
    let attempts = attempts_for_question(pool, q.scenario_id, id)
        .await
        .map_err(store("attempts_for_question", id))?;
    let notes = notes_for_question(pool, id)
        .await
        .map_err(store("notes_for_question", id))?
        .into_iter()
        .filter(|n| n.struck_at.is_none())
        .collect();
    let earlier = list_thread(pool, id)
        .await
        .map_err(store("list_thread", id))?;
    Ok(Rows {
        scenario,
        points: points
            .iter()
            .map(|p| (p.position, p.text.clone(), point_receipt(p, &seeded)))
            .collect(),
        attempts,
        notes,
        earlier,
    })
}

/// How the model is told who asks — by the question's KIND, in the configured
/// words (`question_chat_asker_*`). A kind this build does not know is named to
/// the model as itself and logged, never silently read as one of the three.
pub fn asker_for(kind: &str, chat: &QuestionChatParams) -> String {
    match kind {
        KIND_CROSS => chat.asker_cross.clone(),
        KIND_DIRECT => chat.asker_direct.clone(),
        KIND_REDIRECT => chat.asker_redirect.clone(),
        other => {
            tracing::warn!(
                kind = other,
                "question chat: a question kind with no configured asker"
            );
            other.to_string()
        }
    }
}

/// The tactic card's name, or `None` for a question without one. A number outside
/// the stored vocabulary is logged — the column's CHECK makes it unreachable.
pub fn tactic_name(tactic: Option<i16>, names: &[String]) -> Option<String> {
    let card = tactic?;
    // best-effort: a negative card number (impossible under the column's CHECK)
    // falls through to the warning below rather than being silently dropped.
    let name = usize::try_from(card)
        .ok()
        .and_then(|n| n.checked_sub(1))
        .and_then(|i| names.get(i).cloned());
    if name.is_none() {
        tracing::warn!(card, "question chat: a tactic card with no stored name");
    }
    name
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
                    settings.question_chat.ai_display_name.clone()
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
    let cypher = query(&format!(
        "MATCH (e {{id: $id}})-[:{CONTAINED_IN}]->(d:Document) \
         RETURN d.source_document_id AS doc LIMIT 1"
    ))
    .param("id", evidence_id.clone());
    match state.graph.execute(cypher).await {
        Ok(mut rows) => match rows.next().await {
            Ok(Some(row)) => match row.get::<String>("doc") {
                Ok(doc) => Some(doc),
                Err(e) => {
                    tracing::warn!(question_id = %q.id, %evidence_id, error = %e,
                        "question chat: the source Document has no source_document_id; no primary document");
                    None
                }
            },
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::settings::Settings;

    /// The fixture's bench is one reviewer; the case has two. Widen it as the
    /// seeded store has it.
    fn settings() -> Settings {
        let mut s = Settings::for_test();
        s.practice_read.reviewer_usernames = vec!["cpenzien".into(), "roman".into()];
        s.practice_read.reviewer_display_names = vec!["Chuck".into(), "Roman".into()];
        s
    }

    #[test]
    fn display_name_resolves_the_witness_the_bench_and_anyone_else() {
        let s = settings();
        assert_eq!(display_name(&s, "docmarie"), "Marie");
        assert_eq!(display_name(&s, "cpenzien"), "Chuck");
        assert_eq!(display_name(&s, "roman"), "Roman");
        // Someone outside the case is shown by login — never wrong, never blank.
        assert_eq!(display_name(&s, "akadmin"), "akadmin");
    }

    #[test]
    fn participants_are_the_witness_then_the_bench_in_order() {
        assert_eq!(
            participants(&settings()),
            vec!["docmarie", "cpenzien", "roman"]
        );
    }

    #[test]
    fn the_asker_comes_from_the_kind_in_configured_words() {
        let chat = QuestionChatParams::for_test();
        assert_eq!(asker_for("cross", &chat), chat.asker_cross);
        assert_eq!(asker_for("direct", &chat), chat.asker_direct);
        assert_eq!(asker_for("redirect", &chat), chat.asker_redirect);
        // An unknown kind is named as itself — never silently read as another.
        assert_eq!(asker_for("voir_dire", &chat), "voir_dire");
    }

    #[test]
    fn tactic_names_are_one_based_and_out_of_range_is_none() {
        let names = vec!["Bait".to_string(), "Loop".to_string()];
        assert_eq!(tactic_name(Some(1), &names).as_deref(), Some("Bait"));
        assert_eq!(tactic_name(Some(2), &names).as_deref(), Some("Loop"));
        assert_eq!(tactic_name(Some(3), &names), None);
        assert_eq!(tactic_name(Some(0), &names), None);
        assert_eq!(tactic_name(Some(-1), &names), None);
        assert_eq!(tactic_name(None, &names), None);
    }
}
