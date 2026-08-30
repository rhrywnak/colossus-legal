// =============================================================================
// backend/src/domain — case-agnostic domain vocabularies (code-owned lookups)
// =============================================================================
//
// Home for small, versioned vocabularies that are OWNED BY CODE rather than by a
// database enum or by string literals scattered across match arms. The first
// resident is `actor_role` (D1): the {originated, repeated, adopted} roles a
// party can play in an accusation chain.
//
// Why a `domain` module and not `dto`? These are not wire shapes — they are the
// case-agnostic vocabulary that wire DTOs (e.g. `ScenarioDefinition`'s `Wielder`)
// are validated against. Keeping them here means the same lookup can be reused by
// later work (task 1.3's fact-role vocabulary mirrors the pattern `actor_role`
// establishes) without reaching into the CRUD dto layer.
//
// `case_state` is the one resident that is a FAMILY rather than a flat lookup: it
// is the sole home for case-state computation (the connection-tier partition
// today, readiness verdicts later), and it carries a visibility law its own
// `mod.rs` documents. Vocabularies stay flat here; anything that decides how the
// case STANDS goes inside `case_state`.

pub mod actor_role;
pub mod billing_class;
pub mod card_language;
pub mod case_phase;
pub mod case_state;
pub mod chronology;
pub mod confidence_band;
pub mod date_precision;
pub mod evidence_tier;
pub mod fact_role;
pub mod fact_status;
pub mod fact_tier;
pub mod human_authored;
pub mod link_cut;
/// The Messages API `effort` dial and which call families turn it down —
/// the 2026-08-28 all-reasoning-blocks incident's fix.
pub mod llm_effort;
pub mod llm_params;
pub mod llm_provider_ext;
pub mod practice_params;
/// The verifier's second-chance matcher — numeral stripping and one-gap
/// matching, for quotes the contiguous matcher cannot find. Split from
/// `quote_match` so the strict path stays readable on its own.
pub mod quote_gap;
pub mod quote_match;
pub mod rehearsal_shape;
pub mod ruling_anchor;
pub mod scenario_code;
pub mod sentence_bounds;
pub mod settings;
pub mod wording;
pub mod wording_accusation;
pub mod wording_authoring;
pub mod wording_card_grammar;
pub mod wording_chronology;
// The stored KEYS the block above reads. Split out for Rule 17 when T1.2
// declared the subsets words — see that module's header.
pub mod wording_chronology_keys;
pub mod wording_matrix;
pub mod wording_model_params;
pub mod wording_practice;
pub mod wording_practice_editor;
pub mod wording_practice_flow;
pub mod wording_practice_list;
pub mod wording_practice_print;
pub mod wording_practice_report;
pub mod wording_practice_row;
pub mod wording_rehearsal;
pub mod wording_rehearsal_chrome;
pub mod wording_scan;
pub mod wording_scenario_authoring;
pub mod wording_templates;
pub mod wording_war_room;
