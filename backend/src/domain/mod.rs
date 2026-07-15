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

pub mod actor_role;
pub mod fact_role;
pub mod fact_status;
pub mod llm_params;
