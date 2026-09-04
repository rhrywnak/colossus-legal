//! Every statement this bin sends to Neo4j, and the one transaction that runs
//! them.
//!
//! The queries are `pub const` for the same reason `evidence_corpus_read`'s are:
//! the report has to be able to print the exact text that produced each number,
//! and a query retyped into a document is a query that can drift from the one
//! that ran.

use anyhow::{bail, Context, Result};
use neo4rs::{query, Graph, Txn};

use crate::model::{guard, NodeState, Repair, Untouched};

/// Read one card. Returns EVERY matching row so "none" and "two" stay different
/// facts — the guard, not this function, decides that only one is acceptable.
pub const Q_READ: &str = "MATCH (e:Evidence {id: $id}) \
     RETURN e.source_document AS source_document, e.page_number AS page_number, \
            e.verbatim_quote AS quote";

/// The only write in this repository's one-off bin family that touches
/// `verbatim_quote`. Five properties, named literally, and `set_only_five_named_properties`
/// in the tests reads this string to prove no sixth ever creeps in.
pub const Q_WRITE: &str = "MATCH (e:Evidence {id: $id}) \
     SET e.verbatim_quote_ocr_original = $original, \
         e.verbatim_quote = $new_quote, \
         e.grounding_status = $status, \
         e.ocr_repaired_at = datetime(), \
         e.ocr_repair_source = $source \
     RETURN count(e) AS n";

/// The probe the instruction requires before any write: read an existing
/// hand-grounded node and print it, so the value written into
/// `grounding_status` is one observed on DEV rather than one assumed.
pub const Q_MANUAL_PROBE: &str = "MATCH (e:Evidence) WHERE e.grounding_status = $status \
     RETURN e.id AS id, e.source_document AS source_document, e.page_number AS page_number, \
            e.grounding_status AS grounding_status, keys(e) AS keys \
     ORDER BY id LIMIT 1";

/// Verification read one: how many cards carry this run's stamp.
pub const Q_COUNT_BY_SOURCE: &str =
    "MATCH (e:Evidence) WHERE e.ocr_repair_source = $source RETURN count(e) AS n";

/// Verification read two: how many cards kept a copy of the OCR original.
pub const Q_COUNT_ORIGINALS: &str =
    "MATCH (e:Evidence) WHERE e.verbatim_quote_ocr_original IS NOT NULL RETURN count(e) AS n";

/// Verification read three: every quote, for the B8 re-count.
pub const Q_ALL_QUOTES: &str =
    "MATCH (e:Evidence) RETURN e.id AS id, e.verbatim_quote AS quote ORDER BY id";

/// What one repaired card looked like, for the printed proof.
pub struct Line {
    pub id: String,
    pub page: i64,
    pub how: String,
    pub old_preview: String,
    pub new_preview: String,
}

/// Read one card inside the transaction.
///
/// ## Rust Learning: a `RowStream` and its transaction handle
///
/// `Txn::execute` hands back an owned `RowStream`, but the rows still have to be
/// pulled over the transaction's connection — which is why `next` takes
/// `txn.handle()`. Two mutable borrows of `txn` would not compile if the stream
/// borrowed it; because it does not, the loop below is legal and the read stays
/// inside the same transaction as the write that follows it.
pub async fn read_node(txn: &mut Txn, id: &str) -> Result<Vec<NodeState>> {
    let mut stream = txn
        .execute(query(Q_READ).param("id", id))
        .await
        .with_context(|| format!("reading Evidence node {id}"))?;
    let mut out = Vec::new();
    while let Some(row) = stream
        .next(txn.handle())
        .await
        .with_context(|| format!("reading the next row for Evidence node {id}"))?
    {
        out.push(NodeState {
            source_document: row
                .get("source_document")
                .with_context(|| format!("node {id} carried no source_document"))?,
            page_number: row.get::<i64>("page_number").ok(),
            quote: row
                .get("quote")
                .with_context(|| format!("node {id} carried no verbatim_quote"))?,
        });
    }
    Ok(out)
}

/// Write one card inside the transaction; returns how many nodes it touched.
pub async fn write_node(txn: &mut Txn, repair: &Repair, original: &str) -> Result<i64> {
    let mut stream = txn
        .execute(
            query(Q_WRITE)
                .param("id", repair.id.as_str())
                .param("original", original)
                .param("new_quote", repair.new_quote.as_str())
                .param("status", crate::GROUNDING_STATUS)
                .param("source", crate::REPAIR_SOURCE),
        )
        .await
        .with_context(|| format!("writing the corrected quote onto {}", repair.id))?;
    let row = stream
        .next(txn.handle())
        .await
        .with_context(|| format!("reading the write count for {}", repair.id))?
        .with_context(|| format!("the write for {} returned no count row", repair.id))?;
    row.get("n")
        .with_context(|| format!("the write for {} returned a row with no `n`", repair.id))
}

/// Read-verify-write every card, and check the total. Called inside the
/// transaction; any `Err` here means the caller rolls back.
async fn write_all(txn: &mut Txn, repairs: &[Repair], expect: usize) -> Result<Vec<Line>> {
    let mut lines = Vec::with_capacity(repairs.len());
    let mut touched: i64 = 0;
    for repair in repairs {
        let found = read_node(txn, &repair.id).await?;
        if let Err(stop) = guard(repair, &found) {
            bail!(stop);
        }
        // `guard` proved there is exactly one row, so indexing is safe here and
        // this is the one place the invariant is relied on rather than re-tested.
        let current = found[0].quote.clone();
        touched += write_node(txn, repair, &current).await?;
        lines.push(Line {
            id: repair.id.clone(),
            page: repair.page,
            how: repair.how.clone(),
            old_preview: crate::preview(&repair.old_quote),
            new_preview: crate::preview(&repair.new_quote),
        });
    }
    crate::count_matches(touched, expect)?;
    Ok(lines)
}

/// Open the transaction, run every card through it, then commit or roll back.
///
/// ## Why the dry run takes exactly this path
///
/// A dry run that only READ would prove the guard passes and nothing else. This
/// one issues the real `SET`s and then rolls back, so the dry run also proves
/// the write statement itself is accepted by the server and touches the number
/// of nodes it claims — and still leaves DEV untouched.
pub async fn run_transaction(
    graph: &Graph,
    repairs: &[Repair],
    expect: usize,
    apply: bool,
) -> Result<Vec<Line>> {
    let mut txn = graph
        .start_txn()
        .await
        .context("opening the repair transaction")?;
    match write_all(&mut txn, repairs, expect).await {
        Err(error) => {
            txn.rollback()
                .await
                .context("rolling back after a STOP — the transaction may still be open")?;
            Err(error)
        }
        Ok(lines) => {
            if apply {
                txn.commit().await.context("committing the repair")?;
            } else {
                txn.rollback()
                    .await
                    .context("rolling back the dry run — nothing should have been written")?;
            }
            Ok(lines)
        }
    }
}

/// Run a single-number read outside any transaction.
pub async fn count(graph: &Graph, cypher: &str, param: Option<(&str, &str)>) -> Result<i64> {
    let mut q = query(cypher);
    if let Some((key, value)) = param {
        q = q.param(key, value);
    }
    let mut stream = graph
        .execute(q)
        .await
        .with_context(|| format!("running verification query: {cypher}"))?;
    let row = stream
        .next()
        .await
        .context("reading the verification row")?
        .with_context(|| format!("verification query returned no row: {cypher}"))?;
    row.get("n")
        .with_context(|| format!("verification query returned no `n`: {cypher}"))
}

/// Print the properties of one existing hand-grounded card.
pub async fn probe_manual(graph: &Graph) -> Result<()> {
    let mut stream = graph
        .execute(query(Q_MANUAL_PROBE).param("status", crate::GROUNDING_STATUS))
        .await
        .context("probing an existing hand-grounded node")?;
    match stream.next().await.context("reading the probe row")? {
        None => bail!(
            "no Evidence node carries grounding_status = '{}'. The value this run \
             would write is unverified, so nothing was written.",
            crate::GROUNDING_STATUS
        ),
        Some(row) => {
            let id: String = row.get("id").context("the probe row carried no id")?;
            let document: String = row.get("source_document").unwrap_or_default();
            let page = row.get::<i64>("page_number").ok();
            let status: String = row
                .get("grounding_status")
                .context("the probe row carried no grounding_status")?;
            let keys: Vec<String> = row.get("keys").unwrap_or_default();
            println!("probe — an existing hand-grounded card:");
            println!("  id               {id}");
            println!("  source_document  {document}");
            println!("  page_number      {page:?}");
            println!("  grounding_status {status:?}");
            println!("  keys             {}", keys.join(", "));
            Ok(())
        }
    }
}

/// Read every quote and re-run the B8 rule over it.
///
/// Returns the total still flagged, and how many of the audit's
/// `false_alarm_dash_only` ids are among them — which should be all of them,
/// because `--` is how the court reporter writes an interruption and those
/// cards are correct as stored.
pub async fn recount_b8(graph: &Graph, false_alarms: &[Untouched]) -> Result<(usize, Vec<String>)> {
    let mut stream = graph
        .execute(query(Q_ALL_QUOTES))
        .await
        .context("reading every quote for the B8 re-count")?;
    let expected: std::collections::HashSet<&str> =
        false_alarms.iter().map(|c| c.id.as_str()).collect();
    let (mut flagged, mut still) = (0usize, Vec::new());
    while let Some(row) = stream.next().await.context("reading the next quote")? {
        let id: String = row.get("id").context("a quote row carried no id")?;
        let quote: String = row.get("quote").unwrap_or_default();
        if crate::model::has_ocr_damage(&quote) {
            flagged += 1;
            if expected.contains(id.as_str()) {
                still.push(id);
            }
        }
    }
    still.sort();
    Ok((flagged, still))
}

#[cfg(test)]
#[path = "graph_tests.rs"]
mod tests;
