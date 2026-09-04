//! Tests for the embedding boundary's two guards: the empty input that is
//! refused, and the over-length input that is announced.
//!
//! The pure ones run everywhere. The three at the bottom need the real model
//! weights (~270 MB, cached at `FASTEMBED_CACHE_PATH`) and are `#[ignore]`d, so
//! a checkout with no cache still has a green suite.

use super::*;

/// S-11's composed verbatim query, measured on DEV 2026-09-01 with the real
/// nomic-embed-text-v1.5 WordPiece vocabulary. Pinned as numbers so the case
/// this whole change exists for is asserted rather than described in a comment.
const S11_BYTES: usize = 2444;
const S11_TOKENS: usize = 543;

// ---------------------------------------------------------------------------
// The guard — pure
// ---------------------------------------------------------------------------

/// ⚑ An empty text cannot reach the model, and the error says why.
///
/// This is the guard R2 asked for. It sits at the embedding call rather than on
/// the gather side because `EmbeddingService::model` is private, so
/// `embed_one` and `embed_batch` are the only doors to a vector — a caller who
/// never read a doc comment still hits it.
#[test]
fn an_empty_text_can_never_reach_the_model() {
    for blank in ["", "   ", "\n\t ", "\u{a0}"] {
        let err = reject_empty(&[blank]).expect_err("a blank text must be refused");
        assert!(
            matches!(err, EmbeddingError::EmptyInput { index: 0 }),
            "expected EmptyInput, got {err:?}"
        );
    }
}

/// The refusal explains the consequence, not just the fact.
#[test]
fn the_refusal_says_what_an_empty_vector_would_have_done() {
    let rendered = EmbeddingError::EmptyInput { index: 3 }.to_string();
    assert!(rendered.contains("index 3"), "{rendered}");
    assert!(rendered.contains("degenerate vector"), "{rendered}");
    assert!(
        rendered.contains("look like a working search"),
        "the operator needs to know it fails by looking correct: {rendered}"
    );
}

/// A batch is refused at the FIRST blank, and the index identifies it.
#[test]
fn a_batch_names_which_of_its_texts_was_blank() {
    let err = reject_empty(&["real text", "also real", "  ", "later"])
        .expect_err("a batch with a blank must be refused");
    assert!(matches!(err, EmbeddingError::EmptyInput { index: 2 }));

    reject_empty(&["real text", "also real"]).expect("a clean batch passes");
    reject_empty(&[]).expect("an empty batch has nothing to refuse");
}

// ---------------------------------------------------------------------------
// The truncation bound — pure
// ---------------------------------------------------------------------------

/// The pre-filter is SOUND: it may say "check this" about a safe text, but it
/// must never say "safe" about one that truncates.
///
/// That asymmetry is the whole point. A cheaper, more accurate estimate
/// (chars/4) would be wrong in exactly the direction that reintroduces the
/// silent truncation this module exists to prevent.
#[test]
fn the_truncation_bound_is_sound_rather_than_accurate() {
    // Well under: skipped with no tokenization at all.
    assert!(cannot_truncate("a short query", 8192));
    // A text of exactly `limit - 2` bytes still cannot exceed the limit, because
    // each token eats at least one byte and two specials are added.
    let exact = "a".repeat(8190);
    assert!(cannot_truncate(&exact, 8192));
    // One byte more and the bound no longer holds, so it is checked properly.
    let over = "a".repeat(8191);
    assert!(!cannot_truncate(&over, 8192));
    // The bound is on BYTES, so multi-byte text is treated conservatively — it
    // is checked more often than strictly necessary, never less.
    assert!(!cannot_truncate(&"é".repeat(5000), 8192));
}

/// S-11's composed query — 2444 bytes — is far below the raised cap, so the
/// query that used to be truncated is not even a candidate now.
#[test]
fn s11s_composed_query_is_nowhere_near_the_raised_cap() {
    let composed = "x".repeat(S11_BYTES);
    assert!(
        cannot_truncate(&composed, MAX_INPUT_TOKENS),
        "S-11's query was 543 tokens against the old 512 cap; at {MAX_INPUT_TOKENS} its \
         byte length alone proves it cannot truncate"
    );
    assert!(
        !cannot_truncate(&composed, 512),
        "and under the OLD cap the bound would have sent it to the tokenizer — which is \
         where the 543-token count pinned below turns it into a warning"
    );
    // The bound alone cannot show S-11 truncated at 512; it shows only that the
    // text would be CHECKED. The count is what settles it, so it is pinned as a
    // number here rather than left in a comment.
    assert_eq!(
        truncation_warning(S11_TOKENS, 512, "…").map(|w| w.dropped),
        Some(31),
        "S-11's composed query was 543 tokens against the old 512 cap — 31 tokens, the \
         end of the query, silently discarded"
    );
    assert_eq!(
        truncation_warning(S11_TOKENS, MAX_INPUT_TOKENS, "…"),
        None,
        "and nothing is dropped at the raised cap"
    );
}

// ---------------------------------------------------------------------------
// The warning — pure
// ---------------------------------------------------------------------------

/// ⚑ The warning fires only when tokens are actually lost, and names how many.
///
/// This is the check that stops a silent truncation. Without it the `dropped`
/// arithmetic could be wrong, or the warning could fire on texts that fit, and
/// nothing would notice.
#[test]
fn the_warning_fires_only_when_tokens_are_actually_lost() {
    let over =
        truncation_warning(543, 512, "the composed query").expect("543 against 512 must warn");
    assert_eq!(over.tokens, 543, "it names the PRE-truncation count");
    assert_eq!(over.limit, 512, "and the limit it was measured against");
    assert_eq!(
        over.dropped, 31,
        "and how many tokens the tokenizer discards"
    );
    assert_eq!(over.excerpt, "the composed query");

    assert_eq!(
        truncation_warning(512, 512, "x"),
        None,
        "exactly at the limit fits"
    );
    assert_eq!(
        truncation_warning(208, 512, "x"),
        None,
        "S-9 fit even at the old cap"
    );
    assert_eq!(truncation_warning(0, 512, "x"), None);
}

/// The warning's excerpt is bounded, so the over-long input that triggered it
/// cannot flood the log with the megabytes that were the problem.
#[test]
fn the_warning_does_not_quote_back_the_whole_over_long_input() {
    let huge = "word ".repeat(100_000);
    let w = truncation_warning(50_000, MAX_INPUT_TOKENS, &huge).expect("must warn");
    assert!(
        w.excerpt.chars().count() <= WARN_EXCERPT_CHARS + 1,
        "excerpt was {} chars",
        w.excerpt.chars().count()
    );
}

/// ⚑ The warning is actually WIRED to the embed path.
///
/// The two tests above pin what the warning says; this pins that it is still
/// said. `truncation_warning` could be correct and unreferenced — deleting the
/// `tracing::warn!` would leave every other test green — so the call site is
/// asserted against the module's own source, the way an invariant that spans
/// code rather than data has to be (CLAUDE.md §4 rule 21).
#[test]
fn the_truncation_warning_is_still_wired_into_the_embed_path() {
    let source = include_str!("embedding_service.rs");
    assert!(
        source.contains("self.warn_on_truncation(&refs);"),
        "embed_batch must still check for truncation"
    );
    assert!(
        source.contains("self.warn_on_truncation(&[text]);"),
        "embed_one must still check for truncation"
    );
    let warn_site = source
        .split("if let Some(w) = truncation_warning(count, limit, text)")
        .nth(1)
        .expect("the warn site must still call truncation_warning");
    for field in [
        "tokens = w.tokens",
        "limit = w.limit",
        "dropped = w.dropped",
        "excerpt",
    ] {
        assert!(
            warn_site.contains(field),
            "the warning must still name {field} — an operator cannot act on a bare \
             'input too long'"
        );
    }
}

// ---------------------------------------------------------------------------
// The excerpt — pure
// ---------------------------------------------------------------------------

/// The warning's excerpt never panics on a multi-byte character.
///
/// Slicing a `String` by byte index mid-character panics, which would turn a
/// diagnostic warning into a crash — the observability code taking down the
/// operation it was meant to explain.
#[test]
fn the_excerpt_does_not_split_a_multibyte_character() {
    let accented = "é".repeat(WARN_EXCERPT_CHARS * 2);
    let shown = excerpt(&accented);
    assert_eq!(
        shown.chars().count(),
        WARN_EXCERPT_CHARS + 1,
        "plus the ellipsis"
    );
    assert!(shown.ends_with('…'));

    let short = "a short text";
    assert_eq!(
        excerpt(short),
        short,
        "a short text is quoted whole, with no ellipsis"
    );
}

// ---------------------------------------------------------------------------
// The real model — #[ignore]d, needs the cached weights
// ---------------------------------------------------------------------------

fn cache_path() -> Option<String> {
    std::env::var("FASTEMBED_CACHE_PATH").ok()
}

/// ⚑ THE PROOF the "no re-embed needed" ruling rests on.
///
/// A text under the OLD 512-token cap embeds to the byte-identical vector
/// before and after the raise. If this ever failed, every vector in the corpus
/// would need regenerating and every stored similarity would be stale.
///
/// It holds because `max_length` reaches exactly one place in fastembed —
/// `TruncationParams` — so for an input that does not truncate, the token ids
/// handed to ONNX are the same ids, and ONNX is deterministic.
///
/// ```text
/// FASTEMBED_CACHE_PATH=/mnt/data/models cargo test -p colossus-legal-backend --lib \
///   services::embedding_service::tests::a_short_text_embeds_identically \
///   -- --ignored --nocapture
/// ```
#[test]
#[ignore = "needs the cached model weights; set FASTEMBED_CACHE_PATH"]
fn a_short_text_embeds_identically_under_the_raised_cap() {
    let Some(cache) = cache_path() else {
        panic!("set FASTEMBED_CACHE_PATH to the directory holding the model weights");
    };
    let text = "The scenario's theme, and one allegation's verbatim words about the \
                fifty thousand dollars.";

    let before = embed_with_cap(&cache, 512, text);
    let after = embed_with_cap(&cache, MAX_INPUT_TOKENS, text);

    assert_eq!(
        before.len(),
        768,
        "nomic-embed-text-v1.5 is 768-dimensional"
    );
    assert_eq!(
        before, after,
        "raising the cap must not move a vector that was never truncated — if this \
         fails, the whole corpus needs re-embedding"
    );
}

/// The raise actually took effect and was not clamped away.
///
/// fastembed silently does `max_length.min(model_max_length)`, so a model whose
/// `tokenizer_config.json` says 512 would leave the cap at 512 while every line
/// of code claimed 8192.
#[test]
#[ignore = "needs the cached model weights; set FASTEMBED_CACHE_PATH"]
fn the_requested_cap_is_the_cap_actually_enforced() {
    let Some(cache) = cache_path() else {
        panic!("set FASTEMBED_CACHE_PATH to the directory holding the model weights");
    };
    let service = EmbeddingService::new(&cache).expect("the model loads");
    assert_eq!(
        service.effective_max_tokens,
        Some(MAX_INPUT_TOKENS),
        "the model clamped the requested cap — the raise did not take effect"
    );
}

/// An over-length input is COUNTED correctly, which is what the warning
/// reports. Uses a text long enough to truncate even at the raised cap.
#[test]
#[ignore = "needs the cached model weights; set FASTEMBED_CACHE_PATH"]
fn an_over_length_input_is_counted_before_truncation() {
    let Some(cache) = cache_path() else {
        panic!("set FASTEMBED_CACHE_PATH to the directory holding the model weights");
    };
    let service = EmbeddingService::new(&cache).expect("the model loads");

    let long = "allegation ".repeat(MAX_INPUT_TOKENS);
    let counted = service.count_tokens(&long).expect("the tokenizer counts");
    assert!(
        counted > MAX_INPUT_TOKENS,
        "the count must be the PRE-truncation length, not the capped one; got {counted}"
    );

    let short = "a short query";
    let short_count = service.count_tokens(short).expect("the tokenizer counts");
    assert!(
        (1..=16).contains(&short_count),
        "a four-word text should be a handful of tokens; got {short_count}"
    );
}

/// A text with an embedded model still refuses to embed an empty string —
/// proving the guard is on the real path, not only on the helper.
#[test]
#[ignore = "needs the cached model weights; set FASTEMBED_CACHE_PATH"]
fn the_real_embedder_refuses_an_empty_query() {
    let Some(cache) = cache_path() else {
        panic!("set FASTEMBED_CACHE_PATH to the directory holding the model weights");
    };
    let mut service = EmbeddingService::new(&cache).expect("the model loads");

    let err = service
        .embed_one("   ")
        .expect_err("the real embedder must refuse a blank query");
    assert!(matches!(err, EmbeddingError::EmptyInput { index: 0 }));

    let err = service
        .embed_batch(vec!["real".to_string(), String::new()])
        .expect_err("the real embedder must refuse a blank text in a batch");
    assert!(matches!(err, EmbeddingError::EmptyInput { index: 1 }));
}

/// Build a service at an explicit cap. Test-only: production always uses
/// [`MAX_INPUT_TOKENS`].
fn embed_with_cap(cache: &str, cap: usize, text: &str) -> Vec<f32> {
    let options = InitOptions::new(EmbeddingModel::NomicEmbedTextV15)
        .with_max_length(cap)
        .with_cache_dir(PathBuf::from(cache));
    let model = TextEmbedding::try_new(options).expect("the model loads");
    model
        .embed(vec![text], None)
        .expect("the text embeds")
        .into_iter()
        .next()
        .expect("one vector per text")
}
