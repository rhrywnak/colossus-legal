// =============================================================================
// backend/src/domain/date_precision.rs — how much of a document's date the
// source actually stated (task P4, B2 §3)
// =============================================================================
//
// A document's date is entered by a human at intake, and real documents are
// dated to different precisions: a filing stamp gives a day, a letter may give
// only "November 2009", an exhibit may give a year, and some documents state no
// date at all.
//
// Storing `2009-11-01` for "November 2009" and calling it a day-precision date
// would be a fabricated day that no later reader could tell from a real one —
// and fabricated dates are the class this whole track exists to keep dead
// (B2 §1: no invented dates, ever; nothing parses "041212" out of a title).
// So the DATE column always holds the first day of the stated period, and this
// vocabulary says how much of that period the source actually gave.
//
// ## Why a code-owned lookup, not a Postgres enum
//
// Following the `actor_role` precedent (D1), which is the sanctioned convention
// in this repo: a Rust enum plus a versioned list. A Postgres enum would make
// adding a precision a migration and would couple the vocabulary to one
// database; bare strings compared in match arms would let a typo'd `"moth"` fail
// silently. The migration carries a CHECK constraint listing the same four
// tokens as a backstop for anything that reaches the table another way.

use serde::{Deserialize, Serialize};

/// The version of the date-precision vocabulary THIS build defines.
///
/// Bumped whenever a precision is added or removed. Mirrors
/// `ACTOR_ROLE_LOOKUP_V`.
pub const DATE_PRECISION_LOOKUP_V: u32 = 1;

/// How much of a document's date the source stated.
///
/// ## Rust Learning: `#[serde(rename_all = "snake_case")]` on an enum
///
/// serde would otherwise render `Unknown` as `"Unknown"`. `rename_all` maps every
/// variant to its wire token in one line, so the JSON the frontend sends, the
/// text in the Postgres column and the TypeScript union all stay identical
/// without per-variant attributes. An unknown token fails to deserialize rather
/// than defaulting — the loud boundary Standing Rule 1 asks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DatePrecision {
    /// The source states a full date. `2009-11-05`.
    Day,
    /// The source states a month and year. Stored as the first of that month.
    Month,
    /// The source states only a year. Stored as 1 January of that year.
    Year,
    /// A human looked at the document and it has no usable date.
    ///
    /// Domain note: this is an ANSWER, and it is deliberately not the same as
    /// "nobody has been asked yet" — which is the absence of a precision
    /// altogether. The ruling is mandatory-with-override: intake must ask, may be
    /// told "unknown", and must never accept a silent blank. Collapsing those two
    /// states would make the ask meaningless.
    Unknown,
}

/// Every precision this build defines, in decreasing order of precision.
///
/// The order is the order a UI should offer them: the most common answer first,
/// the escape hatch last.
pub const ALL_DATE_PRECISIONS: &[DatePrecision] = &[
    DatePrecision::Day,
    DatePrecision::Month,
    DatePrecision::Year,
    DatePrecision::Unknown,
];

impl DatePrecision {
    /// The token stored in Postgres and sent over the wire.
    pub fn as_str(&self) -> &'static str {
        match self {
            DatePrecision::Day => "day",
            DatePrecision::Month => "month",
            DatePrecision::Year => "year",
            DatePrecision::Unknown => "unknown",
        }
    }

    /// Parse a stored or submitted token.
    ///
    /// Returns `None` for anything unrecognised rather than falling back to a
    /// default. A precision this build does not know is a real disagreement
    /// between the row and the code, and silently reading it as `Unknown` would
    /// turn a dated document into an undated one.
    pub fn from_token(token: &str) -> Option<Self> {
        ALL_DATE_PRECISIONS
            .iter()
            .copied()
            .find(|p| p.as_str() == token)
    }

    /// Whether this precision expects a date alongside it.
    ///
    /// The database says the same thing in a CHECK constraint; this is the half
    /// that lets the API refuse a bad pair with a message naming the problem,
    /// instead of surfacing a constraint violation.
    pub fn expects_a_date(&self) -> bool {
        !matches!(self, DatePrecision::Unknown)
    }

    /// A human-facing label, for the intake control and the document page.
    pub fn label(&self) -> &'static str {
        match self {
            DatePrecision::Day => "Exact date",
            DatePrecision::Month => "Month and year only",
            DatePrecision::Year => "Year only",
            DatePrecision::Unknown => "No date on the document",
        }
    }
}

/// Why a submitted date and precision could not be accepted.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum DatePrecisionError {
    #[error("unknown date precision '{token}'. Expected one of: day, month, year, unknown")]
    UnknownToken { token: String },

    #[error(
        "precision '{precision}' needs a date, but none was given. If the document \
         carries no usable date, send precision 'unknown' — a blank date is never \
         accepted silently"
    )]
    DateMissing { precision: &'static str },

    #[error(
        "precision 'unknown' means the document has no usable date, but a date \
         ({date}) was given as well. Send one or the other"
    )]
    DateUnexpected { date: String },
}

/// Validate a `(date, precision)` pair before it reaches the database.
///
/// ## Why this is a free function and not a constructor
///
/// The two values arrive from three different places — the upload form, the
/// document-page edit, and a future backfill — and each needs the SAME rule with
/// its OWN error surface. A free function taking the raw pair keeps the rule in
/// one place without forcing a type on any of the three.
///
/// `date` is the caller's already-parsed date, so this function has no opinion
/// about date FORMATS; that is the deserializer's job and it fails loudly on its
/// own.
pub fn validate(
    date: Option<&str>,
    precision_token: &str,
) -> Result<(Option<String>, DatePrecision), DatePrecisionError> {
    let precision = DatePrecision::from_token(precision_token).ok_or_else(|| {
        DatePrecisionError::UnknownToken {
            token: precision_token.to_string(),
        }
    })?;

    match (date, precision.expects_a_date()) {
        (Some(d), true) => Ok((Some(d.to_string()), precision)),
        (None, true) => Err(DatePrecisionError::DateMissing {
            precision: precision.as_str(),
        }),
        (Some(d), false) => Err(DatePrecisionError::DateUnexpected {
            date: d.to_string(),
        }),
        (None, false) => Ok((None, precision)),
    }
}

#[cfg(test)]
#[path = "date_precision_tests.rs"]
mod tests;
