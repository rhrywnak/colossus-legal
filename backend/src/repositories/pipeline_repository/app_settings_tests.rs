//! SQL-shape tests for [`super`] — the configuration store's statements.
//!
//! The writes need a live Postgres and are DEV-verified, the house convention.
//! What is pinned here is the SHAPE of the production statements, read from the
//! `const`s themselves rather than from copies — the lesson of task 1.5's
//! self-asserting delete test.

use super::*;

/// A value edit may not widen its own bounds.
///
/// `value_kind`, `min_value`, `max_value`, `default_value` and `meaning` are
/// declarations about what a parameter IS. If the update touched them, a human
/// raising the talking-points cap could also — deliberately or by a bug in the
/// handler — move the ceiling that was supposed to constrain them, and the
/// bounds check would become theatre.
#[test]
fn the_update_changes_the_value_and_nothing_that_constrains_it() {
    for column in [
        "value_kind",
        "min_value",
        "max_value",
        "default_value",
        "meaning",
        "consumed_by",
    ] {
        assert!(
            !UPDATE_SETTING_SQL.contains(column),
            "the settings update touches {column}; an edit must not be able to \
             change a parameter's own declaration: {UPDATE_SETTING_SQL}"
        );
    }
    assert!(
        UPDATE_SETTING_SQL.contains("value = $2"),
        "{UPDATE_SETTING_SQL}"
    );
}

/// The update is fenced to one key.
#[test]
fn the_update_names_its_key() {
    assert!(
        UPDATE_SETTING_SQL.contains("WHERE key = $1"),
        "an unfenced UPDATE would set every parameter to one value: \
         {UPDATE_SETTING_SQL}"
    );
}

/// Every edit records who and when, on the row as well as in the ledger.
#[test]
fn the_update_stamps_the_actor_and_the_time() {
    for column in ["updated_at", "updated_by"] {
        assert!(
            UPDATE_SETTING_SQL.contains(column),
            "the settings update must write {column} so the page can show who \
             last changed a parameter without a join"
        );
    }
}

/// The ledger APPENDS. An upsert would make it a column pair with extra steps.
#[test]
fn the_change_ledger_is_append_only() {
    assert!(
        !INSERT_CHANGE_SQL.contains("ON CONFLICT"),
        "every configuration change is its own row — the history is the point: \
         {INSERT_CHANGE_SQL}"
    );
    assert!(!INSERT_CHANGE_SQL.contains("UPDATE"));
    assert!(!INSERT_CHANGE_SQL.contains("DELETE"));
}

/// A ledger row answers who, what, and both sides of the change.
#[test]
fn the_ledger_records_both_sides_and_the_actor() {
    for column in ["key", "old_value", "new_value", "actor", "at"] {
        assert!(
            INSERT_CHANGE_SQL.contains(column),
            "a change record without {column} cannot answer what changed or who \
             changed it"
        );
    }
    assert!(INSERT_CHANGE_SQL.contains("$5"));
    assert!(!INSERT_CHANGE_SQL.contains("$6"));
}

/// The projection covers exactly the fields `AppSettingRecord` decodes.
///
/// A column added to the struct without being added here fails at runtime as a
/// decode error on a live query — which this target cannot run. Comparing the two
/// lists catches it at `cargo test` instead.
#[test]
fn the_projection_matches_the_record_the_page_needs() {
    for column in [
        "key",
        "value",
        "value_kind",
        "default_value",
        "min_value",
        "max_value",
        "meaning",
        "consumed_by",
        "updated_at",
        "updated_by",
    ] {
        assert!(
            SETTING_COLUMNS.contains(column),
            "{column} is part of AppSettingRecord and must be selected"
        );
    }
}

// ── The migration↔struct type seam ───────────────────────────────────────────
//
// THE DEFECT THIS SECTION EXISTS FOR. v2.0.0-beta.364 applied every migration,
// seeded the store, and then refused to boot on the first live read:
//
//   error occurred while decoding column "min_value": mismatched types; Rust
//   type `core::option::Option<f64>` (as SQL type `FLOAT8`) is not compatible
//   with SQL type `NUMERIC`
//
// The migration was green. The struct was green. 1240 tests were green. Nothing
// compared the two, because the seam between them is the one place this codebase
// had no way to look: `query_as` (not the `query_as!` macro) does its type check
// at RUNTIME against a live server, so a disagreement is invisible until a real
// row is decoded — and every test builds `AppSettingRecord` from a fixture.
//
// These tests read the declared SQL type out of the migration on disk and
// compare it to the Rust type that decodes it. No database, so they run in CI,
// where they would have failed red on the 1.6 commit.

/// Every column this crate decodes, and the SQL types sqlx can decode it FROM.
///
/// ## Why the accepted-type lists are what they are
///
/// sqlx implements `Type<Postgres>` for `f64` against `FLOAT8` alone. `NUMERIC`
/// decodes only into `Decimal` / `BigDecimal`, and neither `rust_decimal` nor
/// `bigdecimal` is in this workspace's dependency tree — so a bare `NUMERIC`
/// column here has no decodable Rust type AT ALL. That is why `NUMERIC` is
/// absent from every list below rather than merely discouraged for `f64`.
///
/// `REAL` (`FLOAT4`) is absent for the same reason in miniature: it decodes into
/// `f32`, and `Option<f64>` would fail on it exactly as it failed on `NUMERIC`.
/// Accepting a type because it is "close enough" numerically is the mistake this
/// whole file exists to prevent — sqlx checks the wire type, not the range.
///
/// `key` appears once though both tables declare it; both are `TEXT`, so one
/// entry covers both. A future divergence would be caught by whichever table's
/// declaration stopped matching.
///
/// The tuple is (column, the Rust type that decodes it, accepted SQL types).
/// The Rust type is carried as a string only so a failure message can name it —
/// Rust has no reflection, so this table is hand-written. It is chained to
/// reality at both ends: `the_projection_matches_the_record_the_page_needs`
/// pins the projection to the struct, and
/// `the_type_table_covers_every_column_the_projection_selects` (below) pins this
/// table to the projection. Same three-link discipline as the seed fixtures in
/// `settings_store_tests`.
const DECODED_COLUMNS: &[(&str, &str, &[&str])] = &[
    ("key", "String", &["TEXT"]),
    ("value", "String", &["TEXT"]),
    ("value_kind", "String", &["TEXT"]),
    ("default_value", "String", &["TEXT"]),
    ("min_value", "Option<f64>", &["DOUBLE PRECISION", "FLOAT8"]),
    ("max_value", "Option<f64>", &["DOUBLE PRECISION", "FLOAT8"]),
    ("meaning", "String", &["TEXT"]),
    ("consumed_by", "Option<String>", &["TEXT"]),
    ("updated_at", "DateTime<Utc>", &["TIMESTAMPTZ"]),
    ("updated_by", "String", &["TEXT"]),
    // `app_setting_changes`. `id` is deliberately absent: it is never selected
    // and never bound (the DEFAULT supplies it), so it has no decode seam.
    ("old_value", "String", &["TEXT"]),
    ("new_value", "String", &["TEXT"]),
    ("actor", "String", &["TEXT"]),
    ("at", "DateTime<Utc>", &["TIMESTAMPTZ"]),
];

/// Read a migration file from the crate root.
fn migration_text(relative_path: &str) -> String {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(relative_path);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("{} must be readable: {e}", path.display()))
}

/// The SQL type each column ENDS UP with, across every migration that declares it.
///
/// Starts from the `CREATE TABLE` declarations and then applies any later
/// `ALTER COLUMN … TYPE …`, in filename order. Reading only the CREATE would
/// have this test asserting the schema as it was on day one — which, after the
/// hotfix that renamed these very types, would be exactly wrong.
///
/// ## Rust Learning: `split_whitespace` over a regex
///
/// No regex crate is needed for house-authored DDL: the declarations have a
/// fixed shape (`name TYPE [constraints],`), so the column is the first token
/// and the type is what follows until a keyword. A parser that cannot handle
/// arbitrary SQL is honest about it — `effective_types` simply will not find a
/// column it cannot read, and the coverage test below fails loudly when that
/// happens rather than passing on an empty map.
fn effective_types() -> std::collections::HashMap<String, String> {
    let mut types = std::collections::HashMap::new();

    // The migration that creates both tables, named at its one use site the way
    // `the_listing_puts_live_parameters_before_dormant_ones` names the file it
    // reads: a path to a specific file in this repository is a pointer, not a
    // value that could vary by deployment.
    let create = "pipeline_migrations/20260801225147_create_app_settings_store.sql";
    for line in migration_text(create).lines() {
        if let Some((column, sql_type)) = created_column(line) {
            types.insert(column, sql_type);
        }
    }

    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("pipeline_migrations");
    let mut files: Vec<_> = std::fs::read_dir(&dir)
        .expect("pipeline_migrations is readable")
        .filter_map(|entry| entry.ok().map(|e| e.path()))
        // Only `.sql`: sqlx applies nothing else, so nothing else may influence
        // what this test believes the schema is.
        .filter(|path| path.extension().is_some_and(|ext| ext == "sql"))
        .collect();
    files.sort();

    for file in files {
        let sql = std::fs::read_to_string(&file).expect("migration is readable");
        if !sql.contains("ALTER TABLE app_settings") {
            continue;
        }
        for line in sql.lines() {
            if let Some((column, sql_type)) = altered_column(line) {
                types.insert(column, sql_type);
            }
        }
    }
    types
}

/// `    min_value     NUMERIC,` → `("min_value", "NUMERIC")`. `None` for
/// comments, blank lines, constraints and anything that is not a declaration.
fn created_column(line: &str) -> Option<(String, String)> {
    let trimmed = line.trim();
    if trimmed.is_empty() || trimmed.starts_with("--") || trimmed.starts_with("CONSTRAINT") {
        return None;
    }
    let mut tokens = trimmed.split_whitespace();
    let name = tokens.next()?;
    // A column name is a bare identifier. Anything with punctuation is SQL
    // prose (`CREATE TABLE app_settings (`, `);`, a COMMENT body).
    if !name.chars().all(|c| c.is_ascii_lowercase() || c == '_') {
        return None;
    }
    Some((name.to_string(), leading_type(tokens)?))
}

/// `    ALTER COLUMN min_value TYPE DOUBLE PRECISION USING …` →
/// `("min_value", "DOUBLE PRECISION")`.
fn altered_column(line: &str) -> Option<(String, String)> {
    let after = line.trim().strip_prefix("ALTER COLUMN ")?;
    let (name, rest) = after.split_once(" TYPE ")?;
    Some((
        name.trim().to_string(),
        leading_type(rest.split_whitespace())?,
    ))
}

/// The type tokens at the head of a declaration, stopping at the first thing
/// that is not part of a type name.
///
/// Handles the two-word `DOUBLE PRECISION` as well as the single-word types,
/// and stops at `NOT` / `PRIMARY` / `DEFAULT` / `USING` or a trailing comma.
fn leading_type<'a>(tokens: impl Iterator<Item = &'a str>) -> Option<String> {
    const STOP: &[&str] = &[
        "NOT",
        "NULL",
        "PRIMARY",
        "DEFAULT",
        "UNIQUE",
        "USING",
        "REFERENCES",
    ];
    let mut parts: Vec<String> = Vec::new();
    for token in tokens {
        let cleaned = token.trim_end_matches([',', ';']);
        if cleaned.is_empty() || STOP.contains(&cleaned) {
            break;
        }
        parts.push(cleaned.to_ascii_uppercase());
        // A type is at most two words here; a third token is a constraint.
        if parts.len() == 2 {
            break;
        }
    }
    match parts.len() {
        0 => None,
        1 => Some(parts.remove(0)),
        // Only DOUBLE PRECISION is genuinely two words; anything else that
        // reached two tokens is a single-word type followed by a constraint.
        _ if parts[0] == "DOUBLE" => Some(format!("{} {}", parts[0], parts[1])),
        _ => Some(parts.remove(0)),
    }
}

/// Every column this build decodes is declared as a type sqlx can decode.
///
/// THE beta.364 TEST. With `min_value NUMERIC` and `Option<f64>`, this fails —
/// verified by running it against the pre-hotfix migration alone.
#[test]
fn the_migration_declares_only_types_this_build_can_decode() {
    let declared = effective_types();

    for (column, rust_type, accepted) in DECODED_COLUMNS {
        let sql_type = declared.get(*column).unwrap_or_else(|| {
            panic!("{column} is decoded by this crate but no migration declares it")
        });
        assert!(
            accepted.contains(&sql_type.as_str()),
            "{column} is declared {sql_type} but decoded as {rust_type}, which sqlx \
             reads from {accepted:?} only. A NUMERIC column has NO decodable Rust \
             type in this build (no rust_decimal / bigdecimal): change the column \
             type, or cast it in the projection the way llm_models does with \
             `cost_per_input_token::float8`. This is the seam v2.0.0-beta.364 \
             died on."
        );
    }
}

/// The type table above covers every column the projection selects.
///
/// Anti-vacuity, and the third link of the chain: without this, adding a column
/// to `AppSettingRecord` and `SETTING_COLUMNS` would leave the new column
/// untyped-checked, and the test above would pass while the store failed to
/// decode exactly as beta.364 did.
#[test]
fn the_type_table_covers_every_column_the_projection_selects() {
    for column in SETTING_COLUMNS.split(',').map(str::trim) {
        assert!(
            DECODED_COLUMNS.iter().any(|(name, _, _)| *name == column),
            "{column} is selected into AppSettingRecord but is missing from \
             DECODED_COLUMNS, so nothing checks that its declared SQL type can \
             be decoded into its Rust type"
        );
    }

    // The parser must actually have found the columns; an empty or truncated
    // map would make the seam test pass by finding nothing to disagree with.
    let declared = effective_types();
    assert!(
        declared.len() >= DECODED_COLUMNS.len(),
        "the migration parser found only {} columns for {} decoded fields — it \
         has stopped understanding the DDL: {declared:?}",
        declared.len(),
        DECODED_COLUMNS.len()
    );
}

/// Live parameters sort above dormant ones.
///
/// The grouping is a statement about the system — which knobs do anything today —
/// so it is decided here rather than by whichever client happens to render it.
#[test]
fn the_listing_puts_live_parameters_before_dormant_ones() {
    // `consumed_by IS NOT NULL` sorts false (live) before true (dormant), because
    // Postgres orders false < true.
    let sql = "ORDER BY (consumed_by IS NOT NULL), key";
    assert!(
        std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("src/repositories/pipeline_repository/app_settings.rs"),
        )
        .expect("readable")
        .contains(sql),
        "the listing must group live parameters first: {sql}"
    );
}
