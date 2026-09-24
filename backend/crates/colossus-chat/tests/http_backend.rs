//! One real HTTP round trip: the Anthropic backend against a local mock server
//! that replays a recorded SSE transcript. Proves the headers, the body, the
//! streaming forward of prose, and the status classification — on a real socket,
//! with no call to Anthropic and no tokens spent.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use colossus_chat::transport::{RejectionKind, TransportError};
use colossus_chat::{AnthropicBackend, ChatBackend, EngineConfig};
use serde_json::json;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

const SSE: &str = concat!(
    "event: message_start\n",
    r#"data: {"type":"message_start","message":{"id":"m","usage":{"input_tokens":3,"cache_read_input_tokens":9}}}"#,
    "\n\n",
    r#"data: {"type":"content_block_start","index":0,"content_block":{"type":"text","text":""}}"#,
    "\n\n",
    r#"data: {"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"Hel"}}"#,
    "\n\n",
    r#"data: {"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"lo"}}"#,
    "\n\n",
    r#"data: {"type":"content_block_stop","index":0}"#,
    "\n\n",
    r#"data: {"type":"message_delta","delta":{"stop_reason":"end_turn"},"usage":{"output_tokens":2}}"#,
    "\n\n",
    r#"data: {"type":"message_stop"}"#,
    "\n\n",
);

/// Serve ONE connection: capture the raw request, answer with `status` + `body`.
async fn serve_once(
    status: &'static str,
    body: &'static str,
    ctype: &'static str,
) -> (String, Arc<Mutex<String>>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let seen = Arc::new(Mutex::new(String::new()));
    let seen2 = Arc::clone(&seen);
    tokio::spawn(async move {
        let (mut sock, _) = listener.accept().await.unwrap();
        let mut buf = vec![0u8; 65536];
        let mut got = Vec::new();
        // Read until the headers and the declared body have arrived.
        loop {
            let n = sock.read(&mut buf).await.unwrap();
            got.extend_from_slice(&buf[..n]);
            let text = String::from_utf8_lossy(&got).to_string();
            if let Some(h) = text.find("\r\n\r\n") {
                let len = text[..h]
                    .lines()
                    .find_map(|l| {
                        l.to_ascii_lowercase()
                            .strip_prefix("content-length:")
                            .map(|v| v.trim().parse::<usize>().unwrap())
                    })
                    .unwrap_or(0);
                if got.len() >= h + 4 + len {
                    *seen2.lock().unwrap() = text;
                    break;
                }
            }
            if n == 0 {
                break;
            }
        }
        let resp = format!(
            "HTTP/1.1 {status}\r\ncontent-type: {ctype}\r\ncontent-length: {}\r\nretry-after: 7\r\nconnection: close\r\n\r\n{body}",
            body.len()
        );
        sock.write_all(resp.as_bytes()).await.unwrap();
        sock.shutdown().await.unwrap();
    });
    (format!("http://{addr}"), seen)
}

fn config(base_url: String) -> EngineConfig {
    EngineConfig {
        base_url,
        api_key: "test-key".into(),
        api_version: "2023-06-01".into(),
        betas: vec!["compact-2026-01-12".into()],
        connect_timeout: Duration::from_secs(5),
        tcp_keepalive: Duration::from_secs(30),
        idle_timeout: Duration::from_secs(5),
        error_preview_chars: 500,
    }
}

#[tokio::test]
async fn a_streamed_reply_arrives_with_headers_and_forwarded_prose() {
    let (url, seen) = serve_once("200 OK", SSE, "text/event-stream").await;
    let backend = AnthropicBackend::new(config(url)).unwrap();
    let pieces = Mutex::new(Vec::new());
    let on_text = |t: String| pieces.lock().unwrap().push(t);
    let body = json!({"model":"m","max_tokens":1,"stream":true,"messages":[]});
    let reply = backend.call(&body, &on_text).await.unwrap();
    assert_eq!(reply.text(), "Hello");
    assert_eq!(reply.usage.cache_read_input_tokens, Some(9));
    assert_eq!(pieces.into_inner().unwrap().concat(), "Hello");
    let raw = seen.lock().unwrap().to_ascii_lowercase();
    assert!(raw.starts_with("post /v1/messages "));
    assert!(raw.contains("x-api-key: test-key"));
    assert!(raw.contains("anthropic-version: 2023-06-01"));
    assert!(raw.contains("anthropic-beta: compact-2026-01-12"));
    assert!(raw.contains("\"model\":\"m\""));
}

#[tokio::test]
async fn a_529_is_a_typed_rejection_with_retry_after() {
    let (url, _) = serve_once("529 Overloaded", r#"{"type":"error"}"#, "application/json").await;
    let backend = AnthropicBackend::new(config(url)).unwrap();
    let noop = |_t: String| {};
    let err = backend.call(&json!({}), &noop).await.unwrap_err();
    assert!(matches!(
        err,
        TransportError::Rejected {
            kind: RejectionKind::Overloaded,
            retry_after_secs: Some(7)
        }
    ));
}

#[tokio::test]
async fn a_400_carries_its_body() {
    let (url, _) = serve_once("400 Bad Request", r#"{"error":"bad"}"#, "application/json").await;
    let backend = AnthropicBackend::new(config(url)).unwrap();
    let noop = |_t: String| {};
    match backend.call(&json!({}), &noop).await.unwrap_err() {
        TransportError::Status { status, body } => {
            assert_eq!(status, 400);
            assert!(body.contains("bad"));
        }
        other => panic!("expected Status, got {other:?}"),
    }
}

/// A pre-warm is a non-streamed POST to the same endpoint, carrying the same
/// headers, whose JSON reply is read into `Usage`.
#[tokio::test]
async fn a_prewarm_round_trip_returns_its_usage() {
    let reply = r#"{"id":"m","content":[],"stop_reason":"max_tokens","usage":{"input_tokens":4,"cache_read_input_tokens":368832,"cache_creation_input_tokens":0,"output_tokens":0}}"#;
    let (url, seen) = serve_once("200 OK", reply, "application/json").await;
    let backend = AnthropicBackend::new(config(url)).unwrap();
    let body = json!({"model":"m","max_tokens":0,"messages":[]});
    let usage = backend.prewarm(&body).await.unwrap();
    assert_eq!(usage.cache_read_input_tokens, Some(368_832));
    assert_eq!(usage.cache_creation_input_tokens, Some(0));
    let raw = seen.lock().unwrap().to_ascii_lowercase();
    assert!(raw.starts_with("post /v1/messages "));
    assert!(raw.contains("anthropic-beta: compact-2026-01-12"));
    assert!(!raw.contains("accept: text/event-stream"));
    assert!(raw.contains("\"max_tokens\":0"));
}

/// A refused pre-warm keeps the provider's sentence for the log.
#[tokio::test]
async fn a_refused_prewarm_carries_its_body() {
    let (url, _) = serve_once("400 Bad Request", r#"{"error":"nope"}"#, "application/json").await;
    let backend = AnthropicBackend::new(config(url)).unwrap();
    match backend.prewarm(&json!({})).await.unwrap_err() {
        TransportError::Status { status, body } => {
            assert_eq!(status, 400);
            assert!(body.contains("nope"));
        }
        other => panic!("expected Status, got {other:?}"),
    }
}
