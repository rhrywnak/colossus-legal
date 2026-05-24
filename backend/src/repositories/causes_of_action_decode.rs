//! Decoding of the canonical-Element-loader JSON properties stored on
//! `LegalCount` (`controlling_authorities_json`, `doctrinal_requirements_json`).
//!
//! Split from [`super::causes_of_action_builder`] to keep that module within
//! the 300-line limit. The rule that matters here (Standing Rule 1/§5): an
//! *absent* property is fine (empty / null), but a property that is *present
//! and malformed* is signal — the loader guarantees well-formed JSON, so a
//! decode failure means corruption and must surface (→ 500), never be silently
//! replaced by an empty list.

use crate::dto::causes_of_action::{Authority, DoctrinalRequirement};

/// Raised when a JSON-encoded `LegalCount` property is present but malformed.
#[derive(Debug, thiserror::Error)]
pub(crate) enum CausesShapeError {
    #[error("Count {count_number}: failed to decode controlling_authorities_json: {source}")]
    DecodeAuthorities {
        count_number: i64,
        #[source]
        source: serde_json::Error,
    },
    #[error("Count {count_number}: failed to decode doctrinal_requirements_json: {source}")]
    DecodeDoctrinal {
        count_number: i64,
        #[source]
        source: serde_json::Error,
    },
}

/// Decode `controlling_authorities_json`. Absent ⇒ empty list; present ⇒ decode
/// or error (never silently empty on malformed input).
pub(crate) fn decode_authorities(
    count_number: i64,
    json: &Option<String>,
) -> Result<Vec<Authority>, CausesShapeError> {
    match json {
        None => Ok(Vec::new()),
        Some(s) => serde_json::from_str(s).map_err(|source| CausesShapeError::DecodeAuthorities {
            count_number,
            source,
        }),
    }
}

/// Decode `doctrinal_requirements_json`. Absent ⇒ `None` (JSON null); present ⇒
/// decode or error.
pub(crate) fn decode_doctrinal(
    count_number: i64,
    json: &Option<String>,
) -> Result<Option<Vec<DoctrinalRequirement>>, CausesShapeError> {
    match json {
        None => Ok(None),
        Some(s) => serde_json::from_str::<Vec<DoctrinalRequirement>>(s)
            .map(Some)
            .map_err(|source| CausesShapeError::DecodeDoctrinal {
                count_number,
                source,
            }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn malformed_controlling_authorities_json_is_a_decode_error_not_silent_empty() {
        let err = decode_authorities(1, &Some("{ this is not valid json ]".into())).unwrap_err();
        match err {
            CausesShapeError::DecodeAuthorities { count_number, .. } => assert_eq!(count_number, 1),
            other => panic!("expected DecodeAuthorities, got {other:?}"),
        }
    }

    #[test]
    fn malformed_doctrinal_requirements_json_is_a_decode_error() {
        let err = decode_doctrinal(4, &Some("{ not an array ]".into())).unwrap_err();
        match err {
            CausesShapeError::DecodeDoctrinal { count_number, .. } => assert_eq!(count_number, 4),
            other => panic!("expected DecodeDoctrinal, got {other:?}"),
        }
    }
}
