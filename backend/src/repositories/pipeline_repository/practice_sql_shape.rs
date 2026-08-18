// What the shipped artifacts SAY: parsing the practice repository's SQL and the
// practice tables' migrations. No assertions live here — the rules they must
// satisfy are in the sibling `practice_schema_tests`, which is also where the
// defect that made both files necessary is written down.
//
// ## Why the parse is deliberately narrow
//
// This is not a SQL parser and must never grow into one. It answers exactly one
// question — "which column names does this statement mention, and of which
// table" — and every shape it cannot answer that for is SKIPPED rather than
// guessed at. A guard that guesses produces false accusations, and a false
// accusation is how a guard gets deleted.
//
// ## Rust Learning: `pub(super)` on a test-only module's helpers
//
// Both this module and `practice_schema_tests` are children of `practice`, so
// `pub(super)` here means "visible inside `practice` and everything under it" —
// which reaches the sibling and stops there. It is the narrowest visibility that
// works, and the same one `scan_runs_tests` uses to lend its helpers to
// `scan_run_start_tests`.

use std::collections::BTreeMap;

/// The shipped source of `practice.rs`.
fn practice_source() -> String {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src/repositories/pipeline_repository/practice.rs");
    std::fs::read_to_string(path).expect("practice.rs is readable")
}

/// Is this a character Postgres would accept inside an unquoted identifier?
pub(super) fn is_ident_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

/// Every SQL string literal in the repository source, with Rust's line-
/// continuation escapes applied.
///
/// ## Rust Learning: `\` at end of line inside a string literal
///
/// `"SELECT id, \` + newline + spaces + `side"` is ONE line in the value the
/// compiler builds: a trailing backslash eats the newline and the indentation
/// that follows it. This test reads the SOURCE as text, so the compiler has not
/// done that yet — the backslash, the newline and the indentation are all still
/// there, and the parse below would see `id,` and `\` as separate tokens. So the
/// escape is applied here, by hand, before anything is parsed.
///
/// Comment lines are dropped first: a `///` line quoting an example query would
/// otherwise be measured as if it shipped.
pub(super) fn sql_statements() -> Vec<String> {
    let source = practice_source();
    let code: String = source
        .lines()
        .filter(|l| !l.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n");

    let mut statements = Vec::new();
    let mut chars = code.chars().peekable();
    while let Some(c) = chars.next() {
        if c != '"' {
            continue;
        }
        // Inside a literal: collect until the closing quote, keeping escapes
        // verbatim so the continuation pass below can see them.
        let mut raw = String::new();
        while let Some(c) = chars.next() {
            match c {
                '"' => break,
                '\\' => {
                    raw.push('\\');
                    if let Some(next) = chars.next() {
                        raw.push(next);
                    }
                }
                other => raw.push(other),
            }
        }

        let upper = raw.to_uppercase();
        let is_sql = ["SELECT ", "INSERT INTO ", "UPDATE ", "DELETE FROM "]
            .iter()
            .any(|k| upper.contains(k));
        if is_sql {
            statements.push(apply_line_continuations(&raw));
        }
    }
    statements
}

/// Turn `\` + newline + indentation into nothing, the way the compiler does.
fn apply_line_continuations(raw: &str) -> String {
    let mut out = String::new();
    let mut chars = raw.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\\' && chars.peek() == Some(&'\n') {
            chars.next();
            while matches!(chars.peek(), Some(' ' | '\t')) {
                chars.next();
            }
            continue;
        }
        out.push(c);
        if c == '\\' {
            if let Some(next) = chars.next() {
                out.push(next);
            }
        }
    }
    out
}

/// Every column one table has, read from the shipped migrations.
///
/// ## Why this reads EVERY migration, not just the CREATE
///
/// A table's real shape is the union of its migrations. `practice_answers` is
/// created by `20260817213319_practice_session_v0.sql` and gains
/// `read_raw_reply` from `20260818082357_…`; reading only the CREATE would
/// declare that column — the one this whole task is about — a typo.
///
/// ## Why the CREATE block ends on a line that is exactly `);`
///
/// Searching for the first `);` anywhere stops inside a `CHECK (...)` or a
/// COMMENT and truncates the column list, which fails the test with a false
/// accusation — the same naive-textual-match defect this file exists to catch.
/// Anchoring on a bare `);` line is what makes it right. Lines that continue a
/// column (`REFERENCES …`, `CHECK (…)`) and table-level `CONSTRAINT …` clauses
/// are all excluded by the same rule: a column line's first token is a bare
/// lowercase identifier, and nothing else in these files is.
pub(super) fn migration_columns(table: &str) -> Vec<String> {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("pipeline_migrations");
    let mut columns = Vec::new();

    let mut files: Vec<_> = std::fs::read_dir(&dir)
        .expect("pipeline_migrations is readable")
        .map(|e| e.expect("dir entry readable").path())
        .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("sql"))
        .collect();
    files.sort();

    for path in files {
        let sql = std::fs::read_to_string(&path).expect("migration is UTF-8");

        if let Some(start) = find_table(&sql, "CREATE TABLE ", table) {
            for line in sql[start..].lines().skip(1) {
                let line = line.trim();
                if line == ");" {
                    break;
                }
                if line.is_empty() || line.starts_with("--") {
                    continue;
                }
                if let Some(name) = line.split_whitespace().next() {
                    if name.chars().all(|c| c.is_ascii_lowercase() || c == '_') {
                        columns.push(name.to_string());
                    }
                }
            }
        }

        for line in sql.lines() {
            let prefix = format!("ALTER TABLE {table} ADD COLUMN ");
            let Some(rest) = line.trim().strip_prefix(&prefix) else {
                continue;
            };
            let rest = rest.strip_prefix("IF NOT EXISTS ").unwrap_or(rest);
            if let Some(name) = rest.split_whitespace().next() {
                columns.push(name.to_string());
            }
        }
    }

    columns.sort();
    columns.dedup();
    columns
}

/// Byte offset where `<keyword><table>` BEGINS — the caller reads forward from
/// it, so the start of the phrase is what it wants.
///
/// The match must END on a boundary: `CREATE TABLE practice_question` would
/// otherwise match inside `CREATE TABLE practice_questions` and quietly measure
/// the wrong table's columns. Hence the check that the next character is not one
/// an identifier could continue with, and the loop that keeps looking when it is.
fn find_table(sql: &str, keyword: &str, table: &str) -> Option<usize> {
    let needle = format!("{keyword}{table}");
    let mut from = 0;
    while let Some(at) = sql[from..].find(&needle) {
        let at = from + at;
        let after = sql[at + needle.len()..].chars().next();
        if !matches!(after, Some(c) if is_ident_char(c)) {
            return Some(at);
        }
        from = at + needle.len();
    }
    None
}

/// The tables in one statement, as `alias → table`, plus every table named.
///
/// `FROM practice_answers a JOIN practice_questions q ON …` yields
/// `{a: practice_answers, q: practice_questions}`. A table with no alias is
/// recorded under its own name, which is also how an unaliased `t.column`
/// reference would be written.
pub(super) fn tables_of(statement: &str) -> (BTreeMap<String, String>, Vec<String>) {
    let tokens: Vec<&str> = statement.split_whitespace().collect();
    let mut aliases = BTreeMap::new();
    let mut tables = Vec::new();

    for (i, token) in tokens.iter().enumerate() {
        let upper = token.to_uppercase();
        if !["FROM", "JOIN", "INTO", "UPDATE"].contains(&upper.as_str()) {
            continue;
        }
        let Some(table) = tokens.get(i + 1) else {
            continue;
        };
        let table = table.trim_end_matches(&[',', '('][..]).to_string();
        if !table.chars().all(is_ident_char) {
            continue;
        }
        tables.push(table.clone());
        // No alias means the table's own name is how a qualified reference to it
        // would be written, so it is recorded under itself.
        let alias = alias_after(&tokens, i).unwrap_or_else(|| table.clone());
        aliases.insert(alias, table);
    }
    (aliases, tables)
}

/// The alias that follows the table at `tokens[i + 1]`, if there is one.
///
/// A keyword is not an alias: `FROM practice_sessions WHERE …` names no `WHERE`
/// table, and reading one would map every column in that statement to a table
/// this file does not know — which is a SKIP, and silence is how a guard goes
/// blind. Hence the explicit list rather than "anything lowercase".
fn alias_after(tokens: &[&str], i: usize) -> Option<String> {
    const NOT_AN_ALIAS: &[&str] = &[
        "ON",
        "WHERE",
        "GROUP",
        "ORDER",
        "LEFT",
        "JOIN",
        "INNER",
        "LIMIT",
        "SET",
        "RETURNING",
        "HAVING",
        "VALUES",
        "AS",
    ];
    tokens
        .get(i + 2)
        .map(|a| a.trim_end_matches(','))
        .filter(|a| !a.is_empty() && a.chars().all(is_ident_char))
        .filter(|a| !NOT_AN_ALIAS.contains(&a.to_uppercase().as_str()))
        .map(str::to_string)
}

/// The comma-separated contents of the parenthesised list starting at `from`.
fn paren_list(statement: &str, from: usize) -> Vec<String> {
    let Some(open) = statement[from..].find('(').map(|i| i + from) else {
        return Vec::new();
    };
    let Some(close) = statement[open..].find(')').map(|i| i + open) else {
        return Vec::new();
    };
    statement[open + 1..close]
        .split(',')
        .map(|c| c.trim().to_string())
        .filter(|c| !c.is_empty())
        .collect()
}

/// `(table, columns, values)` for every `INSERT INTO` in the repository.
pub(super) fn inserts() -> Vec<(String, Vec<String>, Vec<String>)> {
    let mut out = Vec::new();
    for statement in sql_statements() {
        let Some(at) = statement.find("INSERT INTO ") else {
            continue;
        };
        let rest = &statement[at + "INSERT INTO ".len()..];
        let table = rest
            .split_whitespace()
            .next()
            .unwrap_or_default()
            .trim_end_matches('(')
            .to_string();
        let columns = paren_list(&statement, at);
        let values_at = statement.find("VALUES").unwrap_or(statement.len());
        let values = paren_list(&statement, values_at);
        out.push((table, columns, values));
    }
    out
}

/// `(table, assigned columns)` for every `UPDATE … SET` in the repository.
///
/// ASSUMES no assignment's right-hand side contains a comma — `SET x = f(a, b)`
/// would split into two. Nothing in this repository does, and a violation shows
/// up as a loud failure naming the nonsense column, never as a silent pass.
pub(super) fn updates() -> Vec<(String, Vec<String>)> {
    let mut out = Vec::new();
    for statement in sql_statements() {
        let Some(at) = statement.find("UPDATE ") else {
            continue;
        };
        let rest = &statement[at + "UPDATE ".len()..];
        let table = rest
            .split_whitespace()
            .next()
            .unwrap_or_default()
            .to_string();
        let Some(set_at) = statement.find(" SET ") else {
            continue;
        };
        let body = &statement[set_at + " SET ".len()..];
        let body = body.split(" WHERE ").next().unwrap_or(body);
        let columns = body
            .split(',')
            .filter_map(|a| a.split('=').next())
            .map(|c| c.trim().to_string())
            .filter(|c| !c.is_empty() && c.chars().all(is_ident_char))
            .collect();
        out.push((table, columns));
    }
    out
}

/// Every `alias.column` in a statement, in source order.
pub(super) fn qualified_refs(statement: &str) -> Vec<(String, String)> {
    let bytes: Vec<char> = statement.chars().collect();
    let mut refs = Vec::new();
    for (i, c) in bytes.iter().enumerate() {
        if *c != '.' || i == 0 {
            continue;
        }
        let mut start = i;
        while start > 0 && is_ident_char(bytes[start - 1]) {
            start -= 1;
        }
        let mut end = i + 1;
        while end < bytes.len() && is_ident_char(bytes[end]) {
            end += 1;
        }
        if start == i || end == i + 1 {
            continue;
        }
        refs.push((
            bytes[start..i].iter().collect(),
            bytes[i + 1..end].iter().collect(),
        ));
    }
    refs
}

/// Is this a table this file claims cover for?
pub(super) fn is_practice_table(table: &str) -> bool {
    table.starts_with("practice_")
}

/// The columns every practice table declares, for the assertion messages.
pub(super) fn declared() -> BTreeMap<String, Vec<String>> {
    [
        "practice_questions",
        "practice_point_receipts",
        "practice_sessions",
        "practice_answers",
    ]
    .iter()
    .map(|t| ((*t).to_string(), migration_columns(t)))
    .collect()
}
