pub mod allegation;
pub mod analysis;
pub mod card_grammar_wording;
pub mod case_dto;
pub mod case_header;
pub mod case_health;
pub mod case_summary;
pub mod causes_of_action;
pub mod claim;
pub mod contradiction;
pub mod decision;
pub mod decomposition;
pub mod document;
pub mod evidence;
pub mod evidence_chain;
pub mod evidence_links;
pub mod graph;
pub mod harm;
pub mod hearing;
pub mod matrix_wording;
pub mod model_params_wording;
pub mod motion_claim;
pub mod person;
pub mod person_detail;
pub mod practice;
pub mod practice_wording;
pub mod proof_matrix;
pub mod proof_review;
pub mod query;
pub mod rehearsal;
pub mod scenario;
pub mod scenario_accusation;
pub mod scenario_augmentation;
pub mod scenario_authoring_wording;
pub mod scenario_card;
pub mod scenario_crud;
pub mod scenario_curation;
pub mod scenario_facts;
pub mod scenario_orphans;
pub mod schema;
pub mod settings;
pub mod theme_scan;
pub mod trial_prep;
pub mod war_room_wording;

// Re-export DTOs / request types from submodules
pub use allegation::{AllegationDto, AllegationSummary, AllegationsResponse};
pub use analysis::{AllegationStrength, AnalysisResponse, GapAnalysis};
pub use case_dto::{CaseInfo, CaseResponse, CaseStats, LegalCountSummary, PartiesGroup, PartyDto};
pub use claim::{ClaimCreateRequest, ClaimDto, ClaimUpdateRequest};
pub use contradiction::{ContradictionDto, ContradictionEvidence, ContradictionsResponse};
pub use decision::{DecisionCreateRequest, DecisionUpdateRequest};
pub use decomposition::{AllegationDetailResponse, RebuttalsResponse};
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
pub use scenario_authoring_wording::{
    create_wording, identity_wording, ScenarioCreateWordingDto, ScenarioIdentityWordingDto,
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
    ScanRequest, ScanRunHeader, ScanRunListResponse, ScanRunStatusResponse, ScanStartedResponse,
    ThemeScanRejected, ThemeScanSuggestion, ThemeScanSummary,
};
pub use trial_prep::{
    ScenarioStatus, ScenarioSummary, TrialPrepAlert, TrialPrepDashboard, TrialPrepMetrics,
};
