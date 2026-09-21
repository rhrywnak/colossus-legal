//! Incremental `text/event-stream` framing — bytes in, whole `data:` payloads out.
//!
//! Moved here from `colossus-legal`'s `pipeline::anthropic_stream` (CC_TASK_CHAT_ENGINE_v1,
//! ruling D2) with no change in behavior, so the extraction engine and the chat engine
//! read the same wire through the same code. The framer is protocol, not policy: it
//! knows nothing about Anthropic's event types, only about SSE lines.

/// The SSE field prefix carrying an event's JSON payload.
///
/// CONST: the Server-Sent Events wire format (W3C `text/event-stream`), not a
/// setting. A deployment cannot choose a different spelling for it.
const SSE_DATA_FIELD: &str = "data:";

/// The one way framing can fail: a complete line that is not UTF-8.
///
/// Cannot happen against Anthropic, which sends JSON; surfaced rather than lossily
/// replaced so a mis-framed or corrupted stream is not silently turned into mojibake
/// that then fails much further downstream as a confusing parse error.
#[derive(Debug, thiserror::Error)]
#[error("a server-sent event line was not valid UTF-8: {source}")]
pub struct SseError {
    /// The underlying decode failure.
    #[source]
    pub source: std::str::Utf8Error,
}

/// Incremental `text/event-stream` framer.
///
/// Bytes arrive in arbitrary chunks that split lines and even split multi-byte
/// characters. The decoder buffers **bytes** and only decodes a line once its
/// terminating `\n` has arrived, so a UTF-8 sequence straddling a chunk boundary
/// is reassembled before anyone tries to read it.
///
/// ## Rust Learning: why the buffer is `Vec<u8>` and not `String`
///
/// `String` must be valid UTF-8 at all times, so pushing a half-decoded
/// character into one is not merely unwise — it will not compile without a
/// lossy conversion, and lossy is exactly what we must not do here (a `U+FFFD`
/// substituted into a document quote is a grounding failure much later, in a
/// place that gives no hint where it came from). Buffering raw bytes and
/// decoding whole lines keeps the failure at the boundary where it happened.
#[derive(Debug, Default)]
pub struct SseDecoder {
    /// Bytes received but not yet terminated by a newline.
    pending: Vec<u8>,
    /// `data:` field values collected for the event currently being framed.
    /// SSE allows several `data:` lines per event; they join with `\n`.
    data: String,
}

impl SseDecoder {
    /// Create an empty decoder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Feed the next chunk of socket bytes; return every COMPLETE event payload
    /// it completed, in order.
    ///
    /// An empty return is normal and meaningful: it means the chunk carried only
    /// part of a line, or only SSE fields we ignore (`event:`, `id:`, comments).
    /// It is NOT a failure and, importantly, it is NOT evidence of an idle
    /// stream — see [`crate::transport`] for why the idle clock is reset on
    /// decoded EVENTS rather than on received bytes.
    ///
    /// # Errors
    ///
    /// [`SseError`] if a complete line is not valid UTF-8.
    pub fn push_bytes(&mut self, chunk: &[u8]) -> Result<Vec<String>, SseError> {
        self.pending.extend_from_slice(chunk);
        let mut payloads = Vec::new();

        // ## Rust Learning: draining a buffer by index rather than by iterator
        //
        // We cannot iterate `self.pending` while also mutating it, so the loop
        // finds the next newline by position, splits the buffer with
        // `Vec::drain`, and repeats. `drain(..=idx)` removes the line AND its
        // terminator in one move and yields them; collecting into a `Vec<u8>`
        // ends the borrow before the next iteration begins.
        while let Some(idx) = self.pending.iter().position(|b| *b == b'\n') {
            let line: Vec<u8> = self.pending.drain(..=idx).collect();
            let line = std::str::from_utf8(&line).map_err(|source| SseError { source })?;
            // Trim the terminator in both framings — a `\r\n` stream is legal
            // SSE and a stray `\r` left on a JSON payload would fail the parse.
            let line = line.trim_end_matches('\n').trim_end_matches('\r');

            if line.is_empty() {
                // Blank line = end of event. An event with no `data:` field
                // (a bare `event:` or a keep-alive comment) yields nothing.
                if !self.data.is_empty() {
                    payloads.push(std::mem::take(&mut self.data));
                }
            } else if let Some(rest) = line.strip_prefix(SSE_DATA_FIELD) {
                if !self.data.is_empty() {
                    self.data.push('\n');
                }
                // The spec strips exactly ONE leading space after the colon.
                self.data.push_str(rest.strip_prefix(' ').unwrap_or(rest));
            }
            // Any other field (`event:`, `id:`, `retry:`, `:comment`) is
            // ignored: the payload JSON carries its own `type`, so the
            // `event:` line is redundant for us.
        }

        Ok(payloads)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_payload_split_across_chunks_is_reassembled() {
        let mut d = SseDecoder::new();
        assert!(d.push_bytes(b"data: {\"a\"").unwrap().is_empty());
        let out = d.push_bytes(b":1}\n\n").unwrap();
        assert_eq!(out, vec![r#"{"a":1}"#.to_string()]);
    }

    #[test]
    fn a_multibyte_character_split_across_chunks_survives() {
        let bytes = "data: é\n\n".as_bytes();
        let mut d = SseDecoder::new();
        // Split inside the two-byte `é`.
        assert!(d.push_bytes(&bytes[..7]).unwrap().is_empty());
        assert_eq!(d.push_bytes(&bytes[7..]).unwrap(), vec!["é".to_string()]);
    }

    #[test]
    fn crlf_framing_and_ignored_fields() {
        let mut d = SseDecoder::new();
        let out = d
            .push_bytes(b"event: ping\r\n: comment\r\ndata: x\r\n\r\n")
            .unwrap();
        assert_eq!(out, vec!["x".to_string()]);
    }

    #[test]
    fn invalid_utf8_is_a_named_error() {
        let mut d = SseDecoder::new();
        assert!(d.push_bytes(&[b'd', 0xff, b'\n']).is_err());
    }
}
