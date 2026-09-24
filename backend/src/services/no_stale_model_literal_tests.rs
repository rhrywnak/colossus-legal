// =============================================================================
// ⚑ NO MODEL NAME IS COMPILED INTO A SURFACE THAT SERVES ONE
// =============================================================================
//
// CC_TASK_CHAT_DEFAULT_MODEL_v1. Roman's ruling of 2026-09-19: the amended grep
// becomes a disk test, because a verification that lives only in a report is a
// verification nobody re-runs.
//
// ## What went wrong, and why nothing caught it
//
// `main.rs` carried `const DEFAULT_CHAT_MODEL = "claude-sonnet-4-6"` and
// `frontend/src/services/ask.ts` carried the same id twice more, in a `catch`
// that fabricated a one-model catalogue. On 2026-09-19 that model was
// deactivated in the Admin list. The backend booted cleanly, the frontend
// showed a picker with one entry, and every `/ask` that did not name a model
// answered 400 — because the chat provider map is built from the ACTIVE rows
// and the compiled-in name was no longer among them.
//
// Nothing in the build could notice. A string literal is well-typed; a model id
// that no longer exists is well-typed; and the only place the two vocabularies
// meet is the database at runtime.
//
// ## The rule this test enforces
//
// No Anthropic model id may appear in `backend/src` or `frontend/src` OUTSIDE
// the exemptions below. Model ids are DATA — rows in `llm_models`, or values in
// the settings store — and a surface that names one has taken a decision that
// belongs to Roman's Admin list.
//
// ## The exemptions, and why each one is a decision and not a hole
//
// * `backend/src/pipeline/` — ruled OUT OF SCOPE on 2026-09-19.
//   `pipeline::config::SystemDefaults::model()` is the EXTRACTION default and
//   the same class of defect; it is on the owed list and is the architect's to
//   schedule. Narrowed to that directory rather than waived, so the day it is
//   fixed this test tightens by deleting one line.
// * TESTS AND FIXTURES — a test asserting behaviour for a named model is the
//   test doing its job. Recognised by `#[cfg(test)]`, `_tests.rs`, `/tests/`,
//   `__tests__`, and `.test.`.
// * COMMENTS — this repo documents its rules beside its rules, and half the
//   occurrences are prose explaining exactly this defect. Both comment styles
//   are stripped: `//` for Rust and TypeScript, and `/* … */`, without which
//   the doc comment on `fetchChatModels` reports itself.

use std::path::{Path, PathBuf};

/// Directories whose model literals are ruled out of scope.
// STRUCTURAL: a repo-internal path, and the boundary of a ruling — not a
// deployment value. A moved directory fails this test rather than silently
// widening the exemption.
const EXEMPT_DIRS: &[&str] = &["pipeline"];

/// Single files whose model literal is ruled, each with its ruling.
///
/// * `services/chat_keepwarm_rates.rs` — CC_TASK_KEEPWARM_BUTTON_v1, Law 10,
///   ruled 2026-09-24: the published Opus 5.5 prices sit in named constants
///   until the registry carries cache prices, and "any other model: the cost is
///   not known". Naming the one priced model is what makes every OTHER model
///   answer "not known" instead of borrowing its prices. It chooses no model and
///   serves none. DEBT(CC_TASK_COST_PAGE_v1): delete this line when the prices
///   move into `llm_models`.
// STRUCTURAL: repo-internal paths and the boundary of a ruling.
const EXEMPT_FILES: &[&str] = &["services/chat_keepwarm_rates.rs"];

/// The roots this rule covers.
// STRUCTURAL: repo-internal source paths, exactly as `SURFACE_DIRS` in
// `dto::practice_wording_reach_tests`.
const ROOTS: &[&str] = &["src", "../frontend/src"];

/// Source with both comment styles removed.
///
/// ⚑ Required before any scan of this repository. The rule, and the five
/// instances that produced it, are stated once in `domain::wording_tests` above
/// `seeded_value_in` — with one addition here: TypeScript `/** … */` doc
/// comments carry the history of this very defect, so a scanner stripping only
/// `//` reports the explanation as the offence.
fn without_comments(source: &str) -> String {
    let mut out = String::with_capacity(source.len());
    let bytes: Vec<char> = source.chars().collect();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == '/' && i + 1 < bytes.len() && bytes[i + 1] == '*' {
            // Skip to the closing `*/`, or to the end if it never closes.
            i += 2;
            while i + 1 < bytes.len() && !(bytes[i] == '*' && bytes[i + 1] == '/') {
                i += 1;
            }
            i = (i + 2).min(bytes.len());
            continue;
        }
        if bytes[i] == '/' && i + 1 < bytes.len() && bytes[i + 1] == '/' {
            while i < bytes.len() && bytes[i] != '\n' {
                i += 1;
            }
            continue;
        }
        out.push(bytes[i]);
        i += 1;
    }
    out
}

/// Every `.rs` / `.ts` / `.tsx` file under the roots, exemptions removed.
fn scanned_files() -> Vec<PathBuf> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut out = Vec::new();
    for dir in ROOTS {
        walk(&root.join(dir), &mut out);
    }
    out.sort();
    out
}

fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if path.is_dir() {
            if EXEMPT_DIRS.contains(&name) || name == "__tests__" || name == "tests" {
                continue;
            }
            walk(&path, out);
            continue;
        }
        let is_source = name.ends_with(".rs") || name.ends_with(".ts") || name.ends_with(".tsx");
        let is_test = name.ends_with("_tests.rs") || name.contains(".test.");
        let unix = path.to_string_lossy().replace('\\', "/");
        let is_exempt = EXEMPT_FILES.iter().any(|f| unix.ends_with(f));
        if is_source && !is_test && !is_exempt {
            out.push(path);
        }
    }
}

/// Everything from the first `#[cfg(test)]` onward — a Rust file's test half.
fn without_test_module(source: &str) -> &str {
    match source.find("#[cfg(test)]") {
        Some(at) => &source[..at],
        None => source,
    }
}

#[test]
fn no_anthropic_model_id_is_compiled_into_a_surface() {
    let files = scanned_files();

    // ⚑ ANTI-VACUITY BY SENTINEL, NOT BY COUNT.
    //
    // A scan that read nothing would report nothing forever, and a threshold
    // ("more than 50 files") cannot tell that from a scan that lost one root.
    // One named file per root, each of which exists only in that root.
    for (needle, root) in [
        ("src/services/chat_default.rs", "backend/src"),
        ("frontend/src/services/ask.ts", "frontend/src"),
    ] {
        assert!(
            files
                .iter()
                .any(|f| f.to_string_lossy().replace('\\', "/").contains(needle)),
            "the scan has stopped reading {root} — every literal in it is now escaping"
        );
    }
    // And the exemption is real: the scan must NOT be reading it.
    assert!(
        !files.iter().any(|f| f
            .to_string_lossy()
            .replace('\\', "/")
            .contains("src/pipeline/config.rs")),
        "backend/src/pipeline is exempt by ruling; the walk is no longer skipping it"
    );

    let mut found: Vec<String> = Vec::new();
    for path in &files {
        let Ok(raw) = std::fs::read_to_string(path) else {
            continue;
        };
        let source = without_comments(without_test_module(&raw));
        for line in source.lines() {
            // `claude-<something>` inside a string literal is a model id. The
            // hyphen is what distinguishes it from the word in prose, which the
            // comment strip has already removed anyway.
            if let Some(at) = line.find("\"claude-") {
                let rest = &line[at + 1..];
                let id: String = rest.chars().take_while(|c| *c != '"').collect();
                found.push(format!("{}  ← {}", id, path.display()));
            }
        }
    }
    found.sort();
    found.dedup();

    assert!(
        found.is_empty(),
        "a model id is compiled into a surface that serves one. Model ids are \
         DATA — an llm_models row, or a settings row — and a literal here is a \
         decision taken away from the Admin list. This is what shipped the \
         2026-09-19 PROD defect:\n  {}",
        found.join("\n  ")
    );
}
