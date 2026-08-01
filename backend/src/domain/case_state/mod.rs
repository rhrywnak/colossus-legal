// =============================================================================
// backend/src/domain/case_state — the case-state module family
// =============================================================================
//
// This module family is the ONLY place case-state computation may live. "Case
// state" means every structural judgement the product makes about how the case
// stands: which Evidence is wired into an Allegation and how strongly (L1), and
// — when later phases build them — the readiness verdicts, hazard/ammunition
// findings and conservation identities computed on top of that (L2). Those two
// are separate LAYERS inside this family, never separate homes: L1 partitions
// the graph's edge classes and stops; L2 reads L1's output types and renders
// judgement. Nothing outside `case_state` may do either.
//
// ## Why: the family exists before its tenants
//
// The alternative was to let verdict code land beside scenario code and relocate
// it later. That costs rework, and — worse — it leaves an interval in which a
// second, third or fourth definition of "connected" can grow in whatever module
// happened to need one. Establishing the family first means every later tenant
// arrives into a structure that already has an answer to "where does this go?"
//
// ## The layering law (ratified 2026-08-01)
//
// Verdicts consume only the partition's OUTPUT TYPES — connection tiers are
// newtypes, and the raw edge-class tier sets are private within `partition.rs`,
// invisible to every other module. No code outside `case_state` can name a tier
// set; therefore no future surface can define a connection notion without going
// through the partition.
//
// The enforcement is the Rust visibility system, not this comment: the tier
// lists carry no `pub`, so a module that tries to re-derive "probative" from
// them does not compile. `visibility_invariants` exists to PIN that fact — it
// fails loudly if the visibility ever widens or if a tier-set name appears
// outside this family — because a `pub` added in a hurry is a one-character
// change that review can miss and the compiler would then happily accept.
//
// Note the law's scope: it governs notions of CONNECTEDNESS, not the schema
// vocabulary. `neo4j::schema`'s relationship constants remain freely nameable
// by write paths and by ordinary Cypher traversals — those spell an edge type
// because they are walking the graph, not because they are deciding what counts
// as connected. It is the tier SETS that constitute the definition, and it is
// the tier sets that are imprisoned here.

/// L1 — the connection-tier partition: which Evidence→Allegation edge classes
/// count as a connection, and at which tier.
pub mod partition;

#[cfg(test)]
mod visibility_invariants;
