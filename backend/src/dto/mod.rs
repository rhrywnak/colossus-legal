pub mod claim;
pub mod decision;
pub mod document;
pub mod evidence;
pub mod hearing;
pub mod person;
pub mod schema;

// Re-export DTOs / request types from submodules
pub use claim::{ClaimCreateRequest, ClaimDto, ClaimUpdateRequest};
pub use decision::{DecisionCreateRequest, DecisionUpdateRequest};
pub use document::{DocumentCreateRequest, DocumentDto, DocumentUpdateRequest};
pub use evidence::{EvidenceCreateRequest, EvidenceUpdateRequest};
pub use hearing::{HearingCreateRequest, HearingUpdateRequest};
pub use person::{PersonCreateRequest, PersonUpdateRequest};
pub use schema::SchemaResponse;
