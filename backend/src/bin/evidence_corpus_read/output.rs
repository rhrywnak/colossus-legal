//! The three artefacts: `summary.md`, `cards.csv`, `queries.md`.

use anyhow::{Context, Result};
use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::Path;

use crate::buckets::{overlap_matrix, DuplicateIndex};
use crate::model::{Card, Flags, BUCKETS};
use crate::norm::char_len;

/// Escape one CSV field — RFC 4180, every field quoted unconditionally.
fn field(text: &str) -> String {
    format!("\"{}\"", text.replace('"', "\"\""))
}

/// First `max` characters, newlines flattened.
fn head(text: &str, max: usize) -> String {
    text.split_whitespace()
        .collect::<Vec<&str>>()
        .join(" ")
        .chars()
        .take(max)
        .collect()
}

/// `cards.csv` — one row per card, all twelve flags.
pub fn write_cards_csv(path: &Path, cards: &[Card], flags: &[Flags]) -> Result<usize> {
    let mut body = String::from(
        "evidence_id,document,page,template_file,grounding_status,quote_len,\
         B1,B2,B3,B4,B5,B6,B7,B8,B9,B10,B11,B12,quote_head,title_head\n",
    );
    for (card, flag) in cards.iter().zip(flags.iter()) {
        let digits: Vec<String> = (0..BUCKETS.len())
            .map(|i| flag.digit(i).to_string())
            .collect();
        let _ = writeln!(
            body,
            "{},{},{},{},{},{},{},{},{}",
            field(&card.id),
            field(&card.source_document),
            card.page_number.map(|p| p.to_string()).unwrap_or_default(),
            field(card.template_name.as_deref().unwrap_or("")),
            field(&card.grounding_status),
            char_len(&card.quote),
            digits.join(","),
            field(&head(&card.quote, 80)),
            field(&head(&card.title, 80))
        );
    }
    std::fs::write(path, &body).with_context(|| format!("writing {}", path.display()))?;
    Ok(cards.len() + 1)
}

/// Count cards per key, for the by-template and by-document breakdowns.
fn tally<'a, F>(cards: &'a [Card], key: F) -> BTreeMap<String, usize>
where
    F: Fn(&'a Card) -> String,
{
    let mut out: BTreeMap<String, usize> = BTreeMap::new();
    for card in cards {
        *out.entry(key(card)).or_insert(0) += 1;
    }
    out
}

/// The bucket table — count, share of the corpus, and what the number means.
fn bucket_table(total: usize, flags: &[Flags]) -> String {
    let mut out = String::from("| Bucket | Count | % of corpus | Meaning |\n|---|---:|---:|---|\n");
    for (index, (code, meaning)) in BUCKETS.iter().enumerate() {
        let n = flags.iter().filter(|f| f.0[index]).count();
        let pct = if total > 0 {
            (n as f64) * 100.0 / (total as f64)
        } else {
            0.0
        };
        let _ = writeln!(out, "| **{code}** | {n} | {pct:.1}% | {meaning} |");
    }
    out
}

/// The overlap matrix, rendered as a markdown grid.
fn overlap_table(flags: &[Flags]) -> String {
    let matrix = overlap_matrix(flags);
    let mut out = String::from("|  |");
    for (code, _) in BUCKETS.iter() {
        let _ = write!(out, " {code} |");
    }
    out.push_str("\n|---|");
    out.push_str(&"---:|".repeat(BUCKETS.len()));
    out.push('\n');
    for ((code, _), row) in BUCKETS.iter().zip(matrix.iter()) {
        let _ = write!(out, "| **{code}** |");
        for cell in row {
            let _ = write!(out, " {cell} |");
        }
        out.push('\n');
    }
    out
}

/// Everything the summary needs that is not a card.
pub struct SummaryContext<'a> {
    pub total: usize,
    pub answer_tokens: &'a [String],
    pub short_quote_survey: &'a [(String, usize)],
    /// The `--short-quote-chars` value this run actually used. Carried rather
    /// than assumed: the heading in `summary.md` states the rule that produced
    /// the token set below it, and a heading that says 25 while the operator
    /// passed 30 describes an audit nobody ran.
    pub short_quote_chars: usize,
    pub dropped_statement_types: &'a [String],
    pub near_ratio: f64,
    pub mirror_note: &'a str,
    pub b1_with_question: usize,
    pub duplicates: &'a DuplicateIndex,
    pub twin_count: usize,
    pub cross_ref_count: usize,
}

/// `summary.md` — the document Roman reads.
pub fn write_summary(
    path: &Path,
    cards: &[Card],
    flags: &[Flags],
    ctx: &SummaryContext<'_>,
) -> Result<()> {
    let mut out = String::new();
    let _ = writeln!(out, "# Evidence corpus read — {} cards\n", ctx.total);
    let _ = writeln!(
        out,
        "Read-only census of every `Evidence` node on DEV. No card was changed, \
         deleted or re-extracted. This document counts; it recommends nothing.\n"
    );

    let _ = writeln!(out, "## Buckets\n");
    out.push_str(&bucket_table(ctx.total, flags));

    let clean = flags.iter().filter(|f| f.clean()).count();
    let _ = writeln!(
        out,
        "\n**Clean by these rules: {clean}** of {} ({:.1}%).\n",
        ctx.total,
        (clean as f64) * 100.0 / (ctx.total.max(1) as f64)
    );

    let _ = writeln!(out, "## How the contested rules were set\n");
    let _ = writeln!(
        out,
        "- **B1 answer tokens** (derived from the corpus, not assumed): {}\n",
        ctx.answer_tokens
            .iter()
            .map(|t| format!("`{t}`"))
            .collect::<Vec<String>>()
            .join(", ")
    );
    let _ = writeln!(
        out,
        "- Of the B1 cards, **{}** carry a non-blank `question` — recoverable by \
         reading the field, not by re-extraction.",
        ctx.b1_with_question
    );
    let _ = writeln!(
        out,
        "- **B3 near-duplicate rule**: one normalised quote is a prefix or suffix \
         of the other, and the shorter is ≥ {:.0}% of the longer's length.",
        ctx.near_ratio * 100.0
    );
    let _ = writeln!(
        out,
        "- **B7** reuses `theme_scan_prefilter::dropped_kind` against the stored \
         `theme_scan_prefilter_statement_types` list: {:?}.",
        ctx.dropped_statement_types
    );
    let _ = writeln!(out, "- **B12**: {}\n", ctx.mirror_note);

    let _ = writeln!(
        out,
        "### Every distinct quote under {} characters\n",
        ctx.short_quote_chars
    );
    let _ = writeln!(out, "| Quote | Cards |\n|---|---:|");
    for (quote, n) in ctx.short_quote_survey {
        let _ = writeln!(out, "| `{}` | {n} |", quote.replace('|', "\\|"));
    }

    let _ = writeln!(out, "\n## B2 — duplicate clusters\n");
    let _ = writeln!(
        out,
        "{} clusters covering {} cards. True twins (same document AND page): \
         **{}**. Same quote across DIFFERENT documents: **{}** — that class is a \
         cross-reference, not damage.\n",
        ctx.duplicates.clusters.len(),
        ctx.duplicates.card_count(),
        ctx.twin_count,
        ctx.cross_ref_count
    );
    let _ = writeln!(out, "| Cards | Documents | Quote |\n|---:|---|---|");
    for (quote, members) in ctx.duplicates.largest(10) {
        let docs: Vec<&str> = {
            let mut d: Vec<&str> = members
                .iter()
                .map(|&i| cards[i].source_document.as_str())
                .collect();
            d.sort_unstable();
            d.dedup();
            d
        };
        let _ = writeln!(
            out,
            "| {} | {} | `{}` |",
            members.len(),
            docs.join(", "),
            head(quote, 70).replace('|', "\\|")
        );
    }

    let _ = writeln!(out, "\n## Overlap matrix\n");
    out.push_str(&overlap_table(flags));

    let _ = writeln!(out, "\n## By extraction template\n");
    out.push_str(&template_table(cards, flags));

    let _ = writeln!(out, "\n## By grounding status\n");
    let _ = writeln!(out, "| Status | Cards |\n|---|---:|");
    for (status, n) in tally(cards, |c| c.grounding_status.clone()) {
        let _ = writeln!(out, "| {status} | {n} |");
    }

    std::fs::write(path, &out).with_context(|| format!("writing {}", path.display()))?;
    Ok(())
}

/// B1 and B4 broken down by template version — the "did the newer templates fix
/// it" question the instruction asks.
fn template_table(cards: &[Card], flags: &[Flags]) -> String {
    let mut rows: BTreeMap<String, (usize, usize, usize)> = BTreeMap::new();
    for (card, flag) in cards.iter().zip(flags.iter()) {
        let key = card
            .template_name
            .clone()
            .unwrap_or_else(|| "(no provenance row)".to_string());
        let entry = rows.entry(key).or_insert((0, 0, 0));
        entry.0 += 1;
        entry.1 += usize::from(flag.0[0]);
        entry.2 += usize::from(flag.0[3]);
    }
    let mut out = String::from(
        "| Template | Cards | B1 no text | B4 grounding suspect |\n|---|---:|---:|---:|\n",
    );
    for (template, (total, b1, b4)) in rows {
        let _ = writeln!(out, "| {template} | {total} | {b1} | {b4} |");
    }
    out
}

/// `queries.md` — every query verbatim, with the count it returned.
pub fn write_queries(path: &Path, entries: &[(&str, String, String)]) -> Result<()> {
    let mut out = String::from(
        "# Queries\n\nEvery statement this audit ran, in bucket order, with what it \
         returned. All reads: no `CREATE`, `MERGE`, `SET`, `DELETE`, `INSERT`, \
         `UPDATE` or DDL was issued against either store.\n\n",
    );
    for (label, sql, result) in entries {
        let _ = writeln!(out, "## {label}\n\n```\n{}\n```\n\n{result}\n", sql.trim());
    }
    std::fs::write(path, &out).with_context(|| format!("writing {}", path.display()))?;
    Ok(())
}
