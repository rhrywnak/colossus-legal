//! Authentication integration — re-exports from colossus-auth.
//!
//! This module provides a single import point for auth types.
//! All implementation lives in the shared colossus-auth crate.

pub use colossus_auth::{
    AuthError, AuthMode, AuthUser, MeResponse, Permissions,
    me_handler, require_admin, require_ai, require_edit,
    GROUP_ADMIN, GROUP_AI_USER, GROUP_LEGAL_EDITOR, GROUP_LEGAL_VIEWER,
};
