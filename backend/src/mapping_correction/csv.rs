//! Minimal CSV parser for `--from-file` batch input.
//!
//! Format: one correction per line, `op,from,to`, where `op` is
//! `add` | `delete` | `promote`. Blank lines and lines beginning with `#` are
//! ignored (comments). An optional header line `op,from,to` is also skipped.
//! Hand-rolled (no `csv` crate) because the grammar is three trimmed,
//! comma-separated fields with no embedded commas or quoting — pulling in a
//! dependency for that would be overkill.

use super::{MappingError, OpRequest, Operation};

/// Parse the full contents of a batch file into a list of requests. Fails fast
/// on the first malformed line, naming the 1-based line number (Standing
/// Rule 1: an operator typo is a distinct, located error, not a silent skip).
pub fn parse(contents: &str) -> Result<Vec<OpRequest>, MappingError> {
    let mut requests = Vec::new();
    for (idx, raw) in contents.lines().enumerate() {
        let line_no = idx + 1;
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if is_header(line) {
            continue;
        }
        requests.push(parse_line(line, line_no)?);
    }
    Ok(requests)
}

/// A line is the optional column header if its three fields are exactly the
/// column names — skip it rather than try to parse `op` as an operation.
fn is_header(line: &str) -> bool {
    let fields: Vec<&str> = line.split(',').map(str::trim).collect();
    fields == ["op", "from", "to"]
}

/// Parse one non-comment line into an [`OpRequest`].
fn parse_line(line: &str, line_no: usize) -> Result<OpRequest, MappingError> {
    let fields: Vec<&str> = line.split(',').map(str::trim).collect();
    if fields.len() != 3 {
        return Err(MappingError::Csv {
            line: line_no,
            message: format!("expected 3 comma-separated fields (op,from,to), got {}", fields.len()),
        });
    }
    let op = parse_op(fields[0], line_no)?;
    let from = fields[1];
    let to = fields[2];
    if from.is_empty() || to.is_empty() {
        return Err(MappingError::Csv {
            line: line_no,
            message: "from/to ids must not be empty".to_string(),
        });
    }
    Ok(OpRequest {
        op,
        from: from.to_string(),
        to: to.to_string(),
    })
}

/// Parse the operation token, case-insensitively.
fn parse_op(token: &str, line_no: usize) -> Result<Operation, MappingError> {
    match token.to_ascii_lowercase().as_str() {
        "add" => Ok(Operation::Add),
        "delete" => Ok(Operation::Delete),
        "promote" => Ok(Operation::Promote),
        other => Err(MappingError::Csv {
            line: line_no,
            message: format!("unknown op '{other}' (expected add|delete|promote)"),
        }),
    }
}

// ── Tests ─────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_three_ops_skipping_comments_blanks_and_header() {
        let input = "\
# B2 corrections batch
op,from,to

add,allegation-7,element-1-1
delete, allegation-9 , element-2-3
PROMOTE,allegation-12,element-1-2
";
        let reqs = parse(input).expect("parses");
        assert_eq!(reqs.len(), 3);
        assert_eq!(reqs[0], OpRequest { op: Operation::Add, from: "allegation-7".into(), to: "element-1-1".into() });
        // Whitespace around fields is trimmed.
        assert_eq!(reqs[1], OpRequest { op: Operation::Delete, from: "allegation-9".into(), to: "element-2-3".into() });
        // Op token is case-insensitive.
        assert_eq!(reqs[2].op, Operation::Promote);
    }

    #[test]
    fn rejects_wrong_field_count_with_line_number() {
        let err = parse("add,only-two").expect_err("must fail");
        match err {
            MappingError::Csv { line, .. } => assert_eq!(line, 1),
            other => panic!("expected Csv error, got {other:?}"),
        }
    }

    #[test]
    fn rejects_unknown_op_naming_the_line() {
        // line 1 is a comment, line 2 is the bad op → error must report line 2.
        let err = parse("# header comment\nfrobnicate,a,b").expect_err("must fail");
        match err {
            MappingError::Csv { line, message } => {
                assert_eq!(line, 2);
                assert!(message.contains("frobnicate"), "message names the bad token");
            }
            other => panic!("expected Csv error, got {other:?}"),
        }
    }

    #[test]
    fn rejects_empty_ids() {
        let err = parse("add,,element-1-1").expect_err("must fail");
        assert!(matches!(err, MappingError::Csv { line: 1, .. }));
    }
}
