//! The case file every question-chat call is built on: the prompt, the case
//! narrative, the documents and the tools, plus the model settings that shape
//! them. Shared by a real chat turn and by a cache pre-warm.
//!
//! ## Why this is its own module
//!
//! A pre-warm keeps the case file in the provider's cache only if its bytes are
//! EXACTLY the bytes the next real turn sends. Change one character of the
//! prompt, one document, one tool description, or the thinking or effort
//! setting, and the pre-warm fills a cache entry no turn ever reads. The only
//! reliable way to keep two callers byte-identical is to give them one builder.
//! [`crate::services::chat_question_run::prepare_turn`] builds its request with
//! [`build_request`], and [`case_file_request`] builds the pre-warm's request
//! with the same function, from the same files, the same corpus reader and the
//! same tool list.
//!
//! ## Domain note: what "the case file" is
//!
//! It is everything a chat turn sends BEFORE the per-question part: the system
//! prompt and narrative (cache breakpoint 1) and every document in the record
//! (breakpoint 2). It is the same for every question, which is why one pre-warm
//! serves whichever question Marie opens next.

use std::sync::Arc;

use colossus_chat::{ChatRequest, Message};

use crate::domain::chat_params::QuestionChatParams;
use crate::repositories::pipeline_repository::chat_discussions::load_corpus;
use crate::repositories::pipeline_repository::PipelineRepoError;
use crate::services::chat_question_error::ChatRunError;
use crate::services::chat_question_text::{package_documents, PackagedDocument};
use crate::services::chat_question_tools::tools;
use crate::state::AppState;

/// Why the case file could not be assembled.
#[derive(Debug, thiserror::Error)]
pub enum CaseFileError {
    /// The prompt or the narrative file is missing, unreadable or empty.
    #[error(transparent)]
    Template(#[from] ChatRunError),
    /// The corpus could not be read from the store.
    #[error("load_corpus failed: {0}")]
    Corpus(#[from] PipelineRepoError),
}

/// The engine request for one turn, every value from configuration.
///
/// `context` and `history` are the per-question part. A pre-warm passes them
/// empty, because they sit after the document breakpoint and are never sent in
/// a pre-warm (see `colossus_chat::build_prewarm_body`).
pub fn build_request(
    chat: &QuestionChatParams,
    prompt: String,
    narrative: String,
    context: String,
    documents: &[PackagedDocument],
    history: Vec<Message>,
) -> ChatRequest {
    ChatRequest {
        model: chat.model.clone(),
        max_tokens: chat.max_tokens,
        system: vec![prompt, narrative],
        documents: documents.iter().map(|d| d.block.clone()).collect(),
        context,
        history,
        tools: Vec::new(),
        effort: chat.effort.map(|e| e.as_wire().to_string()),
        // Opus 5 thinks adaptively by default; sending it explicitly keeps the
        // behavior the same on a model whose default differs.
        adaptive_thinking: true,
        compaction_trigger_tokens: chat.compaction_trigger_tokens.map(u64::from),
        cache_ttl: chat.cache_ttl,
    }
}

/// Read a template-directory file, distinguishing absent from empty.
///
/// # Errors
/// [`ChatRunError::FileUnreadable`], naming the file and saying whether it was
/// missing or empty.
pub async fn read_template(
    state: &AppState,
    what: &'static str,
    file: &str,
) -> Result<String, ChatRunError> {
    let path = std::path::Path::new(state.registry.template_dir()).join(file.trim());
    match tokio::fs::read_to_string(&path).await {
        Ok(text) if text.trim().is_empty() => Err(ChatRunError::FileUnreadable {
            what,
            path: path.display().to_string(),
            detail: "the file is EMPTY".into(),
        }),
        Ok(text) => Ok(text),
        Err(e) => Err(ChatRunError::FileUnreadable {
            what,
            path: path.display().to_string(),
            detail: e.to_string(),
        }),
    }
}

/// The case file as a request a pre-warm can be built from, with the tools a
/// real turn offers already set.
///
/// ## Why the tools are set here
///
/// A real turn's tools are set inside `colossus_chat::run_turn`, from
/// `tools.iter().map(|t| t.spec())`. A pre-warm never runs a turn, so it sets
/// them the same way, from the same [`tools`] list. The answer history the
/// `answer_history` tool carries changes what the tool RETURNS, never its spec,
/// so an empty one gives the identical tool definitions (asserted in the tests).
///
/// # Errors
/// [`CaseFileError`]: a template that cannot be read, or a corpus read failure.
pub async fn case_file_request(state: &AppState) -> Result<ChatRequest, CaseFileError> {
    let settings = state.settings.current();
    let chat = &settings.question_chat;
    let prompt = read_template(state, "the question chat's prompt", &chat.prompt_file).await?;
    let narrative = read_template(state, "the case narrative", &chat.narrative_file).await?;
    let corpus = load_corpus(&state.pipeline_pool).await?;
    let documents = Arc::new(package_documents(&corpus));
    let offered = tools(Arc::clone(&documents), String::new());
    let mut request = build_request(
        chat,
        prompt,
        narrative,
        String::new(),
        &documents,
        Vec::new(),
    );
    request.tools = offered.iter().map(|t| t.spec()).collect();
    Ok(request)
}

#[cfg(test)]
#[path = "chat_case_prefix_tests.rs"]
mod tests;
