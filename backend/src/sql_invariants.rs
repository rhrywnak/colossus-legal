//! Disk/code invariant over EVERY parameterised sqlx statement in `src/`
//! (Standing Rule 21: when an invariant must hold across many files, assert
//! it by scanning the files).
//!
//! ## The defect this exists to catch
//!
//! On 2026-07-19, retiring the Theme Scan `dry_run` flag replaced two
//! adjacent `.bind()` lines with one:
//!
//! ```text
//! -    .bind(&start.resolved_params)
//! -    .bind(start.dry_run)
//! +    .bind(false)
//! ```
//!
//! The `resolved_params` bind was collateral. The INSERT still declared `$1`
//! through `$8`, but only seven values were supplied, so every bind after
//! the third shifted one place left: the literal `false` landed in `$4`, the
//! `resolved_params` **jsonb** column. Postgres rejected it —
//! *"column resolved_params is of type jsonb but expression is of type
//! boolean"* — and every Theme Scan failed at its start-record for eleven
//! days, because nothing else exercised that INSERT.
//!
//! ## Why a scan and not a unit test
//!
//! Nothing about the broken code is detectable by the compiler: `.bind()` is
//! variadic-by-chaining, so seven binds against eight placeholders is
//! well-typed Rust. It is only wrong against the SQL string beside it, and
//! only Postgres can say so — at runtime, on a live connection. The defect
//! class is therefore invisible to `cargo test --lib` unless something reads
//! the SQL and the chain together, which is what this does.
//!
//! ## What it checks, and the one thing it cannot
//!
//! It compares the highest `$N` placeholder in each statement against the
//! number of `.bind()` calls chained onto it. That is exactly the mechanical
//! signature of the 2026-07-19 defect, and it generalises: any dropped,
//! duplicated or mis-numbered bind trips it.
//!
//! It does NOT check that each bound value's Rust type matches its column's
//! SQL type — that needs a live connection (`sqlx::Executor::describe`) or
//! the compile-time `query!` macro, neither available in this target. A
//! same-arity type swap between two adjacent columns would still pass here.
//! `scan_runs_tests` narrows that gap for the statement that regressed by
//! checking its column list against the migration DDL on disk.
//!
//! ## Why the terminator is `.await`, not `;`
//!
//! A first version of this scan keyed on the terminating semicolon and
//! produced nine false positives, because an sqlx chain is very often a
//! function's tail expression with no `;` at all — the walk ran into the
//! following function and counted its binds. Chains always end in `.await`;
//! that is the reliable boundary.

use std::fs;
use std::path::{Path, PathBuf};

/// One parameterised statement found in the source tree.
struct Statement {
    file: String,
    line: usize,
    highest_placeholder: usize,
    bind_count: usize,
}

/// Every `.rs` file under `src/`, recursively.
fn rust_sources(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).expect("src/ is readable") {
        let path = entry.expect("dir entry readable").path();
        if path.is_dir() {
            rust_sources(&path, out);
        } else if path.extension().and_then(|e| e.to_str()) == Some("rs") {
            out.push(path);
        }
    }
}

/// Byte offset just past the `(` opening an `sqlx::query…(` call, scanning
/// from `from`. Returns `None` when no further call exists.
fn next_call(text: &str, from: usize) -> Option<usize> {
    let rest = &text[from..];
    let at = rest.find("sqlx::query")?;
    let abs = from + at;
    let open = text[abs..].find('(')? + abs;
    Some(open)
}

/// The call chain's text, from its opening paren to the `.await` (or `;`)
/// that ends it. String literals are skipped so a paren or semicolon inside
/// SQL cannot terminate the walk early.
fn chain_text(text: &str, open_paren: usize) -> &str {
    let bytes: Vec<char> = text[open_paren..].chars().collect();
    let mut depth: i32 = 1;
    let mut i = 1usize;
    // None = code, Some(true) = inside r#"…"#, Some(false) = inside "…"
    let mut in_string: Option<bool> = None;
    let mut byte_len = text[open_paren..].len();
    let mut consumed = open_paren + 1;

    while i < bytes.len() {
        let c = bytes[i];
        match in_string {
            Some(true) => {
                if c == '"' && bytes.get(i + 1) == Some(&'#') {
                    in_string = None;
                    i += 1;
                }
            }
            Some(false) => {
                if c == '\\' {
                    i += 1;
                } else if c == '"' {
                    in_string = None;
                }
            }
            None => {
                if c == 'r' && bytes.get(i + 1) == Some(&'#') && bytes.get(i + 2) == Some(&'"') {
                    in_string = Some(true);
                    i += 2;
                } else if c == '"' {
                    in_string = Some(false);
                } else if c == '(' {
                    depth += 1;
                } else if c == ')' {
                    depth -= 1;
                } else if depth == 0
                    && (bytes[i..].starts_with(&['.', 'a', 'w', 'a', 'i', 't']) || c == ';')
                {
                    byte_len = bytes[..i].iter().map(|c| c.len_utf8()).sum();
                    consumed = open_paren + byte_len;
                    return &text[open_paren..consumed];
                }
            }
        }
        i += 1;
    }
    let _ = consumed;
    &text[open_paren..open_paren + byte_len]
}

/// Highest `$N` in `sql`, or `None` when the statement takes no parameters.
fn highest_placeholder(chain: &str) -> Option<usize> {
    let mut highest = None;
    let bytes: Vec<char> = chain.chars().collect();
    for (i, c) in bytes.iter().enumerate() {
        if *c != '$' {
            continue;
        }
        let digits: String = bytes[i + 1..]
            .iter()
            .take_while(|c| c.is_ascii_digit())
            .collect();
        if let Ok(n) = digits.parse::<usize>() {
            highest = Some(highest.map_or(n, |h: usize| h.max(n)));
        }
    }
    highest
}

fn collect_statements() -> Vec<Statement> {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut files = Vec::new();
    rust_sources(&src, &mut files);
    files.sort();

    let mut statements = Vec::new();
    for path in files {
        let text = fs::read_to_string(&path).expect("source file is UTF-8");
        let rel = path
            .strip_prefix(env!("CARGO_MANIFEST_DIR"))
            .unwrap_or(&path)
            .to_string_lossy()
            .to_string();
        let mut cursor = 0usize;
        while let Some(open) = next_call(&text, cursor) {
            let chain = chain_text(&text, open);
            cursor = open + 1;
            let Some(highest) = highest_placeholder(chain) else {
                continue;
            };
            statements.push(Statement {
                file: rel.clone(),
                line: text[..open].matches('\n').count() + 1,
                highest_placeholder: highest,
                bind_count: chain.matches(".bind(").count(),
            });
        }
    }
    statements
}

/// Every parameterised sqlx statement supplies exactly as many `.bind()`
/// values as its SQL declares placeholders.
#[test]
fn every_sqlx_statement_binds_one_value_per_placeholder() {
    let mismatches: Vec<String> = collect_statements()
        .iter()
        .filter(|s| s.highest_placeholder != s.bind_count)
        .map(|s| {
            format!(
                "{}:{} — SQL declares ${} but {} .bind() call(s) are chained",
                s.file, s.line, s.highest_placeholder, s.bind_count
            )
        })
        .collect();

    assert!(
        mismatches.is_empty(),
        "placeholder/bind arity mismatch — the binding order shifts and a \
         value lands in the wrong column, which Postgres reports as a type \
         error at RUNTIME only:\n  {}",
        mismatches.join("\n  ")
    );
}

/// The scan must actually find the statements it claims to police. Without
/// this, a regression in the parser (an early terminator, a changed call
/// spelling) would empty the corpus and the assertion above would pass
/// vacuously — a green test guarding nothing, which is worse than no test.
#[test]
fn the_scan_finds_a_representative_corpus() {
    let statements = collect_statements();
    assert!(
        statements.len() > 80,
        "expected the pipeline repository's parameterised statements; found {} \
         — the scanner is probably no longer matching call sites",
        statements.len()
    );
    assert!(
        statements
            .iter()
            .any(|s| s.file.ends_with("scan_runs.rs") && s.highest_placeholder >= 8),
        "the scan_runs start INSERT (the statement that regressed on \
         2026-07-19) must be in the scanned corpus"
    );
}
