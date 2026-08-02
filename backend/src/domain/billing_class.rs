// =============================================================================
// backend/src/domain/billing_class.rs — who pays for a model call
// =============================================================================
//
// Task 1.7B. The scan control lists LOCAL models first, defaults to one, and
// labels the rest "(API — billed)", so nobody starts a 148-candidate scan on a
// metered endpoint by accident.
//
// ## Why this is a stored fact and not an inference
//
// Three proxies were available and all three were rejected — see the migration
// `20260802134438_add_billing_class_to_llm_models` for the full argument. The
// short form: `provider` and `api_endpoint` describe which client speaks to the
// endpoint, not who pays; and a NULL `cost_per_input_token` means UNKNOWN, never
// FREE, so inferring "free" from it would tell a human a scan costs nothing
// minutes before it bills them.
//
// ## Why the vocabulary lives here and not in a CHECK constraint
//
// The same argument `ValueKind` carries in the settings store: a CHECK can
// refuse an unknown token, but it cannot tell you the PARSER is missing. An
// unknown token must be refused by the reader, by name, so a human is told which
// value this build cannot understand rather than watching a model quietly vanish
// from a list.

use std::fmt;

/// Who pays for a call to a model.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BillingClass {
    /// Self-hosted. No metered cost per call.
    Local,
    /// A third-party API. Metered — every call is billed.
    Billed,
}

/// Why a stored billing class could not be read.
#[derive(Debug, thiserror::Error, PartialEq)]
#[error(
    "'{token}' is not a billing class this build understands (local, billed). \
     A model whose class cannot be read is not offered for a scan, because the \
     page cannot honestly say whether running it costs money"
)]
pub struct UnknownBillingClass {
    pub token: String,
}

impl BillingClass {
    /// The stable token stored in `llm_models.billing_class`.
    pub fn code(self) -> &'static str {
        match self {
            BillingClass::Local => "local",
            BillingClass::Billed => "billed",
        }
    }

    /// What the model select shows beside a model's name, or `None` when there
    /// is nothing to warn about.
    ///
    /// ## Why only one of the two is labelled
    ///
    /// A label on every row is a label nobody reads. The page's default is local,
    /// so "local" is the unremarkable case and needs no decoration; the one that
    /// changes what a click COSTS is the one that gets words. Composed here rather
    /// than in the browser for the reason every other label on this surface is:
    /// the vocabulary is a statement about the system, and the browser does not
    /// get to phrase it.
    pub fn suffix(self) -> Option<&'static str> {
        match self {
            BillingClass::Local => None,
            BillingClass::Billed => Some("(API — billed)"),
        }
    }

    /// Sort key — local first (v2 §2c scan control, Roman's 2026-08-02 ruling).
    ///
    /// ## Rust Learning: an explicit key instead of `#[derive(Ord)]`
    ///
    /// Deriving `Ord` on the enum would order by DECLARATION order, which means a
    /// future reader reordering the variants for readability would silently
    /// reorder the model dropdown. A function makes the ordering a decision with
    /// a name and a test rather than a side effect of how the type is written.
    pub fn sort_key(self) -> u8 {
        match self {
            BillingClass::Local => 0,
            BillingClass::Billed => 1,
        }
    }
}

impl fmt::Display for BillingClass {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code())
    }
}

impl TryFrom<&str> for BillingClass {
    type Error = UnknownBillingClass;

    fn try_from(token: &str) -> Result<Self, Self::Error> {
        match token {
            "local" => Ok(BillingClass::Local),
            "billed" => Ok(BillingClass::Billed),
            other => Err(UnknownBillingClass {
                token: other.to_string(),
            }),
        }
    }
}

#[cfg(test)]
#[path = "billing_class_tests.rs"]
mod tests;
