//! Small, total editors for streamed content blocks (`serde_json::Value`s).
//!
//! Split out of [`crate::accumulate`] to keep that module under Rule 17's 300
//! lines. Each helper is TOTAL: a block is always a JSON object (the accumulator
//! refuses anything without a `type`), so the non-object arms are unreachable —
//! they leave the value untouched rather than panicking.

use serde_json::Value;

use crate::accumulate::ChatStreamError;

/// A named malformed-payload error quoting at most `preview` characters.
pub(crate) fn malformed(reason: &str, payload: &str, preview: usize) -> ChatStreamError {
    ChatStreamError::Malformed {
        reason: reason.to_string(),
        preview: payload.chars().take(preview).collect(),
    }
}

/// The string field `key` of an event or block, or a named malformed error.
pub(crate) fn str_field<'a>(
    v: &'a Value,
    key: &str,
    payload: &str,
    preview: usize,
) -> Result<&'a str, ChatStreamError> {
    v.get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| malformed(&format!("missing string field `{key}`"), payload, preview))
}

/// The block index of an event, or a named malformed error.
pub(crate) fn index_field(
    v: &Value,
    payload: &str,
    preview: usize,
) -> Result<usize, ChatStreamError> {
    v.get("index")
        .and_then(Value::as_u64)
        // best-effort: an index too large for this platform's `usize` is folded
        // into "missing block index" — the named error below, never a silent drop.
        .and_then(|i| usize::try_from(i).ok())
        .ok_or_else(|| malformed("missing block index", payload, preview))
}

pub(crate) fn grow(v: &mut Vec<Option<Value>>, index: usize) {
    if v.len() <= index {
        v.resize_with(index + 1, || None);
    }
}

pub(crate) fn grow_strings(v: &mut Vec<String>, index: usize) {
    if v.len() <= index {
        v.resize_with(index + 1, String::new);
    }
}

/// Set a field on a block object. A block is always an object (the protocol
/// guarantees it, and `open` refused anything without a `type`), so a non-object
/// here is unreachable; it is simply left untouched rather than panicking.
pub(crate) fn set_field(block: &mut Value, key: &str, value: Value) {
    if let Some(obj) = block.as_object_mut() {
        obj.insert(key.to_string(), value);
    }
}

pub(crate) fn append_str(block: &mut Value, key: &str, text: &str) {
    if let Some(obj) = block.as_object_mut() {
        let current = obj
            .get(key)
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
        obj.insert(key.to_string(), Value::String(current + text));
    }
}

pub(crate) fn push_array(block: &mut Value, key: &str, item: Value) {
    if let Some(obj) = block.as_object_mut() {
        let entry = obj
            .entry(key.to_string())
            .or_insert_with(|| Value::Array(Vec::new()));
        // A `null` citations field (the start block may carry one) becomes a list.
        if !entry.is_array() {
            *entry = Value::Array(Vec::new());
        }
        if let Some(list) = entry.as_array_mut() {
            list.push(item);
        }
    }
}
