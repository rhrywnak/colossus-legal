//! Token accounting for one model call — including the cache columns the
//! extraction decoder never read.
//!
//! ## Domain note: why the cache counts matter here
//!
//! This engine's whole cost model rests on prompt caching: the document package
//! is written to the cache once (≈1.25× price) and then read back at ≈0.1× price on
//! every turn inside the TTL. If the reads silently stop happening — a byte changed
//! somewhere in the prefix — the bill multiplies by ten and nothing else looks
//! wrong. Recording `cache_read_input_tokens` on every turn is what makes that
//! failure visible without SQL spelunking (Standing Rule 1).

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Token counts as the provider reported them. `None` = not reported, which is a
/// different fact from `Some(0)`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Usage {
    /// Uncached input tokens (full price).
    pub input_tokens: Option<u64>,
    /// Output tokens (the final cumulative count).
    pub output_tokens: Option<u64>,
    /// Input tokens written to the cache on this call.
    pub cache_creation_input_tokens: Option<u64>,
    /// Input tokens served from the cache on this call.
    pub cache_read_input_tokens: Option<u64>,
}

impl Usage {
    /// Fold a `usage` object in. Later reports win field by field — the
    /// `message_delta` usage is the final word, `message_start` the first.
    pub fn absorb(&mut self, usage: &Value) {
        let read = |k: &str| usage.get(k).and_then(Value::as_u64);
        if let Some(v) = read("input_tokens") {
            self.input_tokens = Some(v);
        }
        if let Some(v) = read("output_tokens") {
            self.output_tokens = Some(v);
        }
        if let Some(v) = read("cache_creation_input_tokens") {
            self.cache_creation_input_tokens = Some(v);
        }
        if let Some(v) = read("cache_read_input_tokens") {
            self.cache_read_input_tokens = Some(v);
        }
    }

    /// Sum two calls' usage (the tool loop makes several calls per turn). A field
    /// stays `None` only when NEITHER call reported it.
    pub fn plus(&self, other: &Usage) -> Usage {
        fn add(a: Option<u64>, b: Option<u64>) -> Option<u64> {
            match (a, b) {
                (None, None) => None,
                (a, b) => Some(a.unwrap_or(0) + b.unwrap_or(0)),
            }
        }
        Usage {
            input_tokens: add(self.input_tokens, other.input_tokens),
            output_tokens: add(self.output_tokens, other.output_tokens),
            cache_creation_input_tokens: add(
                self.cache_creation_input_tokens,
                other.cache_creation_input_tokens,
            ),
            cache_read_input_tokens: add(
                self.cache_read_input_tokens,
                other.cache_read_input_tokens,
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn absorbs_cache_counts_and_later_reports_win() {
        let mut u = Usage::default();
        u.absorb(&serde_json::json!({"input_tokens": 10, "output_tokens": 1,
            "cache_creation_input_tokens": 0, "cache_read_input_tokens": 170000}));
        u.absorb(&serde_json::json!({"output_tokens": 55}));
        assert_eq!(u.input_tokens, Some(10));
        assert_eq!(u.output_tokens, Some(55));
        assert_eq!(u.cache_read_input_tokens, Some(170000));
        assert_eq!(u.cache_creation_input_tokens, Some(0));
    }

    #[test]
    fn plus_keeps_unreported_as_none() {
        let a = Usage {
            input_tokens: Some(3),
            ..Usage::default()
        };
        let b = Usage {
            input_tokens: Some(4),
            ..Usage::default()
        };
        let s = a.plus(&b);
        assert_eq!(s.input_tokens, Some(7));
        assert_eq!(s.cache_read_input_tokens, None);
    }
}
