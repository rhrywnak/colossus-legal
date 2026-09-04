//! `ocr_repair_apply` — write the 76 corrected quotes from `OCR_REPAIR_v1` onto
//! their Evidence cards.
//!
//! **A one-off data repair, not a pipeline step.** Zero cost: no LLM, no
//! embedding, no paid API. It reads one audit file and changes five properties
//! on the cards that file names, inside ONE transaction, and only after every
//! card has proved it still holds the text the audit read.
//!
//! ## The shape of the guard
//!
//! Read, verify, write, count — per card, in one transaction:
//!
//! 1. exactly one `:Evidence` node carries the id, or STOP;
//! 2. its `source_document` and `page_number` are the ones the audit recorded,
//!    or STOP;
//! 3. its `verbatim_quote`, normalised for whitespace, equals the audit's
//!    `old_quote`, or STOP and print both texts;
//! 4. the write touches exactly the declared number of nodes, or ROLL BACK.
//!
//! Nothing is "fixed" that was not handed in. The old quote is kept on the node
//! in `verbatim_quote_ocr_original`, so the repair is reversible.
//!
//! ## Usage
//!
//! ```text
//! NEO4J_URI=bolt://HOST:7687 NEO4J_USER=neo4j NEO4J_PASSWORD=… \
//! cargo run --bin ocr_repair_apply -- \
//!   --input ~/Documents/colossus-legal/AUDITS/OCR_REPAIR_v1/OCR_REPAIR_v1.json \
//!   --expect-count 76 [--apply]
//! ```
//!
//! Without `--apply` it is a dry run: it does everything, prints everything, and
//! rolls back. Like `evidence_corpus_read` it does NOT read `.env` — a write
//! whose target depends on whatever a dotfile held is a write nobody can check.

mod graph;
mod model;

use anyhow::{bail, Context, Result};
use clap::Parser;
use std::path::PathBuf;
use std::process::ExitCode;

use colossus_legal_backend::oneshot::cli::connect_graph;
use model::RepairFile;

/// The grounding status a repaired card takes.
//
// STRUCTURAL: not a per-environment value. It is the status Roman's own
// hand-groundings already carry, and `graph::probe_manual` reads one of those
// nodes and prints it before any write, so the run PROVES the value it is about
// to store is one that already exists on DEV rather than one assumed here.
const GROUNDING_STATUS: &str = "manual";

/// The stamp that makes this run's 76 cards findable afterwards.
//
// STRUCTURAL: the identity of one specific audit, not a tuning knob. It is the
// name of the folder the input file comes from, and the verification read below
// counts by it; a second repair round would be a second audit with its own name.
const REPAIR_SOURCE: &str = "OCR_REPAIR_v1";

/// How much of a quote the proof lines show.
//
// STRUCTURAL: a terminal-width choice for a human watching one run, exactly as
// the instruction specifies it. No deployment wants a different number.
const PREVIEW_CHARS: usize = 60;

#[derive(Parser, Debug)]
#[command(about = "Write the OCR_REPAIR_v1 corrected quotes onto their Evidence cards.")]
struct Args {
    /// The audit file. Its `apply` array is the only thing written from.
    #[arg(long)]
    input: PathBuf,

    /// How many cards this run must repair. No default: stating it is how the
    /// operator declares which audit they are applying, and a file that has
    /// grown or shrunk since STOPs instead of quietly writing a different number.
    #[arg(long)]
    expect_count: usize,

    /// Commit. Absent — the default — every statement still runs and the
    /// transaction is rolled back.
    #[arg(long, default_value_t = false)]
    apply: bool,
}

#[tokio::main]
async fn main() -> ExitCode {
    if let Err(error) = run().await {
        eprintln!("\nocr_repair_apply STOPPED: {error:#}");
        eprintln!("Nothing was committed.");
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}

async fn run() -> Result<()> {
    let args = Args::parse();
    let file = load(&args.input)?;
    println!(
        "input {}\n  apply {} · false_alarm_dash_only {} · pending_missing_pdf {}",
        args.input.display(),
        file.apply.len(),
        file.false_alarm_dash_only.len(),
        file.pending_missing_pdf.len()
    );
    println!(
        "mode  {}\n",
        if args.apply {
            "--apply — the transaction WILL be committed"
        } else {
            "DRY RUN — every statement runs, nothing is committed"
        }
    );
    count_matches(file.apply.len() as i64, args.expect_count)
        .context("the input file's `apply` array is not the size the operator declared")?;

    // `connect_graph` reports the precise cause through `tracing::error!`, and
    // this bin starts no subscriber — so that text never reaches the terminal.
    // Rather than pull a subscriber into a one-off tool, the wrapper says what
    // the operator has to check, since there are only three possibilities.
    let graph = connect_graph().await.map_err(|code| {
        anyhow::anyhow!(
            "could not connect to Neo4j (exit {code:?}). This bin does NOT read .env: \
             export NEO4J_URI (bolt://HOST:7687), NEO4J_PASSWORD, and NEO4J_USER if it \
             is not `neo4j`, in the shell that runs it. Exit 2 means the connection \
             itself failed — check the URI's host and port are reachable."
        )
    })?;
    graph::probe_manual(&graph).await?;
    println!();

    let lines = graph::run_transaction(&graph, &file.apply, args.expect_count, args.apply).await?;
    print_lines(&lines);
    if args.apply {
        verify(&graph, &file).await?;
    } else {
        println!("\nDRY RUN — the transaction was rolled back. Re-run with --apply to commit.");
    }
    Ok(())
}

/// Read and parse the audit file.
///
/// A missing file, an unreadable file and a malformed file are three different
/// messages, because they need three different fixes.
fn load(path: &PathBuf) -> Result<RepairFile> {
    let text = std::fs::read_to_string(path)
        .with_context(|| format!("reading the audit file {}", path.display()))?;
    serde_json::from_str(&text)
        .with_context(|| format!("parsing {} as OCR_REPAIR_v1 JSON", path.display()))
}

/// The count gate, shared by the input check and the transaction check.
///
/// ## Rust Learning: why this returns `Result<()>` rather than `bool`
///
/// A `bool` would force every caller to write its own message, and standing
/// Rule 1 wants the message to name both numbers. Returning the error means
/// there is exactly one wording of it, and the test can assert on that wording.
pub fn count_matches(actual: i64, expected: usize) -> Result<()> {
    if actual != expected as i64 {
        bail!("expected exactly {expected} cards, got {actual} — nothing was committed");
    }
    Ok(())
}

/// First [`PREVIEW_CHARS`] characters, with the line breaks folded out so one
/// card prints on one line.
///
/// ## Rust Learning: `chars().take(n)` and not `&s[..n]`
///
/// Slicing by byte index panics in the middle of a multi-byte character, and
/// these quotes are full of curly apostrophes. Taking `char`s cannot panic.
pub fn preview(quote: &str) -> String {
    let flat = model::normalise(quote);
    let head: String = flat.chars().take(PREVIEW_CHARS).collect();
    if flat.chars().count() > PREVIEW_CHARS {
        format!("{head}…")
    } else {
        head
    }
}

/// The per-card proof, then the totals.
fn print_lines(lines: &[graph::Line]) {
    for line in lines {
        println!(
            "{} · p{} · {}\n    old  {}\n    new  {}",
            line.id, line.page, line.how, line.old_preview, line.new_preview
        );
    }
    let mut by_how: std::collections::BTreeMap<&str, usize> = std::collections::BTreeMap::new();
    for line in lines {
        *by_how.entry(line.how.as_str()).or_insert(0) += 1;
    }
    println!("\ncards verified and written: {}", lines.len());
    for (how, n) in &by_how {
        println!("  {how:<12} {n}");
    }
}

/// The four read-only checks the instruction asks for after a commit.
async fn verify(g: &neo4rs::Graph, file: &RepairFile) -> Result<()> {
    println!("\n--- verification (read-only) ---");
    let stamped =
        graph::count(g, graph::Q_COUNT_BY_SOURCE, Some(("source", REPAIR_SOURCE))).await?;
    println!("cards with ocr_repair_source = '{REPAIR_SOURCE}': {stamped}");
    count_matches(stamped, file.apply.len()).context("the stamp count is not the repair count")?;

    let originals = graph::count(g, graph::Q_COUNT_ORIGINALS, None).await?;
    println!("cards with verbatim_quote_ocr_original set:      {originals}");
    count_matches(originals, file.apply.len())
        .context("the kept-original count is not the repair count")?;

    let (flagged, still) = graph::recount_b8(g, &file.false_alarm_dash_only).await?;
    println!("\nB8 (the audit's three OCR signatures) over the whole corpus: {flagged}");
    println!(
        "  of the {} false_alarm_dash_only cards, {} still match",
        file.false_alarm_dash_only.len(),
        still.len()
    );
    for id in &still {
        println!("    {id}");
    }
    if still.len() != file.false_alarm_dash_only.len() {
        // Not a STOP — the write is already committed and this is a read. It is
        // a loud finding: a false-alarm card that stopped matching means someone
        // changed a quote this run was told not to touch.
        eprintln!(
            "WARNING: {} of the false_alarm_dash_only cards no longer match B8. \
             They contain `--` and should. Investigate before the next repair round.",
            file.false_alarm_dash_only.len() - still.len()
        );
    }
    Ok(())
}

#[cfg(test)]
mod main_tests {
    use super::*;

    #[test]
    fn preview_truncates_on_character_boundaries_and_flattens_newlines() {
        let quote = "her. to--to owed money some there\u{2019}s And COURT: THE\n\nestate. the on";
        let shown = preview(quote);
        assert!(!shown.contains('\n'));
        assert!(shown.ends_with('\u{2026}'));
        assert_eq!(shown.chars().count(), PREVIEW_CHARS + 1);
        // Short quotes are printed whole, with no ellipsis.
        assert_eq!(preview("  THE COURT: yes.  "), "THE COURT: yes.");
    }

    #[test]
    fn the_two_stamps_are_the_ones_the_instruction_names() {
        assert_eq!(GROUNDING_STATUS, "manual");
        assert_eq!(REPAIR_SOURCE, "OCR_REPAIR_v1");
    }
}
