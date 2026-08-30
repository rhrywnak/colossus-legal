//! ⚑ THE ROUTE-TABLE WALK — the identity proof behind every router refactor.
//!
//! Written for the `api/mod.rs` split (timeline subsets, T1.0). Moving eleven
//! route-group functions into their own modules is a refactor whose ONLY
//! promised property is that nothing moved: the same methods on the same paths,
//! before and after. `cargo check` cannot see a path that lost a segment, and
//! neither can a reviewer reading a 400-line diff of moved code. This does.
//!
//! ## Why it reads `Debug` and not an axum API
//!
//! axum 0.7 exposes no way to enumerate a `Router`'s routes — there is no
//! `routes()`, no iterator, no introspection trait. What it does have is a
//! `Debug` impl that prints the whole `PathRouter`: a `RouteId → MethodRouter`
//! map carrying each route's `allow_header` (the exact method set axum will
//! answer for), and a `RouteId → path` map beside it. Joining the two by id is
//! the real table, taken from the real router, not from a scan of the source
//! that declares it.
//!
//! That is a private formatting detail of a dependency, and this file says so
//! rather than pretending otherwise. The mitigation is the vacuity guard below:
//! if an axum upgrade changes the shape, the parse yields nothing or loses its
//! sentinels and the test FAILS. It cannot quietly start proving less — which is
//! the only failure mode that would matter, because a walker that silently
//! returned an empty table would make every future split look identical to
//! every other.
//!
//! ## Rust Learning: `format!("{:?}")` as a data source
//!
//! Rust's `Debug` is for programmers, not for parsers, and using it as a data
//! source is normally a mistake — the output is not a stable contract. It is
//! defensible HERE, and only here, because the alternative is no proof at all
//! and because the consumer is a test: the cost of the format changing is a red
//! test on the next `cargo update`, not a broken endpoint in production.

use std::collections::BTreeMap;

/// Where the real router's table stops and the fallback router's begins.
///
/// The `Debug` output prints `path_router` first and `fallback_router` after it,
/// and the fallback carries two entries of its own (`/` and axum's private
/// catch-all). Truncating here is what keeps them out of the table — they are
/// not routes anybody declared.
const FALLBACK_MARKER: &str = "fallback_router:";

/// Every `(method, path)` the API router will answer, sorted.
///
/// One line per METHOD, not per route: a route that quietly lost its `.delete()`
/// while keeping its `.get()` shows up as exactly one missing line, which is the
/// granularity a refactor needs to be judged at.
fn route_table() -> Vec<String> {
    let router = super::router();
    let rendered = format!("{router:?}");
    let table = match rendered.find(FALLBACK_MARKER) {
        Some(at) => &rendered[..at],
        // No marker means the format moved. Parse the whole string rather than
        // silently returning nothing: the vacuity guard is what reports it.
        None => rendered.as_str(),
    };

    let methods = methods_by_route_id(table);
    let paths = paths_by_route_id(table);

    let mut out = Vec::new();
    for (id, path) in &paths {
        // A path with no method entry is not skipped — it is reported with a
        // placeholder, because "axum knows a path this walk cannot describe" is
        // exactly the kind of silence Standing Rule 1 forbids.
        let allowed = methods
            .get(id)
            .map(String::as_str)
            .unwrap_or("(no-method-router)");
        for method in allowed.split(',').filter(|m| !m.is_empty()) {
            out.push(format!("{method} {path}"));
        }
    }
    out.sort();
    out
}

/// Everything printed for ONE `RouteId(n)` entry, and the id itself.
///
/// ## Why a span and not a search of the whole string
///
/// Both maps below look for a marker "after the id". Searching the rest of the
/// output would find the NEXT entry's marker whenever this entry has none — so a
/// path with no method router would silently inherit its neighbour's methods,
/// and the walk would report a table that is wrong in exactly the way it exists
/// to catch. A `MethodRouter`'s own `Debug` output contains no nested
/// `RouteId(`, so cutting at the next one bounds each entry exactly.
///
/// ## Rust Learning: returning a `&str` borrowed from the argument
///
/// The returned slice points INTO `table`; no copy is made, and the compiler
/// ties its lifetime to the input so it cannot outlive the string it describes.
/// That is why the two callers below can build maps of `String` from spans
/// without ever holding a dangling reference.
fn entries(table: &str) -> impl Iterator<Item = (u64, &str)> {
    table.match_indices("RouteId(").filter_map(|(at, _)| {
        let rest = &table[at + "RouteId(".len()..];
        let id = leading_number(rest)?;
        let span = match rest.find("RouteId(") {
            Some(next) => &rest[..next],
            None => rest,
        };
        Some((id, span))
    })
}

/// `RouteId(n): MethodRouter(… allow_header: Bytes(b"GET,HEAD") …)` → `n → "GET,HEAD"`.
///
/// `allow_header` is axum's own answer to "what will this route accept", which
/// is why it is read rather than the nine `get:`/`post:`/… fields beside it: it
/// is the value axum itself would put in a 405's `Allow` header.
fn methods_by_route_id(table: &str) -> BTreeMap<u64, String> {
    let mut out = BTreeMap::new();
    for (id, span) in entries(table) {
        let Some((_, after)) = span.split_once("allow_header: Bytes(b\"") else {
            continue;
        };
        let Some((allowed, _)) = after.split_once('"') else {
            continue;
        };
        out.insert(id, allowed.to_string());
    }
    out
}

/// `RouteId(n): "/some/path"` → `n → "/some/path"`.
fn paths_by_route_id(table: &str) -> BTreeMap<u64, String> {
    let mut out = BTreeMap::new();
    for (id, span) in entries(table) {
        // The path form is `RouteId(n): "…"` — the quote follows the colon
        // immediately, so anything else (a MethodRouter, a Route) is not a path.
        let Some((_, tail)) = span.split_once("): ") else {
            continue;
        };
        let Some(quoted) = tail.strip_prefix('"') else {
            continue;
        };
        let Some((path, _)) = quoted.split_once('"') else {
            continue;
        };
        out.insert(id, path.to_string());
    }
    out
}

/// The digits at the front of `rest`, as a number.
fn leading_number(rest: &str) -> Option<u64> {
    let end = rest.find(|c: char| !c.is_ascii_digit())?;
    rest[..end].parse().ok()
}

/// How many `(method, path)` lines the router carries at this commit.
///
/// Pinned deliberately. The T1.0 split promised that this number and every line
/// behind it survive the move unchanged; a task that ADDS endpoints edits this
/// number in the same commit that adds them, which is the point — a route
/// appearing or vanishing becomes a line in a diff somebody signed.
///
/// 288 at the T1.0 split. 301 since T1.3, which added thirteen lines: the nine
/// timeline-subset routes, plus the `HEAD` axum pairs with each of the three
/// `GET`s and the `PUT`/`DELETE`/`POST` that share their paths.
// Tests are allowed literal expected values: this one IS the invariant.
const EXPECTED_ROUTE_LINES: usize = 301;

#[test]
fn the_route_table_is_exactly_what_this_commit_declares() {
    let table = route_table();
    assert_eq!(
        table.len(),
        EXPECTED_ROUTE_LINES,
        "the router's (method, path) table changed. If that was deliberate, \
         update EXPECTED_ROUTE_LINES in the same commit and say which routes \
         moved; if it was not, a route was lost or gained by accident:\n{}",
        table.join("\n")
    );
}

#[test]
fn the_walk_can_actually_see_the_router_it_claims_to_read() {
    // ⚑ THE VACUITY GUARD. Every assertion here is satisfiable by finding
    // nothing, so an axum upgrade that changed the Debug shape would leave a
    // green test walking an empty table. These five lines are declared in five
    // different route groups, with five different methods between them; a parse
    // that lost any of them has stopped working and says so.
    let table = route_table();
    for sentinel in [
        "GET /me",
        "GET /timeline",
        "POST /timeline/events",
        "DELETE /timeline/events/:id",
        "PATCH /qa/:id/rate",
        "PUT /claims/:id",
        "GET /admin/pipeline/models",
        // T1.3's own group, one sentinel per path family, so a subset route
        // lost in a later refactor is named rather than counted.
        "POST /timeline/subsets",
        "PUT /timeline/subsets/:id/events",
        "DELETE /cases/:slug/scenarios/:scenario_id/subsets/:subset_id",
    ] {
        assert!(
            table.iter().any(|line| line == sentinel),
            "the route walk did not find {sentinel:?} — it read {} lines, which \
             means axum's Debug shape moved and this proof has stopped proving",
            table.len(),
        );
    }
}

#[test]
fn every_path_the_router_holds_names_at_least_one_method() {
    // The placeholder `route_table` emits for a path with no method router. It
    // has never appeared; if it does, the walk is describing a router shape this
    // code does not understand, and that must be loud rather than absent.
    let table = route_table();
    let orphans: Vec<&String> = table
        .iter()
        .filter(|line| line.starts_with("(no-method-router)"))
        .collect();
    assert!(
        orphans.is_empty(),
        "these paths carry no method router, so the walk cannot say what they \
         answer:\n  {orphans:?}"
    );
}

/// Print the whole table, for pasting into a refactor's identity proof.
///
/// `#[ignore]` because it asserts nothing — it is a reporting tool, and a suite
/// run should not carry 236 lines of output. Run it on purpose:
///
/// ```text
/// cargo test --lib dump_the_route_table -- --ignored --nocapture
/// ```
#[test]
#[ignore = "reporting tool, not an assertion — run with --ignored --nocapture"]
fn dump_the_route_table() {
    for line in route_table() {
        println!("{line}");
    }
}
