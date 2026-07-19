//! Service layer modules for business logic.

pub mod audit_checks;
pub mod claim_validator;
pub mod claude_client;
pub mod embedding_pipeline;
pub mod embedding_service;
pub mod embedding_text;
pub mod graph_expander;
pub mod graph_expansion_cypher;
pub mod graph_expansion_minor;
pub mod graph_expansion_queries;
pub mod import_validator;
pub mod qdrant_service;
pub mod scan_run_enrich;
pub mod scenario_dashboard;
pub mod scenario_page;
pub mod scenario_subject;
pub mod theme_scan;
pub mod theme_scan_judge;
pub mod theme_scan_parse;
pub mod theme_scan_persist;
pub mod theme_scan_provider;
pub mod theme_scan_run;
pub mod vllm_model_gate;
