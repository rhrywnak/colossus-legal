//! The per-fixture CSV — one row per card, ranked by the verdict surface.
//!
//! Split out of `report` purely for size (standing Rule 17): the printed block
//! and the CSV render the same finished numbers and share no logic.

use crate::fixture::{Card, Fixture};
use crate::report::SurfaceRun;

/// Escape one CSV field — RFC 4180: wrap in quotes, double any inner quote.
///
/// Quotes and significances carry commas, embedded quotation marks and hard
/// newlines, so every field is quoted unconditionally rather than conditionally;
/// a rule with no exceptions cannot be applied inconsistently.
fn csv_field(text: &str) -> String {
    format!("\"{}\"", text.replace('"', "\"\""))
}

/// Write the per-fixture CSV. Returns the number of lines written (header included).
pub fn write_csv(
    path: &std::path::Path,
    fixture: &Fixture,
    runs: &[&SurfaceRun; 3],
    relevant: &[usize],
    included: &[usize],
) -> anyhow::Result<usize> {
    let mut rows: Vec<(usize, String)> = Vec::new();
    for (i, card) in fixture.candidates.iter().enumerate() {
        let flags = (
            usize::from(relevant.contains(&i)),
            usize::from(included.contains(&i)),
            0usize,
        );
        rows.push(csv_row(card, runs, i, false, flags));
    }
    for (i, card) in fixture.outside_pool.iter().enumerate() {
        rows.push(csv_row(card, runs, i, true, (0, 0, 1)));
    }
    rows.sort_by_key(|(rank, _)| *rank);

    let mut body = String::from(
        "rank_S2,score_S2,rank_S1,score_S1,rank_S3,score_S3,id,c_number,opus_relevant,included,outside_pool,document,pinpoint,title\n",
    );
    for (_, line) in &rows {
        body.push_str(line);
        body.push('\n');
    }
    std::fs::write(path, &body)?;
    Ok(rows.len() + 1)
}

/// One CSV line, plus its S2 rank so the caller can sort by it.
fn csv_row(
    card: &Card,
    runs: &[&SurfaceRun; 3],
    i: usize,
    outside: bool,
    flags: (usize, usize, usize),
) -> (usize, String) {
    let pick = |run: &SurfaceRun| -> (usize, f64) {
        if outside {
            (
                run.pool_would_be_ranks.get(i).copied().unwrap_or(0),
                run.pool_scores.get(i).copied().unwrap_or(f64::NAN),
            )
        } else {
            (
                run.candidate_ranks.get(i).copied().unwrap_or(0),
                run.candidate_scores.get(i).copied().unwrap_or(f64::NAN),
            )
        }
    };
    let (r1, s1) = pick(runs[0]);
    let (r2, s2) = pick(runs[1]);
    let (r3, s3) = pick(runs[2]);
    let line = format!(
        "{r2},{s2:.6},{r1},{s1:.6},{r3},{s3:.6},{},{},{},{},{},{},{},{}",
        csv_field(&card.id),
        csv_field(card.c_number.as_deref().unwrap_or("")),
        flags.0,
        flags.1,
        flags.2,
        csv_field(&card.document),
        csv_field(card.pinpoint.as_deref().unwrap_or("")),
        csv_field(&card.title)
    );
    (r2, line)
}
