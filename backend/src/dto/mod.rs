pub mod claim;
pub mod decision;
pub mod document;
pub mod evidence;
pub mod hearing;
pub mod person;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DocumentDto {
    pub id: String,
    pub title: String,
    pub doc_type: String,         // e.g. "pdf", "motion", "ruling", "evidence", "filing"
    pub created_at: Option<String>, // ISO-8601 string or None
}

pub use claim::{ClaimCreateRequest, ClaimDto, ClaimUpdateRequest};
pub use decision::{DecisionCreateRequest, DecisionUpdateRequest};
pub use document::{DocumentCreateRequest, DocumentUpdateRequest};
pub use evidence::{EvidenceCreateRequest, EvidenceUpdateRequest};
pub use hearing::{HearingCreateRequest, HearingUpdateRequest};
pub use person::{PersonCreateRequest, PersonUpdateRequest};

