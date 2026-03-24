//! Document lifecycle status constants.
//!
//! ## Rust Learning — Module-Level Constants
//!
//! These are `&'static str` — string slices with a 'static lifetime,
//! meaning they live for the entire program. They're compiled into the
//! binary's read-only data section. Unlike `String` (heap-allocated),
//! these cost zero runtime allocation. We use &str constants instead
//! of an enum here because the values are stored as strings in Neo4j
//! and compared as strings — an enum would require conversion at every
//! database boundary.

pub const STATUS_UPLOADED: &str = "UPLOADED";
pub const STATUS_CLASSIFIED: &str = "CLASSIFIED";
pub const STATUS_EXTRACTED: &str = "EXTRACTED";
pub const STATUS_IN_REVIEW: &str = "IN_REVIEW";
pub const STATUS_APPROVED: &str = "APPROVED";
pub const STATUS_INGESTED: &str = "INGESTED";
pub const STATUS_INDEXED: &str = "INDEXED";
pub const STATUS_PUBLISHED: &str = "PUBLISHED";

/// All valid statuses, in lifecycle order.
pub const VALID_STATUSES: &[&str] = &[
    STATUS_UPLOADED,
    STATUS_CLASSIFIED,
    STATUS_EXTRACTED,
    STATUS_IN_REVIEW,
    STATUS_APPROVED,
    STATUS_INGESTED,
    STATUS_INDEXED,
    STATUS_PUBLISHED,
];
