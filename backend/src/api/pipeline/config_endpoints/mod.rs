//! Configuration discovery + admin CRUD endpoints for the pipeline.
//!
//! This module splits into one submodule per resource surface so each
//! file stays well under the 300-line module-size rule:
//!
//! - [`models`] — DB-backed CRUD for the `llm_models` table (Section 3.4.1)
//! - [`profiles`] — YAML file CRUD for processing profiles (Section 3.4.2)
//! - [`templates`] — `.md` file CRUD for prompt templates (Section 3.4.3)
//! - [`schemas`] — `.yaml` file CRUD for extraction schemas (Section 3.4.4)
//! - [`system_prompts`] — `.md` file CRUD for system prompts (Section 3.4.3)
//! - [`shared`] — cross-cutting DTOs, filename validator, profile-ref scanner
//!
//! The router in `api/pipeline/mod.rs` imports handlers as
//! `config_endpoints::<handler>`; the `pub use` re-exports below preserve
//! that spelling after the split. Design:
//! DOC_PROCESSING_CONFIG_DESIGN_v2.md Sections 3.4.1 through 3.4.4.

pub mod models;
pub mod profiles;
pub mod schemas;
pub mod shared;
pub mod system_prompts;
pub mod templates;

pub use models::{create_model, delete_model, list_models, toggle_model, update_model};
pub use profiles::{
    create_profile, deactivate_profile, get_profile, list_profiles, update_profile,
};
pub use schemas::{create_schema, delete_schema, get_schema, list_schemas, update_schema};
pub use system_prompts::{
    create_system_prompt, delete_system_prompt, get_system_prompt, list_system_prompts,
    update_system_prompt,
};
pub use templates::{
    create_template, delete_template, get_template, list_templates, update_template,
};
