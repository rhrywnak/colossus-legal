//! Every time the practice pages show, in the CASE's day.
//!
//! ## The defect this exists to end
//!
//! Every timestamp on these tables is UTC, and every practice surface rendered
//! it raw and in 24-hour: `09:57`, `17:36`. Two things wrong with that, and the
//! second is worse than the first.
//!
//! Marie practises in Michigan in the evening. At 5:36 pm she saw `21:36` — a
//! time four hours in her future, written in a format she does not use. It is
//! not a rounding error; it is the screen telling a witness something untrue
//! about when she did the work, on a page whose whole job is to be a faithful
//! record of it.
//!
//! The 08-19 hotfix fixed the COMPARISON half — "is this answer from today?" —
//! by doing it in Postgres in the case's zone. This is the DISPLAY half, and it
//! is the half she actually reads.
//!
//! ## One formatter, and why that matters more than it sounds
//!
//! Seven surfaces show a time: the unfinished-session line, a deck row's status,
//! the review page's attempts, Chuck's sheet, the changed-since box, notes, and
//! the last-session line. Before this module each formatted its own way from two
//! shared constants — which is exactly the shape where six get fixed and the
//! seventh keeps printing UTC for a year, because nothing connects them.
//!
//! So there is one module, three functions, and a test per surface.
//!
//! ## Rust Learning: `chrono_tz::Tz` and `with_timezone`
//!
//! `DateTime<Utc>` is an instant plus the knowledge that it is being *expressed*
//! in UTC. `with_timezone(&tz)` returns the SAME instant expressed in another
//! zone — no clock arithmetic on our side, and the daylight-saving rules come
//! from the IANA database `chrono-tz` compiles in. That is the whole reason the
//! zone is a name (`America/Detroit`) and not an offset: an offset would be
//! right for half the year, which is worse than being wrong all of it, because
//! nobody notices in November.

use chrono::{DateTime, Utc};
use chrono_tz::Tz;

// STRUCTURAL: the two formats below are not per-deployment values and are
// deliberately NOT settings rows. Two reasons, and the second decides it.
//
// They are the shape of a date and a clock on ONE witness surface, chosen to read
// like something a person says out loud rather than like a timestamp. Nothing
// about them varies between DEV and PROD, and this case has one locale. What DOES
// vary — the zone — is a settings row, and it is the argument to every function
// here.
//
// And a strftime string is the one kind of stored value the settings store cannot
// validate. A typo does not fail: `%a %-d %v` renders "Wed 19 %v" onto Chuck's
// printed sheet, silently, with every other check green. The store's whole
// promise is that a value it accepts is a value that works, and it could not keep
// that promise for these two.

/// `Wed 19 Aug` — the day, as these surfaces say it.
const DATE_FORMAT: &str = "%a %-d %b";

/// `5:36 pm` — 12-hour, lower-case meridiem, no leading zero.
///
/// `%-I` (not `%I`) drops the leading zero, so it reads `9:05 am` and not
/// `09:05 am`; `%P` is the lower-case meridiem, where `%p` would give `PM`.
/// Roman's ruling: `5:36 pm`.
const CLOCK_FORMAT: &str = "%-I:%M %P";

/// Zone names already warned about, so a page load does not become a log storm.
///
/// ## Why this guard exists
///
/// `zone` is called once per rendered timestamp. A review page with forty
/// questions and three attempts each calls it 240 times — so a single mistyped
/// settings row would bury every other line in the log under 240 copies of one
/// warning, which is the opposite of making a problem visible.
///
/// Keyed by the offending VALUE rather than a plain `Once`: a bad zone corrected
/// to a *different* bad zone is a new fact and deserves its own line. Per
/// process, so a restart says it again — which is right, because a restart is
/// when somebody is watching.
///
/// ## Rust Learning: `OnceLock` + `Mutex` for lazy shared state
///
/// `OnceLock` gives a `static` that is initialised exactly once on first use,
/// without the unsafe of a mutable static and without a macro. The `Mutex`
/// inside it is what makes the set safe to touch from many request tasks at
/// once. A poisoned lock is treated as "already warned" rather than panicking:
/// a formatter must not take a witness's page down over its own log de-duplication.
static WARNED_ZONES: std::sync::OnceLock<std::sync::Mutex<std::collections::HashSet<String>>> =
    std::sync::OnceLock::new();

/// True the FIRST time this process sees this bad zone name.
fn first_sighting(name: &str) -> bool {
    let Ok(mut seen) = WARNED_ZONES
        .get_or_init(|| std::sync::Mutex::new(std::collections::HashSet::new()))
        .lock()
    else {
        return false;
    };
    seen.insert(name.to_string())
}

/// The zone to render in, or UTC when the stored name is not one Postgres and
/// `chrono-tz` both know.
///
/// ## Why an unknown zone falls back LOUDLY, but only once
///
/// A zone name nobody recognises is a settings-row typo, and the honest
/// behaviour is to say so and keep rendering — a practice page that refused to
/// load because a timezone string was misspelt would be a worse failure than a
/// time in the wrong zone. So: log it once per distinct bad value, and fall back
/// to UTC, which is at least the value actually stored.
///
/// The same string is handed to Postgres for the "today" comparison, where an
/// unknown zone makes the READ fail loudly. Two different treatments of the same
/// bad value, deliberately: a failed comparison is a wrong answer and must stop;
/// a failed render is a cosmetic degrade and must not.
fn zone(name: &str) -> Tz {
    name.parse::<Tz>().unwrap_or_else(|_| {
        if first_sighting(name) {
            tracing::warn!(
                timezone = %name,
                "practice: the stored case timezone is not an IANA zone name — rendering in UTC. \
                 Fix `practice_case_timezone` in Settings. Logged once per value per process"
            );
        }
        Tz::UTC
    })
}

/// `5:36 pm` — the clock alone.
pub fn local_clock(at: DateTime<Utc>, timezone: &str) -> String {
    at.with_timezone(&zone(timezone))
        .format(CLOCK_FORMAT)
        .to_string()
}

/// `Wed 19 Aug` — the day alone.
///
/// Used where the surface already says which day some other way, or where only
/// the date matters (a note's age, the last session's date).
pub fn local_date(at: DateTime<Utc>, timezone: &str) -> String {
    at.with_timezone(&zone(timezone))
        .format(DATE_FORMAT)
        .to_string()
}

/// `Wed 19 Aug · 5:36 pm` — the full stamp.
///
/// The separator is the middle dot this product uses everywhere between two
/// facts of equal weight. It is a literal here rather than a settings row
/// because it is punctuation, not wording: a case that wanted a different
/// separator would want it in forty other places first.
pub fn local_stamp(at: DateTime<Utc>, timezone: &str) -> String {
    let local = at.with_timezone(&zone(timezone));
    // STRUCTURAL: the separator is punctuation, not wording. This product uses
    // the middle dot between two facts of equal weight in forty places; a case
    // that wanted a different one would want it in all of them, not here.
    format!(
        "{} · {}",
        local.format(DATE_FORMAT),
        local.format(CLOCK_FORMAT)
    )
}

#[cfg(test)]
#[path = "practice_clock_tests.rs"]
mod tests;
