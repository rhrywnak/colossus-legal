//! Disk/disk invariants over the extraction profiles, templates and schemas
//! (Standing Rule 21: when an invariant must hold across many files, assert it by
//! scanning the files).
//!
//! ## The defect this exists to catch
//!
//! Measured 2026-08-15, while building task P3. The v5.3 batch added one shared
//! date paragraph to every pass-1 template:
//!
//! > **Date format — `event_date` and `statement_date` MUST be ISO-8601.**
//!
//! It was pasted identically into all eight. But `statement_date` is declared in
//! exactly ONE schema — `affidavit_schema_v5_1.yaml`. The complaint, court-ruling
//! and discovery-response templates were all instructing the model to emit a
//! property their own schemas do not have.
//!
//! Nothing catches that. The template is prose; the schema is YAML; the pipeline
//! reads both and never compares them. The model either invents the property (and
//! it is dropped downstream) or ignores the instruction — and both outcomes are
//! silent. It survived from 2026-07-31 to 2026-08-15 in three templates at once,
//! which is exactly the profile of a defect that needs a scan rather than a
//! reviewer.
//!
//! ## What it checks
//!
//! 1. Every `template_file` and `pass2_template_file` a profile names EXISTS.
//! 2. Every `schema_file` a profile names EXISTS.
//! 3. Every pass-1 template carries the ISO-8601 date paragraph (P3's
//!    requirement, and the reason a future template cannot quietly omit it).
//! 4. No pass-1 template names a DATE property in its prose that its own schema
//!    does not declare.
//!
//! ## What it deliberately does not check
//!
//! Whether the prose is any good, and whether a property the schema declares is
//! mentioned in the prose. A schema may legitimately carry a property the
//! template never discusses — the schema JSON is substituted into the prompt in
//! full, so the model sees it either way. The failure runs one direction only:
//! prose promising something the schema has not declared.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

/// Date properties this scan knows about.
///
/// Deliberately a short explicit list rather than "any word ending in `_date`":
/// the prose is full of phrases like "the response date" and "the document's own
/// date", and a pattern loose enough to catch a property name would catch those
/// too. These are the two the schemas actually declare.
const DATE_PROPERTIES: &[&str] = &["event_date", "statement_date", "sent_date"];

/// The heading every pass-1 template's date rule sits under, and the standard it
/// must name (task P3, B2 §2).
///
/// Two fragments rather than one sentence, because the templates word the rule
/// differently — `discovery_response` says "`event_date` MUST be ISO-8601" and
/// `correspondence` says "Date format — ISO-8601." Both are the rule. Matching
/// the heading plus the standard lets an author reword the examples without
/// failing the build, while removing the rule still does.
const DATE_RULE_HEADING: &str = "Date format";
const DATE_RULE_STANDARD: &str = "ISO-8601";

/// Live profiles whose pass-1 template predates the ISO date rule.
///
/// Recorded rather than silently tolerated, the same way `oneshot::refs`
/// records what the re-key does not cover. All three point at COMPLAINT
/// templates two and three generations stale:
///
/// - `complaint.yaml` (v4) and `complaint_v5.yaml` (v5.0) are superseded by
///   `complaint_v5_1.yaml`, which is the one the pipeline registry maps
///   `complaint` to. They sit on disk unreferenced by the registry.
/// - **`default.yaml` is NOT superseded** — the registry maps it to
///   "Other / Unknown", so it is the profile any unrecognised document type
///   lands on, and it runs `pass1_complaint_v4.md`. Found 2026-08-15 by this
///   scan; reported, not fixed, because repointing the fallback profile changes
///   what an unknown document extracts and that is a ruling, not a cleanup.
///
/// Shrinking this list is the fix. Growing it needs a reason written here.
const PROFILES_WITHOUT_DATE_RULE: &[&str] =
    &["complaint.yaml", "complaint_v5.yaml", "default.yaml"];

fn backend_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Read one YAML scalar (`key: value`) without pulling in a parser.
///
/// The profiles are flat at the top level for these keys, and a real parse would
/// need the whole profile struct — which lives in the pipeline config and would
/// couple this scan to it.
fn yaml_scalar(body: &str, key: &str) -> Option<String> {
    body.lines()
        .map(str::trim_end)
        .find_map(|line| line.strip_prefix(&format!("{key}: ")))
        .map(|v| v.trim().trim_matches('"').to_string())
}

/// Every profile YAML on disk, as `(file name, body)`.
fn profiles() -> Vec<(String, String)> {
    let dir = backend_dir().join("profiles");
    let mut out = Vec::new();
    let entries =
        std::fs::read_dir(&dir).unwrap_or_else(|e| panic!("cannot read {}: {e}", dir.display()));
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("yaml") {
            continue;
        }
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default()
            .to_string();
        let body = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
        out.push((name, body));
    }
    out.sort();
    out
}

fn template_path(file: &str) -> PathBuf {
    backend_dir().join("extraction_templates").join(file)
}

fn schema_path(file: &str) -> PathBuf {
    backend_dir().join("extraction_schemas").join(file)
}

fn exists(path: &Path) -> bool {
    path.is_file()
}

#[test]
fn every_profile_names_a_template_that_exists() {
    let mut missing = Vec::new();
    for (profile, body) in profiles() {
        for key in ["template_file", "pass2_template_file"] {
            let Some(file) = yaml_scalar(&body, key) else {
                continue;
            };
            if !exists(&template_path(&file)) {
                missing.push(format!("{profile} → {key}: {file}"));
            }
        }
    }
    assert!(
        missing.is_empty(),
        "profiles name template files that are not on disk — the pipeline would \
         fail at extraction time, on a live document:\n  {}",
        missing.join("\n  ")
    );
}

#[test]
fn every_profile_names_a_schema_that_exists() {
    let mut missing = Vec::new();
    for (profile, body) in profiles() {
        let Some(file) = yaml_scalar(&body, "schema_file") else {
            continue;
        };
        if !exists(&schema_path(&file)) {
            missing.push(format!("{profile} → {file}"));
        }
    }
    assert!(
        missing.is_empty(),
        "profiles name schema files that are not on disk:\n  {}",
        missing.join("\n  ")
    );
}

#[test]
fn every_live_pass_one_template_carries_the_iso_date_rule() {
    // P3's requirement, asserted rather than reviewed. "Live" means referenced by
    // a profile: superseded templates are kept on disk deliberately and are not
    // held to the current rules.
    let mut without = Vec::new();
    for (profile, body) in profiles() {
        let Some(file) = yaml_scalar(&body, "template_file") else {
            continue;
        };
        let path = template_path(&file);
        let Ok(text) = std::fs::read_to_string(&path) else {
            continue; // the existence test above owns this failure
        };
        let has_rule = text.contains(DATE_RULE_HEADING) && text.contains(DATE_RULE_STANDARD);
        if !has_rule && !PROFILES_WITHOUT_DATE_RULE.contains(&profile.as_str()) {
            without.push(format!("{profile} → {file}"));
        }
    }
    assert!(
        without.is_empty(),
        "pass-1 templates with no ISO-8601 date rule. Without it, dates come back \
         in whatever format the source used and no chronology can sort them:\n  {}",
        without.join("\n  ")
    );
}

#[test]
fn the_recorded_date_rule_exceptions_are_all_still_real() {
    // The other direction. If someone fixes `default.yaml` to point at a current
    // template, this fails and makes them shrink the exception list — so the list
    // cannot quietly outlive the problem it records.
    let mut fixed = Vec::new();
    for (profile, body) in profiles() {
        if !PROFILES_WITHOUT_DATE_RULE.contains(&profile.as_str()) {
            continue;
        }
        let Some(file) = yaml_scalar(&body, "template_file") else {
            continue;
        };
        let Ok(text) = std::fs::read_to_string(template_path(&file)) else {
            continue;
        };
        if text.contains(DATE_RULE_HEADING) && text.contains(DATE_RULE_STANDARD) {
            fixed.push(format!("{profile} → {file}"));
        }
    }
    assert!(
        fixed.is_empty(),
        "these profiles now DO carry the date rule — remove them from \
         PROFILES_WITHOUT_DATE_RULE:\n  {}",
        fixed.join("\n  ")
    );
}

#[test]
fn no_pass_one_template_promises_a_date_property_its_schema_lacks() {
    // The measured defect: three templates named `statement_date`, one schema
    // declared it.
    let mut violations = Vec::new();
    for (profile, body) in profiles() {
        let (Some(template_file), Some(schema_file)) = (
            yaml_scalar(&body, "template_file"),
            yaml_scalar(&body, "schema_file"),
        ) else {
            continue;
        };
        let (Ok(template), Ok(schema)) = (
            std::fs::read_to_string(template_path(&template_file)),
            std::fs::read_to_string(schema_path(&schema_file)),
        ) else {
            continue; // the existence tests above own these failures
        };

        for property in declared_and_promised(&strip_authoring_comments(&template), &schema) {
            violations.push(format!(
                "{profile}: {template_file} names `{property}` but {schema_file} \
                 does not declare it"
            ));
        }
    }
    assert!(
        violations.is_empty(),
        "a template instructs the model to emit a property its schema has not \
         declared. The model either invents it (and it is dropped downstream) or \
         ignores the instruction, and both are silent:\n  {}",
        violations.join("\n  ")
    );
}

/// Drop `<!-- ... -->` blocks, the way the extraction pipeline does before the
/// prompt reaches the model.
///
/// Load-bearing here: a change note explaining that `statement_date` was REMOVED
/// mentions `statement_date`, and scanning the raw file would read that as the
/// template still promising it. The authoring comments are for humans; the scan
/// must see what the model sees.
///
/// ## Rust Learning: `split` on a multi-character pattern
///
/// `str::split` takes any `Pattern`, and `&str` is one — so splitting on `"-->"`
/// works exactly like splitting on a char. Taking the part after each terminator
/// is enough because these comments never nest.
fn strip_authoring_comments(template: &str) -> String {
    let mut out = String::with_capacity(template.len());
    let mut rest = template;
    while let Some((before, after_open)) = rest.split_once("<!--") {
        out.push_str(before);
        match after_open.split_once("-->") {
            Some((_, after_close)) => rest = after_close,
            // An unterminated comment swallows the remainder, which is what the
            // pipeline's own stripper does too.
            None => return out,
        }
    }
    out.push_str(rest);
    out
}

/// Date properties the template mentions in backticks that the schema does not
/// declare.
///
/// Backticks matter: the templates discuss "the response date" and "the
/// document's own date" in plain prose all the time, and only a backticked
/// `event_date` is a promise about a property name.
fn declared_and_promised(template: &str, schema: &str) -> BTreeSet<String> {
    DATE_PROPERTIES
        .iter()
        .filter(|property| template.contains(&format!("`{property}`")))
        .filter(|property| !schema.contains(&format!("name: {property}")))
        .map(|property| (*property).to_string())
        .collect()
}

#[test]
fn the_scan_can_actually_see_the_files_it_claims_to_check() {
    // A scan that silently found nothing would pass every test above. This is the
    // canary: if the directory layout moves, this fails first and names the
    // reason, instead of four green tests that checked an empty list.
    let found = profiles();
    assert!(
        found.len() >= 5,
        "found only {} profile YAML(s) under {}/profiles — the layout moved and \
         the invariant scans above are checking nothing",
        found.len(),
        backend_dir().display()
    );
    assert!(
        found
            .iter()
            .any(|(_, body)| yaml_scalar(body, "template_file").is_some()),
        "no profile parsed a template_file — the scalar reader stopped working"
    );
}
