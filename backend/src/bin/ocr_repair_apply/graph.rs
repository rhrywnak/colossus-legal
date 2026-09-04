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
            e.verbatim_quote AS quote, \
            e.verbatim_quote_ocr_original AS existing_original";

/// The only write in this repository's one-off bin family that touches
/// `verbatim_quote`. Five properties, named literally, and
/// `the_write_sets_only_the_five_authorised_properties` in the tests reads this
/// string to prove no sixth ever creeps in.
///
/// ## Why `coalesce` on the original, and only there
///
/// A repair round can correct a card the PREVIOUS round already corrected — v1a
/// fixes sixteen of v1's own 76. On that second write the "current" quote is not
/// the OCR text any more, it is v1's output, and storing it would overwrite the
/// only surviving copy of what Surya actually produced. `coalesce` keeps the
/// first value ever written and ignores every later one, so
/// `verbatim_quote_ocr_original` means "the text before ANY repair" no matter how
/// many rounds run. The other four are unconditional on purpose: the quote, the
/// status, the stamp and the timestamp all describe the LATEST round, and that is
/// what they should say.
pub const Q_WRITE: &str = "MATCH (e:Evidence {id: $id}) \
     SET e.verbatim_quote_ocr_original = \
             coalesce(e.verbatim_quote_ocr_original, $original), \
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

/// Verification read: THIS run's cards, by id. Asked instead of a corpus-wide
/// count because a corpus count cannot tell a second repair round from a first —
/// after v1a the stamp count is still 76 while the round wrote 16 — and a gate
/// that has to be re-derived per round is a gate that will be got wrong.
pub const Q_VERIFY_THIS_RUN: &str = "MATCH (e:Evidence) WHERE e.id IN $ids \
     RETURN count(e) AS matched, \
            count(CASE WHEN e.ocr_repair_source = $source THEN 1 END) AS stamped, \
            count(CASE WHEN e.verbatim_quote_ocr_original IS NOT NULL THEN 1 END) \
                AS originals";

/// Verification read three: every quote, for the B8 re-count.
pub const Q_ALL_QUOTES: &str =
    "MATCH (e:Evidence) RETURN e.id AS id, e.verbatim_quote AS quote ORDER BY id";

/// What one repaired card looked like, for the printed proof.
pub struct Line {
    pub id: String,
    pub page: i64,
    pub how: String,
    /// Whether this write STORED the pre-repair text or kept an earlier round's.
    pub original_action: &'static str,
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
            // best-effort: `page_number` is an OPTIONAL property — absent and
            // wrong-typed both mean "no usable page here", which is exactly the
            // `None` the guard then compares against the audit's page and STOPs
            // on. Nothing is swallowed: `Stop::WrongPage` carries the `None`.
            page_number: row.get::<i64>("page_number").ok(),
            quote: row
                .get("quote")
                .with_context(|| format!("node {id} carried no verbatim_quote"))?,
            // best-effort: absent is the NORMAL case — it means no earlier round
            // has touched this card — and it is reported per card, not swallowed.
            existing_original: row.get::<String>("existing_original").ok(),
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
        // Domain note: what `coalesce` will do is decided by what is already on
        // the node, so the decision is reported per card rather than left for the
        // operator to infer from a count at the end.
        let original_action = match found[0].existing_original {
            None => "orig SET (first repair of this card)",
            Some(_) => "orig KEPT (an earlier round already stored it)",
        };
        touched += write_node(txn, repair, &current).await?;
        lines.push(Line {
            id: repair.id.clone(),
            page: repair.page,
            how: repair.how.clone(),
            original_action,
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
            // Print the STOP BEFORE attempting the rollback. If the rollback
            // itself fails, its `?` would carry the rollback error away and the
            // guard failure — the card id, the two texts, the reason — would be
            // lost, leaving the operator a message they cannot act on. This is
            // the one place two errors can race, so the first one is emitted
            // where nothing can drop it.
            eprintln!("\nSTOP: {error:#}");
            txn.rollback().await.context(
                "the STOP above fired AND the rollback failed — the transaction may \
                 still be open. Check Neo4j for a hanging transaction before re-running",
            )?;
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

/// Read back exactly the cards this run wrote. Returns
/// `(matched, stamped, originals)`; all three must equal the run's card count.
pub async fn verify_this_run(graph: &Graph, ids: Vec<String>) -> Result<(i64, i64, i64)> {
    let mut stream = graph
        .execute(
            query(Q_VERIFY_THIS_RUN)
                .param("ids", ids)
                .param("source", crate::REPAIR_SOURCE),
        )
        .await
        .context("reading back the cards this run wrote")?;
    let row = stream
        .next()
        .await
        .context("reading the read-back row")?
        .context("the read-back query returned no row")?;
    Ok((
        row.get("matched")
            .context("read-back returned no matched")?,
        row.get("stamped")
            .context("read-back returned no stamped")?,
        row.get("originals")
            .context("read-back returned no originals")?,
    ))
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
             would write is unverified, so nothing was written. Check, in this order: \
             (1) NEO4J_URI points at the graph you meant — DEV, not PROD or an empty \
             local container; (2) the vocabulary still uses this word — run \
             `MATCH (e:Evidence) WHERE e.grounding_status IS NOT NULL \
             RETURN DISTINCT e.grounding_status` to see what the graph actually holds; \
             (3) something on this database has been hand-grounded at all.",
            crate::GROUNDING_STATUS
        ),
        Some(row) => {
            let id: String = row.get("id").context("the probe row carried no id")?;
            // best-effort: the probe is a PRINT, not a decision. Its job is to
            // show the operator a real hand-grounded card before any write; the
            // decision it feeds is `grounding_status`, read with `?` below. A
            // blank document line here degrades the display, never the write.
            let document: String = row.get("source_document").unwrap_or_default();
            // best-effort: same — display only. See the note above.
            let page = row.get::<i64>("page_number").ok();
            let status: String = row
                .get("grounding_status")
                .context("the probe row carried no grounding_status")?;
            // best-effort: display only. See the note above.
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
    let mut quoteless = Vec::new();
    while let Some(row) = stream.next().await.context("reading the next quote")? {
        let id: String = row.get("id").context("a quote row carried no id")?;
        // A node with no `verbatim_quote` at all and a node with an empty one are
        // different facts, and B8 cannot fire on either. Collapsing them into
        // "not damaged" would let a card with NO quote pass a verification read
        // silently — so they are collected and named instead.
        let quote: String = match row.get::<String>("quote") {
            Ok(text) => text,
            Err(_) => {
                quoteless.push(id);
                continue;
            }
        };
        if crate::model::has_ocr_damage(&quote) {
            flagged += 1;
            if expected.contains(id.as_str()) {
                still.push(id);
            }
        }
    }
    still.sort();
    if !quoteless.is_empty() {
        eprintln!(
            "WARNING: {} Evidence node(s) carry no readable verbatim_quote and could \
             not be tested for B8 damage. STOP 0 of EVIDENCE_CORPUS_READ_v1 found the \
             property on all 1,209 nodes, so this is new:",
            quoteless.len()
        );
        for id in &quoteless {
            eprintln!("    {id}");
        }
    }
    Ok((flagged, still))
}

#[cfg(test)]
#[path = "graph_tests.rs"]
mod tests;
