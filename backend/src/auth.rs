//! Authentication integration — re-exports from colossus-auth.
//!
//! This module provides a single import point for auth types.
//! All implementation lives in the shared colossus-auth crate.

pub use colossus_auth::{
    me_handler, require_admin, require_ai, require_edit, AuthError, AuthMode, AuthUser, MeResponse,
    Permissions, GROUP_ADMIN, GROUP_AI_USER, GROUP_LEGAL_EDITOR, GROUP_LEGAL_VIEWER,
};
