//! Tests for the measured package size: no socket, no key, no tokens.

use std::sync::Mutex;

use colossus_chat::{AssistantMessage, CacheTtl, ChatDocument, ChatTransportError};
use serde_json::Value;

use super::*;
use crate::repositories::pipeline_repository::chat_discussions::CorpusDocument;
use crate::services::chat_question_text::{package_documents, PackagedDocument};

/// The borrowed prefix, built the way `prepare_turn` builds it.
fn parts<'a>(system: &'a [String], documents: &'a [PackagedDocument]) -> PrefixParts<'a> {
    PrefixParts {
        system,
        documents: documents
            .iter()
            .map(|d| {
                (
                    d.block.title.as_str(),
                    d.block.context.as_deref(),
                    d.block.text.as_str(),
                )
            })
            .collect(),
    }
}

/// A backend that answers a scripted count and records how often it was asked.
///
/// ## Rust Learning: `Mutex` in a test double, not `RefCell`
///
/// `ChatBackend` is `Send + Sync`, so a double must be too — and `RefCell` is
/// neither. `Mutex` gives interior mutability that crosses threads, which is what
/// lets this be passed as `&dyn ChatBackend` into an `async fn`.
struct Counter {
    answer: Mutex<Vec<Result<u64, String>>>,
    asked: Mutex<Vec<Value>>,
}

impl Counter {
    fn new(answers: Vec<Result<u64, String>>) -> Self {
        let mut answers = answers;
        answers.reverse();
        Self {
            answer: Mutex::new(answers),
            asked: Mutex::new(Vec::new()),
        }
    }

    fn times_asked(&self) -> usize {
        self.asked.lock().expect("test lock").len()
    }
}

#[async_trait::async_trait]
impl colossus_chat::ChatBackend for Counter {
    async fn call(
        &self,
        _body: &Value,
        _on_text: &(dyn Fn(String) + Send + Sync),
    ) -> Result<AssistantMessage, ChatTransportError> {
        unreachable!("the size guard never generates")
    }

    async fn count_tokens(&self, body: &Value) -> Result<u64, ChatTransportError> {
        self.asked.lock().expect("test lock").push(body.clone());
        match self
            .answer
            .lock()
            .expect("test lock")
            .pop()
            .expect("the script ran out of answers")
        {
            Ok(n) => Ok(n),
            Err(message) => Err(ChatTransportError::Status {
                status: 500,
                body: message,
            }),
        }
    }

    async fn prewarm(&self, _body: &Value) -> Result<colossus_chat::Usage, ChatTransportError> {
        unreachable!("the size guard never pre-warms")
    }
}

fn corpus(pages: &[&str]) -> Vec<CorpusDocument> {
    vec![CorpusDocument {
        id: "doc-a".into(),
        title: "A".into(),
        document_date: None,
        pages: pages
            .iter()
            .enumerate()
            .map(|(i, t)| (i32::try_from(i).unwrap() + 1, (*t).to_string()))
            .collect(),
    }]
}

fn system() -> Vec<String> {
    vec!["PROMPT".to_string(), "NARRATIVE".to_string()]
}

fn probe(system: Vec<String>, documents: &[PackagedDocument]) -> ChatRequest {
    ChatRequest {
        model: "m".into(),
        max_tokens: 16_000,
        system,
        documents: documents.iter().map(|d| d.block.clone()).collect(),
        context: "CONTEXT".into(),
        history: Vec::new(),
        tools: Vec::new(),
        effort: None,
        adaptive_thinking: true,
        compaction_trigger_tokens: None,
        cache_ttl: CacheTtl::OneHour,
    }
}

/// `chars_per_token` is 4 here so the tail's contribution is a number the test
/// can state rather than a coincidence: "CONTEXT" is 7 characters, 7 / 4 = 1.
const TAIL_DIVISOR: usize = 4;
const TAIL_TOKENS: u64 = 1;

/// The call under test, with the arguments every case shares.
///
/// ## Rust Learning: why the probe closure is built HERE and not passed in
///
/// `package_size` takes `FnOnce`, so the closure may consume what it captures —
/// which means it cannot be reused across calls. Building a fresh one inside this
/// helper is what lets each test call `size(...)` as many times as it likes.
async fn size(
    backend: &Counter,
    cache: &PrefixSizeCache,
    system: &[String],
    documents: &[PackagedDocument],
    context: &str,
) -> PackageSize {
    let owned = system.to_vec();
    let prefix = parts(system, documents);
    package_size(backend, cache, &prefix, context, TAIL_DIVISOR, || {
        probe(owned, documents)
    })
    .await
}

#[tokio::test]
async fn the_prefix_count_is_measured_not_estimated() {
    let documents = package_documents(&corpus(&["a page of text"]));
    let backend = Counter::new(vec![Ok(370_000)]);
    let cache = PrefixSizeCache::new();
    let measured = size(&backend, &cache, &system(), &documents, "CONTEXT").await;
    // The provider's number, plus the estimated tail — NOT characters / divisor,
    // which for this tiny corpus would be a handful of tokens.
    assert_eq!(measured, PackageSize::Measured(370_000 + TAIL_TOKENS));
    assert_eq!(measured.provenance(), "measured");
    assert_eq!(backend.times_asked(), 1);
}

#[tokio::test]
async fn a_second_turn_on_the_same_corpus_does_not_ask_again() {
    let documents = package_documents(&corpus(&["a page of text"]));
    // One answer only: a second request to the provider would panic the script.
    let backend = Counter::new(vec![Ok(370_000)]);
    let cache = PrefixSizeCache::new();
    for context in ["CONTEXT", "a different question's context entirely"] {
        let measured = size(&backend, &cache, &system(), &documents, context).await;
        assert_eq!(measured.provenance(), "measured");
    }
    assert_eq!(
        backend.times_asked(),
        1,
        "the prefix is the same, so it is counted once"
    );
}

#[tokio::test]
async fn a_changed_corpus_forces_a_recount() {
    let cache = PrefixSizeCache::new();
    let backend = Counter::new(vec![Ok(370_000), Ok(371_500)]);
    let first = package_documents(&corpus(&["a page of text"]));
    let second = package_documents(&corpus(&["a page of text", "and another"]));
    let a = size(&backend, &cache, &system(), &first, "CONTEXT").await;
    let b = size(&backend, &cache, &system(), &second, "CONTEXT").await;
    assert_eq!(a, PackageSize::Measured(370_000 + TAIL_TOKENS));
    assert_eq!(b, PackageSize::Measured(371_500 + TAIL_TOKENS));
    assert_eq!(backend.times_asked(), 2);
}

/// An edited prompt or narrative is part of the prefix too — a recount must
/// follow it, not only a corpus change.
#[tokio::test]
async fn an_edited_system_block_forces_a_recount() {
    let documents = package_documents(&corpus(&["a page of text"]));
    let cache = PrefixSizeCache::new();
    let backend = Counter::new(vec![Ok(370_000), Ok(370_400)]);
    let edited = vec!["PROMPT, revised".to_string(), "NARRATIVE".to_string()];
    let _ = size(&backend, &cache, &system(), &documents, "CONTEXT").await;
    let after = size(&backend, &cache, &edited, &documents, "CONTEXT").await;
    assert_eq!(after, PackageSize::Measured(370_400 + TAIL_TOKENS));
    assert_eq!(backend.times_asked(), 2);
}

/// Ruled 2026-09-23 Q2(a): a provider that cannot be asked yields an ESTIMATE and
/// a warning, never a refusal and never a silent zero.
#[tokio::test]
async fn a_failed_count_falls_back_to_the_estimate_and_is_not_remembered() {
    // 14 characters of page + "PROMPT" + "NARRATIVE" = 29; 29 / 4 = 7, + 1 tail.
    let documents = package_documents(&corpus(&["a page of text"]));
    let cache = PrefixSizeCache::new();
    let backend = Counter::new(vec![Err("the provider said no".into()), Ok(370_000)]);
    let fallback = size(&backend, &cache, &system(), &documents, "CONTEXT").await;
    assert_eq!(fallback.provenance(), "estimated");
    assert_eq!(fallback, PackageSize::Estimated(7 + TAIL_TOKENS));

    // A failure must not poison the cache: the next turn asks again and succeeds.
    let recovered = size(&backend, &cache, &system(), &documents, "CONTEXT").await;
    assert_eq!(recovered, PackageSize::Measured(370_000 + TAIL_TOKENS));
    assert_eq!(backend.times_asked(), 2);
}

/// What is counted must be what is sent: the probe body carries the real system
/// blocks and the real document blocks, and none of the generation keys.
#[tokio::test]
async fn the_counted_body_is_the_body_that_would_be_sent() {
    let documents = package_documents(&corpus(&["a page of text"]));
    let backend = Counter::new(vec![Ok(1)]);
    let cache = PrefixSizeCache::new();
    let _ = size(&backend, &cache, &system(), &documents, "CONTEXT").await;
    let asked = backend.asked.lock().expect("test lock");
    let body = asked.first().expect("the provider was asked");
    assert_eq!(body["system"][0]["text"], "PROMPT");
    assert_eq!(body["system"][1]["text"], "NARRATIVE");
    assert_eq!(body["messages"][0]["content"][0]["type"], "document");
    assert_eq!(
        body["messages"][0]["content"][0]["source"]["data"],
        "a page of text"
    );
    for rejected in [
        "stream",
        "max_tokens",
        "output_config",
        "context_management",
    ] {
        assert!(
            body.get(rejected).is_none(),
            "count_tokens rejects `{rejected}`, so it must not be sent"
        );
    }
}

/// The per-question halves are replaced by the probe text, which is what makes
/// ONE count valid for every question that shares the prefix.
#[tokio::test]
async fn the_probe_hides_the_per_question_half() {
    let documents = package_documents(&corpus(&["a page of text"]));
    let backend = Counter::new(vec![Ok(1)]);
    let cache = PrefixSizeCache::new();
    let _ = size(
        &backend,
        &cache,
        &system(),
        &documents,
        "a very specific question's context",
    )
    .await;
    let asked = backend.asked.lock().expect("test lock");
    let body = asked.first().expect("the provider was asked");
    let sent = serde_json::to_string(body).expect("serializable");
    assert!(
        !sent.contains("a very specific question's context"),
        "{sent}"
    );
    assert!(sent.contains(PREFIX_PROBE), "{sent}");
}

#[test]
fn a_package_size_says_which_kind_of_number_it_is() {
    assert_eq!(PackageSize::Measured(10).tokens(), 10);
    assert_eq!(PackageSize::Estimated(10).tokens(), 10);
    assert_eq!(PackageSize::Measured(10).provenance(), "measured");
    assert_eq!(PackageSize::Estimated(10).provenance(), "estimated");
    assert_ne!(PackageSize::Measured(10), PackageSize::Estimated(10));
}

/// Guards the one thing a `ChatDocument` must keep: `package_documents` builds
/// the blocks the fingerprint hashes, so an empty corpus must not hash the same
/// as a populated one.
#[test]
fn an_empty_corpus_does_not_fingerprint_like_a_full_one() {
    let full = package_documents(&corpus(&["a page of text"]));
    let empty: Vec<PackagedDocument> = Vec::new();
    let system = system();
    assert_ne!(
        prefix_fingerprint(&parts(&system, &full)),
        prefix_fingerprint(&parts(&system, &empty))
    );
}

/// A document whose TEXT is unchanged but whose title or context line moved is
/// still a changed prefix — the cache invalidates on bytes, not on lengths.
#[test]
fn a_retitled_document_changes_the_fingerprint() {
    let original = package_documents(&corpus(&["a page of text"]));
    let mut retitled = original.clone();
    retitled[0].block = ChatDocument {
        title: "A, RENAMED".into(),
        ..retitled[0].block.clone()
    };
    let system = system();
    assert_ne!(
        prefix_fingerprint(&parts(&system, &original)),
        prefix_fingerprint(&parts(&system, &retitled))
    );
}

/// An empty context line and a missing one are different prefixes; a fingerprint
/// that collapsed them would let a stale count guard a changed package.
#[test]
fn a_missing_context_line_does_not_fingerprint_like_an_empty_one() {
    let system = system();
    let text = "a page of text";
    let missing = PrefixParts {
        system: &system,
        documents: vec![("A", None, text)],
    };
    let empty = PrefixParts {
        system: &system,
        documents: vec![("A", Some(""), text)],
    };
    assert_ne!(prefix_fingerprint(&missing), prefix_fingerprint(&empty));
}

/// Two documents swapped is a different prefix — the ORDER is what v2.2.2 fixed,
/// so a fingerprint blind to it would be blind to the whole defect.
#[test]
fn reordering_the_documents_changes_the_fingerprint() {
    let system = system();
    let a = ("A", Some("dated"), "alpha");
    let b = ("B", Some("dated"), "beta");
    let forward = PrefixParts {
        system: &system,
        documents: vec![a, b],
    };
    let reversed = PrefixParts {
        system: &system,
        documents: vec![b, a],
    };
    assert_ne!(prefix_fingerprint(&forward), prefix_fingerprint(&reversed));
}
