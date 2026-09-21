//! The question chat's READS: the switcher, one thread, the earlier discussion.
//!
//! Visibility (Roman's ruling, 2026-09-21): every thread on a question is
//! readable by everyone on the case. Only the OWNER writes — `read_only` is
//! decided here and enforced again on the write path.

use chrono::Utc;
use uuid::Uuid;

use crate::dto::chat_discussion::{ChatThreadsPayload, EarlierRowDto, ThreadPayload, ThreadRowDto};
use crate::repositories::pipeline_repository::chat_discussions::{
    find_discussion, list_messages, list_threads, ThreadSummary, ANCHOR_QUESTION,
};
use crate::repositories::pipeline_repository::models::get_model_by_id;
use crate::repositories::pipeline_repository::practice_discussions::list_thread;
use crate::repositories::pipeline_repository::PipelineRepoError;
use crate::services::chat_question_error::{store, ChatRunError};
use crate::services::chat_question_gather::{display_name, participants, require_question};
use crate::services::chat_question_view::{earlier_dto, message_dto, short_day};
use crate::state::AppState;

/// STRUCTURAL: the name prefix the chip drops ("Claude Opus 5" → "Opus 5") —
/// the mockup's chip names the model family, and every grounded model is Claude.
const CHIP_DROPPED_PREFIX: &str = "Claude ";

/// The switcher's payload for `viewer` on `question_id`.
///
/// # Errors
/// [`ChatRunError::QuestionNotFound`] or a named store failure.
pub async fn threads_payload(
    state: &AppState,
    question_id: Uuid,
    viewer: &str,
) -> Result<ChatThreadsPayload, ChatRunError> {
    require_question(state, question_id).await?;
    let settings = state.settings.current();
    let chat = &settings.question_chat;
    let tz = &settings.practice_read.case_timezone;
    let pool = &state.pipeline_pool;
    let stored = list_threads(pool, ANCHOR_QUESTION, &question_id.to_string(), viewer)
        .await
        .map_err(store("list_threads", question_id))?;
    let model = get_model_by_id(pool, &chat.model)
        .await
        .map_err(|e| store("get_model_by_id", question_id)(PipelineRepoError::from(e)))?;
    let today = short_day(Utc::now(), tz);
    let threads = switcher_order(&participants(&settings), viewer, &stored)
        .iter()
        .map(|u| {
            row(
                u,
                viewer,
                stored.iter().find(|t| &t.username == u),
                &settings,
                tz,
                &today,
            )
        })
        .collect::<Vec<_>>();
    let others = threads
        .iter()
        .filter(|t| !t.is_viewer)
        .map(|t| t.display_name.clone())
        .collect();
    Ok(ChatThreadsPayload {
        question_id: question_id.to_string(),
        viewer: viewer.to_string(),
        model_short_name: model
            .as_ref()
            .map_or_else(|| chat.model.clone(), |m| chip_name(&m.display_name)),
        grounded: model.as_ref().is_some_and(|m| m.grounded && m.is_active),
        others,
        threads,
        earlier: earlier_row(state, question_id, tz).await?,
        client_idle_timeout_secs: chat.client_idle_timeout_secs,
        max_turns: chat.max_turns,
    })
}

/// Switcher order: the viewer first, then the case's participants, then anyone
/// else who has a thread (a login outside the bench is shown, never hidden).
pub fn switcher_order(
    participants: &[String],
    viewer: &str,
    stored: &[ThreadSummary],
) -> Vec<String> {
    let mut order = vec![viewer.to_string()];
    order.extend(participants.iter().filter(|u| *u != viewer).cloned());
    for t in stored {
        if !order.contains(&t.username) {
            order.push(t.username.clone());
        }
    }
    order
}

/// "Claude Opus 5" → "Opus 5" — the chip names the model, not the vendor.
pub fn chip_name(display_name: &str) -> String {
    display_name
        .strip_prefix(CHIP_DROPPED_PREFIX)
        .unwrap_or(display_name)
        .to_string()
}

/// The Earlier team discussion row, when the question has old-dock rows.
async fn earlier_row(
    state: &AppState,
    question_id: Uuid,
    tz: &str,
) -> Result<Option<EarlierRowDto>, ChatRunError> {
    let old = list_thread(&state.pipeline_pool, question_id)
        .await
        .map_err(store("list_thread", question_id))?;
    Ok(old.last().map(|last| EarlierRowDto {
        message_count: i64::try_from(old.len()).unwrap_or(i64::MAX),
        last_on: short_day(last.created_at, tz),
        preview: last.text.clone(),
    }))
}

fn row(
    username: &str,
    viewer: &str,
    summary: Option<&ThreadSummary>,
    settings: &crate::domain::settings::Settings,
    tz: &str,
    today: &str,
) -> ThreadRowDto {
    let name = display_name(settings, username);
    ThreadRowDto {
        username: username.to_string(),
        initial: name
            .chars()
            .next()
            .map(|c| c.to_uppercase().collect())
            .unwrap_or_default(),
        display_name: name,
        is_viewer: username == viewer,
        read_only: username != viewer,
        message_count: summary.map_or(0, |s| s.visible_count),
        unread: summary.map_or(0, |s| s.unread),
        preview: summary.and_then(|s| s.last_text.clone()),
        resumed_from: summary
            .filter(|s| s.visible_count > 0)
            .map(|s| short_day(s.created_at, tz))
            .filter(|day| day != today),
    }
}

/// One person's thread, as `viewer` sees it.
///
/// # Errors
/// [`ChatRunError::QuestionNotFound`] or a named store failure.
pub async fn thread_payload(
    state: &AppState,
    question_id: Uuid,
    owner: &str,
    viewer: &str,
) -> Result<ThreadPayload, ChatRunError> {
    require_question(state, question_id).await?;
    let settings = state.settings.current();
    let tz = &settings.practice_read.case_timezone;
    let owner_name = display_name(&settings, owner);
    let found = find_discussion(
        &state.pipeline_pool,
        ANCHOR_QUESTION,
        &question_id.to_string(),
        owner,
    )
    .await
    .map_err(store("find_discussion", question_id))?;
    let messages = match found {
        None => Vec::new(),
        Some(d) => list_messages(&state.pipeline_pool, d.id)
            .await
            .map_err(store("list_messages", question_id))?
            .iter()
            .filter_map(|m| {
                let author = if m.role == "assistant" {
                    settings.question_chat.ai_display_name.as_str()
                } else {
                    owner_name.as_str()
                };
                message_dto(m, author, tz)
            })
            .collect(),
    };
    Ok(ThreadPayload {
        username: Some(owner.to_string()),
        display_name: owner_name,
        read_only: owner != viewer,
        messages,
    })
}

/// The old dock's shared thread, read-only (ADDENDUM_1). Nothing here writes.
///
/// # Errors
/// [`ChatRunError::QuestionNotFound`] or a named store failure.
pub async fn earlier_payload(
    state: &AppState,
    question_id: Uuid,
) -> Result<ThreadPayload, ChatRunError> {
    require_question(state, question_id).await?;
    let settings = state.settings.current();
    let tz = &settings.practice_read.case_timezone;
    let turns = list_thread(&state.pipeline_pool, question_id)
        .await
        .map_err(store("list_thread", question_id))?;
    Ok(ThreadPayload {
        username: None,
        display_name: String::new(),
        read_only: true,
        messages: turns
            .iter()
            .enumerate()
            .map(|(i, t)| earlier_dto(t, i, tz))
            .collect(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn summary(username: &str) -> ThreadSummary {
        ThreadSummary {
            id: Uuid::nil(),
            username: username.into(),
            created_at: Utc::now(),
            visible_count: 1,
            last_seq: 1,
            seen_seq: 0,
            unread: 1,
            last_text: None,
        }
    }

    #[test]
    fn the_viewer_leads_then_the_case_then_anyone_else_with_a_thread() {
        let case = vec!["docmarie".to_string(), "cpenzien".into(), "roman".into()];
        assert_eq!(
            switcher_order(&case, "cpenzien", &[]),
            vec!["cpenzien", "docmarie", "roman"]
        );
        // A login outside the case who has written is shown, never hidden.
        assert_eq!(
            switcher_order(&case, "docmarie", &[summary("akadmin"), summary("roman")]),
            vec!["docmarie", "cpenzien", "roman", "akadmin"]
        );
    }

    #[test]
    fn the_chip_drops_the_vendor_prefix_only() {
        assert_eq!(chip_name("Claude Opus 5"), "Opus 5");
        assert_eq!(chip_name("Qwen 32B"), "Qwen 32B");
    }
}
