pub mod allegation;
pub mod analysis;
pub mod case_dto;
pub mod case_header;
pub mod case_summary;
pub mod causes_of_action;
pub mod claim;
pub mod contradiction;
pub mod decision;
pub mod decomposition;
pub mod document;
pub mod evidence;
pub mod evidence_chain;
pub mod graph;
pub mod harm;
pub mod hearing;
pub mod motion_claim;
pub mod person;
pub mod person_detail;
pub mod proof_matrix;
pub mod proof_review;
pub mod query;
pub mod scenario;
pub mod scenario_crud;
pub mod scenario_facts;
pub mod schema;
pub mod theme_scan;
pub mod trial_prep;

// Re-export DTOs / request types from submodules
pub use allegation::{AllegationDto, AllegationSummary, AllegationsResponse};
pub use analysis::{
    AllegationStrength, AnalysisResponse, ContradictionBrief, ContradictionsSummary,
    DocumentCoverage, EvidenceCoverage, GapAnalysis,
};
pub use case_dto::{CaseInfo, CaseResponse, CaseStats, LegalCountSummary, PartiesGroup, PartyDto};
pub use claim::{ClaimCreateRequest, ClaimDto, ClaimUpdateRequest};
pub use contradiction::{ContradictionDto, ContradictionEvidence, ContradictionsResponse};
pub use decision::{DecisionCreateRequest, DecisionUpdateRequest};
pub use decomposition::{
    AllegationDetailResponse, AllegationOverview, DecompositionResponse, RebuttalsResponse,
};
pub use document::{DocumentCreateRequest, DocumentDto, DocumentUpdateRequest};
pub use evidence::{EvidenceCreateRequest, EvidenceDto, EvidenceResponse, EvidenceUpdateRequest};
pub use evidence_chain::{
    ChainAllegation, ChainDocument, ChainSummary, EvidenceChainResponse, EvidenceWithDocument,
    MotionClaimWithEvidence,
};
pub use graph::{GraphEdge, GraphNode, GraphNodeType, GraphResponse};
pub use harm::{HarmDto, HarmsResponse};
pub use hearing::{HearingCreateRequest, HearingUpdateRequest};
pub use motion_claim::{MotionClaimDto, MotionClaimsResponse};
pub use person::{PersonCreateRequest, PersonDto, PersonUpdateRequest, PersonsResponse};
pub use scenario::{
    ScenarioContradiction, ScenarioContradictionEvidence, ScenarioContradictionsResponse,
    ScenarioPage, ScenarioPageParams, ScenarioRebuttalFact, ScenarioRebuttalFactsResponse,
    ScenarioRelatedAllegation, ScenarioRelatedAllegationsResponse,
};
pub use scenario_crud::{
    ScenarioCreateRequest, ScenarioDefinition, ScenarioDto, ScenarioUpdateRequest, Wielder,
};
pub use scenario_facts::{
    AddFactRequest, CandidateDto, FactAction, FactActionRequest, GatherCandidatesResponse,
    ScenarioFactDto,
};
pub use schema::SchemaResponse;
pub use theme_scan::{
    ScanRequest, ScanRunHeader, ScanRunListResponse, ScanRunMergeRequest, ScanRunMergeResponse,
    ScanRunStatusResponse, ScanStartedResponse, ThemeScanRejected, ThemeScanSuggestion,
    ThemeScanSummary,
};
pub use trial_prep::{
    ScenarioStatus, ScenarioSummary, TrialPrepAlert, TrialPrepDashboard, TrialPrepMetrics,
};
