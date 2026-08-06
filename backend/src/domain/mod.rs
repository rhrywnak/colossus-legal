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
pub mod case_state;
pub mod confidence_band;
pub mod fact_role;
pub mod fact_status;
pub mod fact_tier;
pub mod human_authored;
pub mod link_cut;
pub mod llm_params;
pub mod llm_provider_ext;
pub mod quote_match;
pub mod ruling_anchor;
pub mod scenario_code;
pub mod sentence_bounds;
pub mod settings;
pub mod wording;
pub mod wording_accusation;
