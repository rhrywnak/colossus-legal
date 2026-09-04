//! What the run leaves behind: two JSON files, a README, and the printed proof.
//!
//! ## Why the README is written by the tool and not by hand
//!
//! A hand-written README records what somebody MEANT to run. This one records
//! the arguments this run actually parsed, so the command it prints is the
//! command that produced the files beside it. The one thing it deliberately does
//! NOT echo is `--database-url`: reconstructing the invocation from the parsed
//! `Args` rather than from `std::env::args()` is what keeps a password out of a
//! file in the documents folder.

use std::path::Path;
use std::process::ExitCode;

use colossus_legal_backend::oneshot::exit::EXIT_BAD_INPUT;
use colossus_legal_backend::services::gate_fixture::{FixtureAudit, GateFixture};
use tracing::{error, info};

use crate::Args;

/// Write one fixture, pretty-printed so a human can read it.
pub(crate) fn write_json(dir: &Path, name: &str, fixture: &GateFixture) -> Result<(), ExitCode> {
    std::fs::create_dir_all(dir).map_err(|e| {
        error!(error = %e, dir = %dir.display(), "could not create the output directory");
        ExitCode::from(EXIT_BAD_INPUT)
    })?;
    let json = serde_json::to_string_pretty(fixture).map_err(|e| {
        error!(error = %e, "could not serialize the fixture");
        ExitCode::from(EXIT_BAD_INPUT)
    })?;
    let path = dir.join(name);
    std::fs::write(&path, json).map_err(|e| {
        error!(error = %e, path = %path.display(), "could not write the fixture");
        ExitCode::from(EXIT_BAD_INPUT)
    })?;
    info!(path = %path.display(), "fixture written");
    Ok(())
}

/// One scenario's detail, printed as it is built.
pub(crate) fn report(fixture: &GateFixture, audit: &FixtureAudit) {
    println!("\n--- {} ---", fixture.scenario);
    println!("run {} started {}", fixture.run_id, fixture.run_started_at);
    println!(
        "query: subject {} · theme {} · {} allegations · {} talking points",
        fixture.query.subject,
        if fixture.query.theme.is_some() {
            "present"
        } else {
            "ABSENT"
        },
        fixture.query.allegations.len(),
        fixture.query.talking_points.len()
    );
    for a in &fixture.query.allegations {
        println!("  {} {}", a.id, first_clause(&a.text));
    }
    println!("outside_pool ({}):", fixture.outside_pool.len());
    for c in &fixture.outside_pool {
        println!("  {} · {}", c.id, c.title);
    }
    for check in &audit.checks {
        println!(
            "  [{}] {} — {}",
            mark(check.passed),
            check.name,
            check.detail
        );
    }
    for id in &audit.included_not_relevant {
        println!("  FINDING: Included but not called relevant: {id}");
    }
}

fn mark(passed: bool) -> &'static str {
    if passed {
        "ok"
    } else {
        "FAIL"
    }
}

/// The first ~80 characters, for a one-line echo of a long allegation.
fn first_clause(text: &str) -> String {
    match text.char_indices().nth(80) {
        Some((cut, _)) => format!("{}…", &text[..cut]),
        None => text.to_string(),
    }
}

/// The block the report pastes verbatim.
pub(crate) fn print_assertion_block(audits: &[(GateFixture, FixtureAudit)]) {
    println!("\n=== ASSERTIONS ===");
    for (_, audit) in audits {
        println!("{}", audit.count_line);
    }
    for (fixture, audit) in audits {
        if !audit.counts_match {
            println!(
                "{}: a count differs from the expectation. NOT adjusted — see the line above.",
                fixture.scenario
            );
        }
    }
}

/// The README that makes the fixtures re-derivable by anyone.
pub(crate) fn write_readme(
    args: &Args,
    audits: &[(GateFixture, FixtureAudit)],
) -> Result<(), ExitCode> {
    let mut body = String::from("# GATE — frozen fixtures for the reranker gate\n\n");
    body.push_str("Written by `backend/src/bin/gate_fixture/` (`cargo run --bin gate_fixture`). READ-ONLY, zero cost: no paid API call, no embedding, and every Postgres session it opens is `SET SESSION CHARACTERISTICS AS TRANSACTION READ ONLY`.\n\n");
    for (fixture, audit) in audits {
        body.push_str(&format!(
            "## {}\n\n- run id: `{}`\n- run started: {}\n- extracted: {}\n- counts: `{}`\n\n",
            fixture.scenario,
            fixture.run_id,
            fixture.run_started_at,
            fixture.extracted_at,
            audit.count_line
        ));
    }
    body.push_str(&format!(
        "## Regenerating both files\n\nNeeds `GATE_FIXTURE_DATABASE_URL` (the pipeline \
         database, `{}`) and `NEO4J_URI` / `NEO4J_USER` / `NEO4J_PASSWORD`. This tool does \
         NOT read `.env`.\n\n```bash\n{}\n```\n",
        args.expect_database,
        regeneration_command(args)
    ));
    write_text(&args.out_dir, "README.md", &body)
}

/// Rebuild the invocation from the parsed arguments — never from `env::args()`,
/// which would echo a `--database-url` carrying a password into a file.
fn regeneration_command(args: &Args) -> String {
    let mut cmd = format!(
        "cargo run --bin gate_fixture -- \\\n  --expect-database {} \\\n  --case-slug {} \\\n  \
         --out-dir {} \\\n  --pinpoint-template '{}'",
        args.expect_database,
        args.case_slug,
        args.out_dir.display(),
        args.pinpoint_template
    );
    for (flag, values) in [
        ("--scenario", &args.scenarios),
        ("--run", &args.runs),
        ("--expect", &args.expects),
        ("--file", &args.files),
        ("--outside", &args.outside),
    ] {
        for value in values {
            cmd.push_str(&format!(" \\\n  {flag} '{value}'"));
        }
    }
    cmd
}

fn write_text(dir: &Path, name: &str, body: &str) -> Result<(), ExitCode> {
    let path = dir.join(name);
    std::fs::write(&path, body).map_err(|e| {
        error!(error = %e, path = %path.display(), "could not write the README");
        ExitCode::from(EXIT_BAD_INPUT)
    })?;
    info!(path = %path.display(), "README written");
    Ok(())
}
