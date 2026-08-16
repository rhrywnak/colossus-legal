//! The party census and the rulings template generated from it.
//!
//! ## What this is for
//!
//! Roman rules the clusters in one sitting. Everything he needs to rule one —
//! what the node is called, whether it is a Person or an Organization, how many
//! sworn statements hang off it, which documents it came from, what other names
//! the extraction recorded for it — has to be on the page in front of him, or the
//! session turns into a series of graph queries.
//!
//! ## The reading aid, and its exact authority: none
//!
//! The generated file groups parties that share a name token, so `Karen A. Tighe`
//! and `Tighe` land next to each other. That grouping is a SORT ORDER and nothing
//! else. Every generated block is pre-filled `SKIP`, so a template returned
//! unedited merges nothing at all; a party may appear under more than one
//! suggested heading, and if Roman rules it in two the parser refuses the file
//! by name and line number.
//!
//! This is the boundary the P7 addendum draws — no fuzzy matching in the tool —
//! honoured exactly: the SUGGESTION may be fuzzy, the EXECUTION reads only what a
//! human typed.

use std::collections::BTreeMap;
use std::fmt::Write as _;

/// One party node as the graph reports it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PartyNode {
    pub id: String,
    /// `Person` or `Organization`.
    pub label: String,
    /// `party_name` where set, else `name`. Both are present on 60 of 62 nodes;
    /// two Organizations carry only `name`.
    pub display_name: String,
    /// Sworn statements attributed to this node — incoming `STATED_BY` from
    /// `Evidence`. This is the number the merge must conserve.
    pub statement_count: u64,
    pub source_documents: Vec<String>,
    pub aliases: Vec<String>,
}

/// Name particles that carry no identity and must not group two parties.
///
/// Generic English name furniture and entity suffixes — not case vocabulary. A
/// list that included case-specific words would be the kind of hardcoding
/// Standing Rule 2 forbids; "mr" and "llc" are neither.
const NON_IDENTIFYING_TOKENS: &[&str] = &[
    "mr",
    "mrs",
    "ms",
    "miss",
    "dr",
    "judge",
    "justice",
    "hon",
    "honorable",
    "the",
    "of",
    "and",
    "for",
    "a",
    "an",
    "attorney",
    "llc",
    "pllc",
    "inc",
    "co",
    "corp",
    "company",
    "cj",
    "jr",
    "sr",
    "ii",
    "iii",
];

/// Split a display name into identity-bearing lowercase tokens.
///
/// ## Rust Learning: `char::is_alphanumeric` as the filter
///
/// `retain` on a `String` keeps the characters the closure accepts, in place.
/// Dropping punctuation this way turns `Penzien & McBride, PLLC` into three clean
/// tokens without a regex, and turns `C.J.` into `cj`, which the stop list then
/// removes.
pub fn identity_tokens(display_name: &str) -> Vec<String> {
    display_name
        .split_whitespace()
        .map(|raw| {
            let mut token: String = raw.to_lowercase();
            token.retain(char::is_alphanumeric);
            token
        })
        // A single character is an initial, and initials group strangers.
        .filter(|t| t.chars().count() > 1)
        .filter(|t| !NON_IDENTIFYING_TOKENS.contains(&t.as_str()))
        .collect()
}

/// Parties grouped by a shared name token — the reading aid, nothing more.
///
/// Returns `(token, party ids)` for every token held by two or more parties,
/// ordered by token so two runs of unchanged data produce identical files.
pub fn suggested_groups(parties: &[PartyNode]) -> Vec<(String, Vec<String>)> {
    let mut by_token: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for party in parties {
        for token in identity_tokens(&party.display_name) {
            let ids = by_token.entry(token).or_default();
            // A name like "Awad Awad" must not list its party twice.
            if !ids.contains(&party.id) {
                ids.push(party.id.clone());
            }
        }
    }
    by_token
        .into_iter()
        .filter(|(_, ids)| ids.len() > 1)
        .map(|(token, mut ids)| {
            ids.sort();
            (token, ids)
        })
        .collect()
}

/// Render the rulings template Roman fills in.
pub fn render_template(parties: &[PartyNode]) -> String {
    let mut out = String::new();
    render_header(&mut out, parties);
    render_suggestions(&mut out, parties);
    render_full_census(&mut out, parties);
    out
}

fn render_header(out: &mut String, parties: &[PartyNode]) {
    let persons = parties.iter().filter(|p| p.label == "Person").count();
    let orgs = parties.len() - persons;
    let with_statements = parties.iter().filter(|p| p.statement_count > 0).count();

    out.push_str(
        "# PARTY MERGE RULINGS — fill this in, then run:\n\
         #     cargo run --bin merge_parties -- --rulings THIS_FILE            (dry run)\n\
         #     cargo run --bin merge_parties -- --rulings THIS_FILE --apply    (writes)\n\
         #\n\
         # FORMAT — four keywords, no punctuation, no indentation rules:\n\
         #\n\
         #     CLUSTER <any label you like>\n\
         #     SURVIVOR <node id that survives>\n\
         #     MERGE <node id that merges into it>      (repeat for each)\n\
         #     END\n\
         #\n\
         #   or, to leave a cluster alone:\n\
         #\n\
         #     CLUSTER <label>\n\
         #     SKIP <why — required, it is the record of the decision>\n\
         #     END\n\
         #\n\
         # EVERY BLOCK BELOW IS PRE-FILLED WITH SKIP. A file returned unedited\n\
         # merges NOTHING. To merge a cluster, delete its SKIP line and write a\n\
         # SURVIVOR line plus one MERGE line per node.\n\
         #\n\
         # The groupings are a READING AID — parties that share a name token are\n\
         # listed together so you are not hunting. They are not a judgement and\n\
         # they are not fuzzy matching by the tool: nothing merges that you have\n\
         # not named. A party may appear under more than one heading; if you rule\n\
         # the same node in two blocks the tool refuses the whole file and says\n\
         # which node and which lines.\n\
         #\n",
    );
    let _ = writeln!(
        out,
        "# CENSUS: {} parties — {persons} Person, {orgs} Organization. \
         {with_statements} carry statements;\n# the rest are mentioned-but-silent, \
         which is why the People page looks fragmented.\n",
        parties.len()
    );
}

fn render_suggestions(out: &mut String, parties: &[PartyNode]) {
    let groups = suggested_groups(parties);
    let _ = writeln!(
        out,
        "# ══ SUGGESTED GROUPS ({} shared-name-token groups) ══════════════════\n",
        groups.len()
    );
    for (token, ids) in groups {
        let _ = writeln!(out, "CLUSTER {}", token.to_uppercase());
        for id in &ids {
            if let Some(party) = parties.iter().find(|p| &p.id == id) {
                render_party_comment(out, party);
            }
        }
        let _ = writeln!(
            out,
            "SKIP not yet ruled — replace this line with SURVIVOR/MERGE to merge\nEND\n"
        );
    }
}

/// Every party, including the ones no suggestion caught.
///
/// A group the token heuristic misses — `Camille Handley` beside
/// `Camille Hanley` is caught by the shared first name, but two names that
/// differ in every token would not be — has to be hand-written, and it can only
/// be hand-written from a list that holds everything.
fn render_full_census(out: &mut String, parties: &[PartyNode]) {
    let _ = writeln!(
        out,
        "# ══ FULL CENSUS — every party, alphabetical ═════════════════════════\n\
         # Nothing below is a block. Copy ids from here to hand-write a cluster\n\
         # the groupings above did not suggest.\n"
    );
    let mut sorted: Vec<&PartyNode> = parties.iter().collect();
    sorted.sort_by(|a, b| a.display_name.cmp(&b.display_name).then(a.id.cmp(&b.id)));
    for party in sorted {
        render_party_comment(out, party);
    }
}

/// One party's facts, as comment lines so they survive inside a block.
fn render_party_comment(out: &mut String, party: &PartyNode) {
    let _ = writeln!(
        out,
        "#   {} · {} · {} statement(s)",
        party.id, party.label, party.statement_count
    );
    let _ = writeln!(out, "#     name     : {}", party.display_name);
    let _ = writeln!(
        out,
        "#     documents: {}",
        if party.source_documents.is_empty() {
            "—".to_string()
        } else {
            party.source_documents.join(", ")
        }
    );
    let _ = writeln!(
        out,
        "#     aliases  : {}",
        if party.aliases.is_empty() {
            "—".to_string()
        } else {
            party.aliases.join(", ")
        }
    );
}

#[cfg(test)]
#[path = "census_tests.rs"]
mod tests;
