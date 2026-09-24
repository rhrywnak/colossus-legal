//! What the caller asks for, and the Messages API body built from it.
//!
//! ## The shape of a request, and where the cache breakpoints go
//!
//! Anthropic renders a request in the order `tools → system → messages`, and a
//! cache entry is a PREFIX of that rendering: change one byte anywhere and every
//! breakpoint after it misses. So the request is laid out stable-first:
//!
//! ```text
//! tools                      (fixed per anchor type)
//! system  [prompt blocks…]   ← breakpoint 1: the prompt and any stable preamble
//! messages[0] user:
//!     document, document, …  ← breakpoint 2: the document package
//!     text: the context      (changes when an answer, note, or thread changes)
//! messages[1..]: history, verbatim
//! last user turn             ← breakpoint 3: everything so far, for the next turn
//! ```
//!
//! Three breakpoints; the API allows four. [`build_body`] asserts the count.
//!
//! ## Domain note: why documents are plain text, not custom content
//!
//! Plain-text documents are sentence-chunked by the provider and cited by
//! character range, so a citation is a sentence or a run of sentences — the right
//! size for a quotation card. Custom-content documents cite whole blocks, which
//! with one block per page would quote an entire page.

use serde_json::{json, Map, Value};

/// STRUCTURAL: the Messages API allows at most four `cache_control` breakpoints.
const MAX_BREAKPOINTS: usize = 4;

/// STRUCTURAL: the content-block type of a document (wire protocol).
const BLOCK_DOCUMENT: &str = "document";

/// How long a cache entry lives. The API offers exactly these two.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CacheTtl {
    /// Five minutes (the provider default; cheapest write).
    FiveMinutes,
    /// One hour (a costlier write that survives the pauses of a long conversation).
    OneHour,
}

impl CacheTtl {
    /// Parse the wire spelling (`5m` / `1h`), as stored in configuration.
    ///
    /// # Errors
    /// Any other spelling, named, so a mistyped setting fails loudly at startup.
    pub fn parse(raw: &str) -> Result<Self, RequestError> {
        match raw.trim() {
            "5m" => Ok(Self::FiveMinutes),
            "1h" => Ok(Self::OneHour),
            other => Err(RequestError::BadTtl(other.to_string())),
        }
    }

    fn wire(self) -> &'static str {
        match self {
            Self::FiveMinutes => "5m",
            Self::OneHour => "1h",
        }
    }
}

/// One document the model may quote from. Its `text` is ALSO what citations are
/// verified against ([`crate::citation`]), so it must be byte-identical to what
/// the caller stores.
#[derive(Debug, Clone, PartialEq)]
pub struct ChatDocument {
    /// The document's display name, as the model should speak of it.
    pub title: String,
    /// Metadata the model reads but cannot cite (date, kind, role in the case).
    pub context: Option<String>,
    /// The full text.
    pub text: String,
}

/// One tool the model may call. `input_schema` is a JSON Schema object.
#[derive(Debug, Clone, PartialEq)]
pub struct ToolSpec {
    /// The tool's name, as the model calls it.
    pub name: String,
    /// When to call it and what it returns — the model's only guide.
    pub description: String,
    /// JSON Schema for the input.
    pub input_schema: Value,
}

/// A message in the conversation: a role and its verbatim content blocks.
#[derive(Debug, Clone, PartialEq)]
pub struct Message {
    /// `user` or `assistant`.
    pub role: Role,
    /// Content blocks exactly as sent or received.
    pub content: Vec<Value>,
}

/// Who a message is from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    /// The human side (also carries tool results).
    User,
    /// The model.
    Assistant,
}

impl Role {
    /// The wire spelling.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::User => "user",
            Self::Assistant => "assistant",
        }
    }
}

/// Everything one call needs. Every value comes from the caller's configuration.
#[derive(Debug, Clone)]
pub struct ChatRequest {
    /// Model id.
    pub model: String,
    /// Output cap for each call.
    pub max_tokens: u32,
    /// System prompt blocks (plain text). Breakpoint 1 goes on the last one.
    pub system: Vec<String>,
    /// The documents, in order. Breakpoint 2 goes on the last one.
    pub documents: Vec<ChatDocument>,
    /// The per-anchor context text that follows the documents in `messages[0]`.
    pub context: String,
    /// The conversation so far, INCLUDING the new user message, oldest first.
    pub history: Vec<Message>,
    /// Tools the model may call.
    pub tools: Vec<ToolSpec>,
    /// `output_config.effort`, when configured.
    pub effort: Option<String>,
    /// Send `thinking: {type: adaptive}` explicitly.
    pub adaptive_thinking: bool,
    /// Server-side compaction trigger in input tokens; `None` = compaction off.
    pub compaction_trigger_tokens: Option<u64>,
    /// Cache lifetime for every breakpoint.
    pub cache_ttl: CacheTtl,
}

/// A request that cannot be sent as specified.
#[derive(Debug, thiserror::Error, PartialEq)]
pub enum RequestError {
    /// A TTL spelling the API does not accept.
    #[error("cache TTL `{0}` is not one the API accepts (`5m` or `1h`)")]
    BadTtl(String),
    /// No new user message to answer.
    #[error("the conversation must end with a user message; it ends with {0}")]
    NotUserLast(&'static str),
    /// Too many cache breakpoints were placed.
    #[error("{0} cache breakpoints were placed; the API allows {MAX_BREAKPOINTS}")]
    TooManyBreakpoints(usize),
    /// A document with no text cannot be cited from.
    #[error("document `{0}` has no text — a document the model cannot quote must not be sent")]
    EmptyDocument(String),
    /// A pre-warm with no documents would warm nothing the next turn could read.
    #[error("there are no documents to keep loaded — a pre-warm needs the document package")]
    NothingToWarm,
}

/// Build the JSON body for one streamed Messages call.
///
/// # Errors
/// See [`RequestError`].
pub fn build_body(req: &ChatRequest) -> Result<Value, RequestError> {
    match req.history.last() {
        Some(m) if m.role == Role::User => {}
        Some(_) => return Err(RequestError::NotUserLast("an assistant message")),
        None => return Err(RequestError::NotUserLast("nothing")),
    }
    let cache = json!({"type": "ephemeral", "ttl": req.cache_ttl.wire()});
    let mut body = Map::new();
    body.insert("model".into(), json!(req.model));
    body.insert("max_tokens".into(), json!(req.max_tokens));
    body.insert("stream".into(), json!(true));
    body.insert("system".into(), system_blocks(&req.system, &cache));
    body.insert("messages".into(), messages(req, &cache)?);
    if !req.tools.is_empty() {
        body.insert("tools".into(), tools(&req.tools));
    }
    if req.adaptive_thinking {
        body.insert("thinking".into(), json!({"type": "adaptive"}));
    }
    if let Some(effort) = &req.effort {
        body.insert("output_config".into(), json!({"effort": effort}));
    }
    if let Some(trigger) = req.compaction_trigger_tokens {
        body.insert(
            "context_management".into(),
            json!({"edits": [{"type": "compact_20260112",
                "trigger": {"type": "input_tokens", "value": trigger}}]}),
        );
    }
    let body = Value::Object(body);
    let placed = count_breakpoints(&body);
    if placed > MAX_BREAKPOINTS {
        return Err(RequestError::TooManyBreakpoints(placed));
    }
    Ok(body)
}

/// Body keys the `count_tokens` endpoint does not accept.
///
/// STRUCTURAL: protocol vocabulary. `count_tokens` answers "how many input tokens
/// is this?", so the keys describing what to GENERATE are rejected, not ignored.
const COUNT_STRIPPED: [&str; 4] = [
    "stream",
    "max_tokens",
    "output_config",
    "context_management",
];

/// The same body, shaped for `POST /v1/messages/count_tokens`.
///
/// # Why: derived from `build_body`, never rebuilt alongside it
///
/// The whole point of counting is to learn the size of the body that will
/// actually be sent. A second builder would drift from the first the moment
/// either changed, and the count would then be a confident measurement of
/// something nobody sends. So this calls [`build_body`] and removes the four keys
/// the endpoint rejects — a body that cannot silently disagree with the real one.
///
/// The `cache_control` markers are deliberately left in place: the endpoint
/// accepts them and they do not change the count, so keeping them keeps the two
/// bodies one edit apart instead of two.
///
/// # Errors
/// See [`RequestError`] — the same ones [`build_body`] raises.
pub fn build_count_body(req: &ChatRequest) -> Result<Value, RequestError> {
    let mut body = build_body(req)?;
    // `build_body` always returns a JSON object; this is the borrow, not a guess.
    if let Some(map) = body.as_object_mut() {
        for key in COUNT_STRIPPED {
            map.remove(key);
        }
    }
    Ok(body)
}

/// The body for a cache PRE-WARM: the same prefix a real turn sends, asking for
/// no reply at all.
///
/// ## Why: derived from `build_body`, like [`build_count_body`]
///
/// A pre-warm is worth something only if its bytes are the bytes the next real
/// turn will send — tools, system prompt, documents, and the thinking and effort
/// settings, which the provider renders into the prompt. So this calls
/// [`build_body`] and then takes away, never adds:
///
/// - `messages` is cut to its first message, and that message to its
///   documents. The per-question context and the conversation sit AFTER the
///   document breakpoint, so dropping them changes nothing the cache is keyed on,
///   and leaves no tail to write. Two breakpoints remain: the system prompt and
///   the last document.
/// - `max_tokens` becomes `0`, the provider's documented pre-warm: it runs the
///   prefill, answers `content: []`, and bills no output.
/// - `stream` is removed, because the provider refuses `max_tokens: 0` with
///   streaming. Streaming is how bytes travel, not part of the cached prompt.
///
/// `thinking`, `output_config` and `context_management` are kept exactly as a
/// real turn sends them. Measured on 2026-09-24 (CC_TASK_CACHE_KEEPWARM_v1
/// Stage P): this body, with `thinking: adaptive`, read 368,832 cached tokens
/// and wrote none.
///
/// The request's `history` is ignored: the body keeps no conversation.
///
/// # Errors
/// [`RequestError::NothingToWarm`] with no documents, or whatever
/// [`build_body`] raises.
pub fn build_prewarm_body(req: &ChatRequest) -> Result<Value, RequestError> {
    if req.documents.is_empty() {
        return Err(RequestError::NothingToWarm);
    }
    // `build_body` insists on a conversation ending with a user message. An
    // EMPTY one satisfies it and places no breakpoint (there is no block to put
    // one on), and it is cut away below with everything else after messages[0].
    let mut shaped = req.clone();
    shaped.history = vec![Message {
        role: Role::User,
        content: Vec::new(),
    }];
    let mut body = build_body(&shaped)?;
    // `build_body` always returns a JSON object; this is the borrow, not a guess.
    if let Some(map) = body.as_object_mut() {
        map.insert("max_tokens".into(), json!(0));
        map.remove("stream");
        if let Some(Value::Array(messages)) = map.get_mut("messages") {
            messages.truncate(1);
            if let Some(Value::Array(content)) =
                messages.first_mut().and_then(|m| m.get_mut("content"))
            {
                content.retain(|b| b.get("type").and_then(Value::as_str) == Some(BLOCK_DOCUMENT));
            }
        }
    }
    Ok(body)
}

fn system_blocks(parts: &[String], cache: &Value) -> Value {
    let last = parts.len().saturating_sub(1);
    Value::Array(
        parts
            .iter()
            .enumerate()
            .map(|(i, text)| {
                let mut b = json!({"type": "text", "text": text});
                if i == last {
                    b["cache_control"] = cache.clone();
                }
                b
            })
            .collect(),
    )
}

fn messages(req: &ChatRequest, cache: &Value) -> Result<Value, RequestError> {
    let mut first: Vec<Value> = Vec::with_capacity(req.documents.len() + 1);
    for doc in &req.documents {
        if doc.text.trim().is_empty() {
            return Err(RequestError::EmptyDocument(doc.title.clone()));
        }
        let mut b = json!({
            "type": BLOCK_DOCUMENT,
            "source": {"type": "text", "media_type": "text/plain", "data": doc.text},
            "title": doc.title,
            "citations": {"enabled": true},
        });
        if let Some(ctx) = &doc.context {
            b["context"] = json!(ctx);
        }
        first.push(b);
    }
    if let Some(last_doc) = first.last_mut() {
        last_doc["cache_control"] = cache.clone();
    }
    first.push(json!({"type": "text", "text": req.context}));

    let mut out = vec![json!({"role": "user", "content": first})];
    let last = req.history.len() - 1; // non-empty: checked by build_body
    for (i, m) in req.history.iter().enumerate() {
        let mut content = m.content.clone();
        if i == last {
            if let Some(tail) = content.last_mut() {
                tail["cache_control"] = cache.clone();
            }
        }
        out.push(json!({"role": m.role.as_str(), "content": content}));
    }
    Ok(Value::Array(out))
}

fn tools(specs: &[ToolSpec]) -> Value {
    Value::Array(
        specs
            .iter()
            .map(|t| {
                json!({"name": t.name, "description": t.description,
                       "input_schema": t.input_schema})
            })
            .collect(),
    )
}

/// Count `cache_control` markers anywhere in the body.
fn count_breakpoints(v: &Value) -> usize {
    match v {
        Value::Object(map) => {
            usize::from(map.contains_key("cache_control"))
                + map.values().map(count_breakpoints).sum::<usize>()
        }
        Value::Array(items) => items.iter().map(count_breakpoints).sum(),
        _ => 0,
    }
}

#[cfg(test)]
#[path = "request_tests.rs"]
mod tests;
