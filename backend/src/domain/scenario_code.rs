// =============================================================================
// backend/src/domain/scenario_code.rs — the scenario's human handle (S-n)
// =============================================================================
//
// §2a (RATIFIED 2026-08-01) gives every scenario a short stable handle — S-1,
// S-2, … — assigned at creation, never changed, never reused, displayed wherever
// the scenario appears. This module owns how the stored ordinal becomes the
// string a human reads and says out loud.
//
// ## Why the BACKEND formats it
//
// The sibling candidate handle (`C-14`) is formatted in the frontend by
// `candidateChip`. This one deliberately is not, and the difference is the
// standing rule: no business logic in the frontend. A handle's spelling is part
// of the case's vocabulary — it appears in a rehearsal note, a margin scribble,
// and eventually a printed exhibit list — so the prefix belongs in one place that
// every surface reads, not re-typed in each component. The frontend receives
// `"S-3"` and renders it; it never learns that the separator is a hyphen or that
// the prefix is "S".
//
// (The candidate chip is not being changed here — rewiring a shipped, tested
// display is not this task, and its formatting is at least centralized in one
// helper. Worth aligning when the card work lands in 1.3.)

/// The prefix distinguishing a scenario handle from a candidate handle (`C-`).
///
/// ## Rust Learning: a `const` for a vocabulary token, not a config value
///
/// Standing Rule 2 asks whether a value varies across environments, cases or
/// deployments. This one cannot: `S-3` is a name humans have already said aloud,
/// and a deployment that rendered it `SC-3` would be renaming every reference in
/// every notebook. It is law, versioned with the code — the same reasoning that
/// keeps the connection-tier partition out of YAML.
const SCENARIO_CODE_PREFIX: &str = "S-";

/// Render a scenario's stored ordinal as its display handle.
///
/// ## Why this takes the ordinal rather than the whole record
///
/// The formatting depends on exactly one field, so taking only that field means
/// this function is callable from a DTO builder that holds a record, from a log
/// line that holds a number, and from a test that holds neither — with no struct
/// to construct. It also makes the function trivially total: every `i32` maps to a
/// string, so there is no error path and no `Option` for a caller to unwrap.
///
/// Negative ordinals are unreachable — the column carries a `CHECK`-backed
/// non-negative sequence and is assigned only by `insert_scenario` — so they get
/// no special handling here. If one ever appeared it would render as `S--1`,
/// which is visibly wrong rather than silently plausible, and that is the right
/// failure mode for an impossible value.
pub fn scenario_code(ordinal: i32) -> String {
    format!("{SCENARIO_CODE_PREFIX}{ordinal}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_the_ratified_shape() {
        // The exact strings humans will say and write. Pinned so a well-meaning
        // edit to zero-pad or bracket them fails the build — every such change
        // silently invalidates references already made outside this system.
        assert_eq!(scenario_code(1), "S-1");
        assert_eq!(scenario_code(3), "S-3");
        assert_eq!(scenario_code(42), "S-42");
    }

    #[test]
    fn does_not_pad_or_truncate_large_ordinals() {
        assert_eq!(scenario_code(100), "S-100");
        assert_eq!(scenario_code(9999), "S-9999");
    }

    #[test]
    fn is_distinguishable_from_a_candidate_handle() {
        // S-14 and C-14 name completely different things — a scenario and a
        // candidate fact inside one. If the prefixes ever collided, a rehearsal
        // reference would be ambiguous, so assert they differ rather than trusting
        // two constants in two files to stay apart.
        assert!(scenario_code(14).starts_with("S-"));
        assert_ne!(scenario_code(14), "C-14");
    }
}
