//! How big the chat package really is — asked of the provider, not estimated.
//!
//! ## Why this module exists
//!
//! `chat_question_run::check_size` refuses a package the model's context cannot
//! hold. Until v2.2.2 it sized that package by dividing characters by
//! `question_chat_chars_per_token` (3). CC_TASK_CHAT_COST_FIX_v1 measured the
//! truth with the provider's own `count_tokens` endpoint: **690,965 characters of
//! corpus bill as 368,810 tokens — 1.90 characters per token**, because every
//! document block is sent with `citations: {enabled: true}` and the API chunks and
//! indexes it. The estimate was under-counting by **1.6×**: it read 230k for a
//! package that bills 370k, and it was comparing that number against a limit
//! expressed in real API tokens. Two token scales in one guard.
//!
//! So the guard now asks. The count is free — `count_tokens` generates nothing.
//!
//! ## What is measured and what is still estimated
//!
//! The **prefix** (system blocks + every document block) is measured. It is also
//! the expensive part, the cached part, and — since v2.2.2 — identical for every
//! question, which is exactly what makes one memoised number valid for all of
//! them (see `chat_question_text::package_documents`).
//!
//! The **per-question tail** (the rendered context block) is still estimated from
//! `question_chat_chars_per_token`. It is ~1,400 tokens against the prefix's
//! ~369,000 — 0.4% — and it is the one part that genuinely changes every request,
//! so measuring it would mean a network call per turn to refine a rounding error.
//!
//! ## Rust Learning: interior mutability behind a shared reference
//!
//! Callers hold `&AppState`, so this cache is only ever reachable through a `&`.
//! A plain `Option<(u64, u64)>` could never be written through one. `RwLock` moves
//! the borrow check to runtime: many readers, or one writer, enforced by the lock
//! rather than the compiler. `RwLock` and not `Mutex` because the steady state is
//! "every request reads, nobody writes".
//!
//! The invalidation key is a **fingerprint of the content**, not a timer. A cache
//! that expires on a clock is wrong for a while by design; a cache keyed on what
//! it measured is either right or absent.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::RwLock;

use colossus_chat::request::build_count_body;
use colossus_chat::{ChatBackend, ChatRequest, Message, Role};
use serde_json::json;

use crate::services::chat_question_text::estimate_tokens;

/// Stands in for the per-question context while the PREFIX is counted.
///
/// STRUCTURAL: it is never sent to a model — `count_tokens` generates nothing —
/// and it exists only because the Messages body requires a non-empty text block
/// after the documents. Its own handful of tokens are counted into the prefix,
/// which makes the guard very slightly conservative: the right direction for a
/// guard, and far inside the 80,000-token headroom it is compared against.
const PREFIX_PROBE: &str = "(counting the fixed prefix)";

/// The size of one package, and whether it was measured or estimated.
///
/// ## Rust Learning: why an enum and not `(u64, bool)`
///
/// The two carry the same data, but a `bool` needs a name to be read — a caller
/// writing `if size.1` is guessing. More importantly, Standing Rule 1 wants the
/// two states observable: the enum is what lets the log line and `run_config` say
/// WHICH number guarded a turn, so an operator reading a refusal can tell a
/// measurement from a fallback without reading this file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PackageSize {
    /// The provider counted it.
    Measured(u64),
    /// The provider could not be asked; this is `chars_per_token` arithmetic.
    /// The failure has already been logged by name.
    Estimated(u64),
}

impl PackageSize {
    /// The number, whichever it is.
    pub fn tokens(self) -> u64 {
        match self {
            Self::Measured(n) | Self::Estimated(n) => n,
        }
    }

    /// `"measured"` or `"estimated"`, for logs and `run_config`.
    pub fn provenance(self) -> &'static str {
        match self {
            Self::Measured(_) => "measured",
            Self::Estimated(_) => "estimated",
        }
    }
}

/// Everything inside the cached prefix, borrowed.
///
/// # Why: borrowed parts and not the caller's document type
///
/// The caching and counting here is a mechanism, not a domain rule — nothing in
/// it knows what a legal document is. Taking `&[PackagedDocument]` would have
/// bound it to this application's type for no gain and made the mechanism
/// unreachable to any other caller (the reusability checkpoint, Standing Rule
/// 11). Borrowed `&str`s carry the same bytes with no copy, and the one caller
/// that DOES know about documents builds this in two lines.
///
/// `context` stays an `Option` rather than flattening to `""`: a document with no
/// context line and one with an empty context line are different prefixes, and a
/// fingerprint that collapsed them would be wrong in exactly the silent way
/// Standing Rule 1 forbids.
///
/// ## Rust Learning: the `'a` on this struct
///
/// The lifetime says "this value may not outlive whatever it borrowed from". It
/// is what lets the struct hold `&str` into the caller's own `String`s instead of
/// owning copies of a 690KB corpus — and the compiler, not a comment, is what
/// guarantees those strings are still alive when the fingerprint reads them.
pub struct PrefixParts<'a> {
    /// The system blocks, in order.
    pub system: &'a [String],
    /// `(title, context, text)` per document block, in order.
    pub documents: Vec<(&'a str, Option<&'a str>, &'a str)>,
}

/// One measured prefix, remembered against the content it measured.
///
/// Lives in `AppState` and is shared by every chat request.
#[derive(Debug, Default)]
pub struct PrefixSizeCache {
    /// `(fingerprint of the prefix, its token count)`.
    seen: RwLock<Option<(u64, u64)>>,
}

impl PrefixSizeCache {
    /// An empty cache. The first chat request fills it (ruled 2026-09-23, Q3:
    /// lazy, so a provider outage delays a chat rather than the whole backend).
    pub fn new() -> Self {
        Self::default()
    }

    fn get(&self, fingerprint: u64) -> Option<u64> {
        // best-effort: a poisoned lock means a previous writer panicked mid-write.
        // Treating that as "nothing cached" costs one extra free count and keeps
        // the chat working; it is logged rather than propagated because a cache
        // miss is not a reason to refuse a turn.
        match self.seen.read() {
            Ok(seen) => seen.and_then(|(f, tokens)| (f == fingerprint).then_some(tokens)),
            Err(_) => {
                tracing::warn!(
                    "chat prefix size: the cache lock was poisoned; recounting this turn"
                );
                None
            }
        }
    }

    fn put(&self, fingerprint: u64, tokens: u64) {
        match self.seen.write() {
            Ok(mut seen) => *seen = Some((fingerprint, tokens)),
            Err(_) => tracing::warn!(
                "chat prefix size: the cache lock was poisoned; this count is not remembered"
            ),
        }
    }
}

/// The message the probe request ends with, so the body has a user turn to answer.
///
/// ## Rust Learning: building a JSON content block by hand
///
/// `Message::content` is the RAW API block array — the same `Vec<Value>` a real
/// turn replays — because thinking blocks carry signatures and tool ids that must
/// survive verbatim. A probe has none of that, so one text block is the whole of
/// it.
fn probe_message() -> Message {
    Message {
        role: Role::User,
        content: vec![json!({"type": "text", "text": PREFIX_PROBE})],
    }
}

/// Measure the package: the prefix from the provider, the tail from arithmetic.
///
/// `prefix` is the cached half, borrowed — the hot path (a cache hit) copies
/// nothing. `build_probe` is called ONLY on a miss, because building the request
/// it returns clones the whole corpus; passing a ready-made request would pay
/// that ~690KB copy on every turn to use it on roughly none of them.
///
/// Never fails. A provider that cannot be asked yields [`PackageSize::Estimated`]
/// and a warning naming the operation (ruled 2026-09-23, Q2(a)): refusing to
/// answer a witness because a *size estimate* could not be refined would be a
/// worse failure than the one being fixed.
///
/// ## Rust Learning: `FnOnce() -> ChatRequest` as a lazy argument
///
/// `build_probe` is a closure the caller hands over rather than a value. `FnOnce`
/// is the loosest of the three call traits — it may consume what it captured —
/// which is what lets the caller move owned strings into it. Nothing runs until
/// it is called, so "only on a miss" is enforced by where the call sits, not by a
/// comment asking the caller to be careful.
pub async fn package_size<F>(
    backend: &dyn ChatBackend,
    cache: &PrefixSizeCache,
    prefix: &PrefixParts<'_>,
    context: &str,
    chars_per_token: usize,
    build_probe: F,
) -> PackageSize
where
    F: FnOnce() -> ChatRequest,
{
    let tail = estimate_tokens(&[context], chars_per_token) as u64;
    let fingerprint = prefix_fingerprint(prefix);
    if let Some(tokens) = cache.get(fingerprint) {
        // The quiet path, and the one an operator most needs to SEE: without it,
        // a cache that had silently stopped working would look exactly like a
        // cache that was working perfectly (one info line, then nothing).
        tracing::debug!(
            prefix_tokens = tokens,
            "chat prefix size: served from the remembered count"
        );
        return PackageSize::Measured(tokens + tail);
    }
    match count_prefix(backend, build_probe()).await {
        Ok(tokens) => {
            tracing::info!(
                prefix_tokens = tokens,
                documents = prefix.documents.len(),
                "chat prefix size: counted by the provider and remembered"
            );
            cache.put(fingerprint, tokens);
            PackageSize::Measured(tokens + tail)
        }
        Err(error) => {
            let parts: Vec<&str> = prefix
                .system
                .iter()
                .map(String::as_str)
                .chain(prefix.documents.iter().map(|(_, _, text)| *text))
                .collect();
            let estimate = estimate_tokens(&parts, chars_per_token) as u64;
            tracing::warn!(
                %error,
                chars_per_token,
                estimate,
                "chat prefix size: count_tokens failed, so the size guard is running on a \
                 CHARACTER ESTIMATE for this turn — it is known to under-count a \
                 citation-enabled corpus by about 1.6x"
            );
            PackageSize::Estimated(estimate + tail)
        }
    }
}

/// Ask the provider for the prefix's size.
///
/// The probe's per-question halves are [`PREFIX_PROBE`], so the answer describes
/// the prefix alone and is valid for every question that shares it.
async fn count_prefix(backend: &dyn ChatBackend, mut probe: ChatRequest) -> Result<u64, String> {
    probe.context = PREFIX_PROBE.to_string();
    probe.history = vec![probe_message()];
    let body = build_count_body(&probe).map_err(|e| e.to_string())?;
    backend.count_tokens(&body).await.map_err(|e| e.to_string())
}

/// A fingerprint of everything inside the cached prefix.
///
/// Hashes the actual bytes — both system blocks and every document's title,
/// context and text — rather than lengths or ids. A corpus edit that keeps the
/// character count identical is rare, but a fingerprint that misses it would
/// leave the guard quoting a number for a package that no longer exists.
///
/// # Why: the per-question context and the history are deliberately NOT hashed
///
/// They are not in the prefix. Hashing them would make the fingerprint change
/// every turn, which would turn a memoised count into a network call per request
/// — the exact cost this feature exists to remove.
fn prefix_fingerprint(prefix: &PrefixParts<'_>) -> u64 {
    let mut hasher = DefaultHasher::new();
    prefix.system.hash(&mut hasher);
    for (title, context, text) in &prefix.documents {
        title.hash(&mut hasher);
        context.hash(&mut hasher);
        text.hash(&mut hasher);
    }
    hasher.finish()
}

#[cfg(test)]
#[path = "chat_prefix_size_tests.rs"]
mod tests;
