pub mod allegation;
pub mod analysis;
pub mod case_dto;
pub mod claim;
pub mod contradiction;
pub mod decision;
pub mod document;
pub mod evidence;
pub mod evidence_chain;
pub mod decomposition;
pub mod graph;
pub mod case_summary;
pub mod harm;
pub mod hearing;
pub mod motion_claim;
pub mod person;
pub mod person_detail;
pub mod schema;

// Re-export DTOs / request types from submodules
pub use allegation::{AllegationDto, AllegationSummary, AllegationsResponse};
pub use harm::{HarmDto, HarmsResponse};
pub use claim::{ClaimCreateRequest, ClaimDto, ClaimUpdateRequest};
pub use contradiction::{ContradictionDto, ContradictionEvidence, ContradictionsResponse};
pub use decision::{DecisionCreateRequest, DecisionUpdateRequest};
pub use document::{DocumentCreateRequest, DocumentDto, DocumentUpdateRequest};
pub use evidence::{EvidenceCreateRequest, EvidenceDto, EvidenceResponse, EvidenceUpdateRequest};
pub use hearing::{HearingCreateRequest, HearingUpdateRequest};
pub use person::{PersonCreateRequest, PersonDto, PersonUpdateRequest, PersonsResponse};
pub use motion_claim::{MotionClaimDto, MotionClaimsResponse};
pub use evidence_chain::{
    ChainAllegation, ChainDocument, ChainSummary, EvidenceChainResponse,
    EvidenceWithDocument, MotionClaimWithEvidence,
};
pub use graph::{GraphEdge, GraphNode, GraphNodeType, GraphResponse};
pub use schema::SchemaResponse;
pub use case_dto::{CaseInfo, CaseResponse, CaseStats, LegalCountSummary, PartiesGroup, PartyDto};
pub use analysis::{
    AllegationStrength, AnalysisResponse, ContradictionBrief, ContradictionsSummary,
    DocumentCoverage, EvidenceCoverage, GapAnalysis,
};
pub use decomposition::{
    AllegationDetailResponse, AllegationOverview, DecompositionResponse,
    RebuttalsResponse,
};