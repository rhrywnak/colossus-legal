//! The one thin HTTP layer: readiness, and one batch of scores.
//!
//! Two calls, both against the vLLM server described by
//! `VLLM_MODEL_LIFECYCLE_HANDOFF_v1` §2–3. No test in this bin touches the
//! network; everything worth asserting lives in `pure`.

use anyhow::{bail, Context};
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// `GET /v1/models` — the readiness rule.
//
// serde: allows unknown fields because this is vLLM's response, not ours. It
// carries `object`, `created`, `owned_by` and whatever a later release adds, and
// refusing to parse a body because the server grew a field would turn a working
// reranker into STOP 1 on somebody else's upgrade. The fields we DO need are
// required, so a body that lost `data` is still an error.
#[derive(Debug, Deserialize)]
struct ModelList {
    data: Vec<ModelEntry>,
}

// serde: allows unknown fields — same vLLM body as ModelList above.
#[derive(Debug, Deserialize)]
struct ModelEntry {
    id: String,
}

/// `POST /score` request body — the hand-off's exact shape.
#[derive(Debug, Serialize)]
struct ScoreRequest<'a> {
    model: &'a str,
    text_1: &'a str,
    text_2: &'a [String],
}

// serde: allows unknown fields because /score returns `object`, `model`, `usage`
// and an `id`, none of which this gate reads. Only `data` matters and it is
// required.
#[derive(Debug, Deserialize)]
struct ScoreResponse {
    data: Vec<ScoreEntry>,
}

// serde: allows unknown fields — an entry may carry `object` alongside the two
// values the gate needs, both of which are required here.
#[derive(Debug, Deserialize)]
struct ScoreEntry {
    /// Position in the `text_2` array that was sent — NOT a rank.
    index: usize,
    score: f64,
}

/// A client bound to one base URL and one model id.
pub struct RerankClient {
    http: reqwest::Client,
    base_url: String,
    model: String,
}

impl RerankClient {
    /// Build the client.
    ///
    /// ## Rust Learning: the builder is not optional here
    ///
    /// `reqwest::Client::new()` has NO request timeout — a hung server would
    /// park this bin forever with nothing in the log. Standing rule 13 requires
    /// both a total timeout and a connect timeout on every HTTP client in this
    /// repo, and one client is built and reused for the whole run rather than
    /// one per batch, so the connection pool survives all 292 pairs.
    pub fn new(base_url: &str, model: &str, timeout_secs: u64) -> anyhow::Result<Self> {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(timeout_secs))
            // DEFAULT: five seconds to establish the TCP connection, against the
            // caller-supplied `timeout_secs` for the whole request. Separate on
            // purpose — scoring 292 pairs can legitimately take a minute, but a
            // reranker that has not accepted a CONNECTION in five seconds is not
            // slow, it is absent, and the operator should be told so while they
            // are still watching. Override the REQUEST budget with the CLI flag;
            // this one has no knob because no deployment wants a longer wait to
            // discover a host is down.
            .connect_timeout(Duration::from_secs(5))
            .build()
            .context("building the reranker HTTP client")?;
        Ok(Self {
            http,
            base_url: base_url.trim_end_matches('/').to_string(),
            model: model.to_string(),
        })
    }

    /// STOP 1 and STOP 2: the server answers, and it is serving THIS model.
    ///
    /// The hand-off is explicit that `/health` is not enough — a vLLM that is up
    /// with a different model loaded is healthy and wrong. Returns every id the
    /// server lists, so the caller can print what it actually found rather than
    /// only that the match failed.
    pub async fn readiness(&self) -> anyhow::Result<Vec<String>> {
        let url = format!("{}/v1/models", self.base_url);
        let response = self
            .http
            .get(&url)
            .send()
            .await
            .with_context(|| format!("STOP 1 — GET {url} did not answer"))?;
        let status = response.status();
        let body = response
            .text()
            .await
            .with_context(|| format!("STOP 1 — reading the body of GET {url}"))?;
        if !status.is_success() {
            bail!("STOP 1 — GET {url} returned HTTP {status}: {body}");
        }
        let list: ModelList = serde_json::from_str(&body)
            .with_context(|| format!("STOP 1 — GET {url} returned unparseable JSON: {body}"))?;
        Ok(list.data.into_iter().map(|m| m.id).collect())
    }

    /// Score one batch of candidate texts against one query.
    ///
    /// Returns the scores in the SAME ORDER as `texts`, having proved the server
    /// returned one score for every text sent (STOP 4).
    pub async fn score_batch(&self, query: &str, texts: &[String]) -> anyhow::Result<Vec<f64>> {
        let url = format!("{}/score", self.base_url);
        let request = ScoreRequest {
            model: &self.model,
            text_1: query,
            text_2: texts,
        };
        let response = self
            .http
            .post(&url)
            .json(&request)
            .send()
            .await
            .with_context(|| format!("POST {url} with {} texts", texts.len()))?;
        let status = response.status();
        let body = response
            .text()
            .await
            .with_context(|| format!("reading the body of POST {url}"))?;
        if !status.is_success() {
            bail!("POST {url} returned HTTP {status}: {body}");
        }
        let parsed: ScoreResponse = serde_json::from_str(&body)
            .with_context(|| format!("POST {url} returned unparseable JSON: {body}"))?;
        reorder_by_index(parsed.data, texts.len())
    }
}

/// STOP 4: turn the server's `{index, score}` list into a dense score vector.
///
/// Refuses — rather than pads or skips — on any of: a count that differs from
/// the batch sent, an index outside the batch, a duplicated index, a missing
/// index, or a non-finite score. Every one of those would otherwise silently
/// misalign a score with the wrong card, and a misaligned score is a wrong gate
/// verdict that looks exactly like a right one.
fn reorder_by_index(entries: Vec<ScoreEntry>, expected: usize) -> anyhow::Result<Vec<f64>> {
    if entries.len() != expected {
        bail!(
            "STOP 4 — sent {expected} texts, the server returned {} scores",
            entries.len()
        );
    }
    let mut scores: Vec<Option<f64>> = vec![None; expected];
    for entry in entries {
        let slot = scores.get_mut(entry.index).ok_or_else(|| {
            anyhow::anyhow!(
                "STOP 4 — the server returned index {} for a batch of {expected}",
                entry.index
            )
        })?;
        if slot.is_some() {
            bail!("STOP 4 — the server returned index {} twice", entry.index);
        }
        if !entry.score.is_finite() {
            bail!(
                "STOP 4 — the server returned a non-finite score {} at index {}",
                entry.score,
                entry.index
            );
        }
        *slot = Some(entry.score);
    }
    scores
        .into_iter()
        .enumerate()
        .map(|(i, s)| s.ok_or_else(|| anyhow::anyhow!("STOP 4 — no score returned for index {i}")))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(index: usize, score: f64) -> ScoreEntry {
        ScoreEntry { index, score }
    }

    /// The happy path, and the thing that makes this function necessary:
    /// the server may answer OUT OF ORDER, and `index` — not position — says
    /// which text a score belongs to. Returning them in arrival order would
    /// silently attribute every score to the wrong candidate.
    #[test]
    fn scores_come_back_in_the_order_the_texts_were_sent() {
        let scores = reorder_by_index(vec![entry(2, 0.3), entry(0, 0.9), entry(1, 0.5)], 3)
            .expect("a complete, well-formed batch");
        assert_eq!(scores, vec![0.9, 0.5, 0.3]);
    }

    #[test]
    fn a_short_or_long_batch_stops() {
        let short = reorder_by_index(vec![entry(0, 0.1)], 2).expect_err("a short batch must STOP");
        assert!(short.to_string().contains("STOP 4"));
        assert!(short.to_string().contains("sent 2 texts"));

        let long = reorder_by_index(vec![entry(0, 0.1), entry(1, 0.2)], 1)
            .expect_err("a long batch must STOP");
        assert!(long.to_string().contains("STOP 4"));
    }

    #[test]
    fn an_index_past_the_end_of_the_batch_stops() {
        let error = reorder_by_index(vec![entry(0, 0.1), entry(9, 0.2)], 2)
            .expect_err("an out-of-range index must STOP");
        assert!(error.to_string().contains("returned index 9"));
    }

    #[test]
    fn the_same_index_twice_stops() {
        let error = reorder_by_index(vec![entry(1, 0.1), entry(1, 0.2)], 2)
            .expect_err("a duplicate index must STOP");
        assert!(error.to_string().contains("index 1 twice"));
    }

    /// NaN and the infinities, each separately. A non-finite score does not
    /// merely rank wrongly — it makes the whole ordering undefined, because
    /// every comparison against NaN is false.
    #[test]
    fn a_non_finite_score_stops() {
        for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            let error = reorder_by_index(vec![entry(0, 0.1), entry(1, bad)], 2)
                .expect_err("a non-finite score must STOP");
            assert!(
                error.to_string().contains("non-finite"),
                "got: {error} for {bad}"
            );
        }
    }

    /// The count is right and an index is still missing — which is only
    /// reachable together with a duplicate, and is caught by the duplicate
    /// check first. The final `ok_or_else` is the belt to that braces, and this
    /// pins the message it would produce.
    #[test]
    fn a_gap_is_unreachable_without_a_duplicate_and_both_are_caught() {
        let error = reorder_by_index(vec![entry(0, 0.1), entry(0, 0.2)], 2)
            .expect_err("index 1 never arrived");
        let rendered = error.to_string();
        assert!(
            rendered.contains("twice") || rendered.contains("no score returned for index 1"),
            "got: {rendered}"
        );
    }
}
