//! The practice read's judgment parameters (PRACTICE v0).
//!
//! Seven stored values that decide what the one-sentence read is TOLD, by WHICH
//! model, and what shape of reply this build will put in front of a witness.
//!
//! ## Why a nested block rather than seven more fields on `Settings`
//!
//! The reason that file gives for its eleven wording blocks, applied to numbers:
//! `Settings` is the parameters this system judges by, and a reader looking for a
//! confidence cutoff should not scroll past a witness surface's word caps to find
//! it. Seven flat fields would also have taken `domain::settings` past the
//! 300-line limit (Rule 17) — which is the mechanical half of the same argument.
//!
//! ## Why these are NOT wording
//!
//! Nobody reads them on a screen. `fine_token` is the closest call and it is
//! still not wording: the model writes it and the parser recognises it, and it
//! reaches Marie only as the first word of a sentence the model composed. It is
//! stored for a different reason — see its field note.

/// What the read is told, and what it is allowed to say back.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PracticeReadParams {
    /// The file, in the extraction-template directory, holding the read's system
    /// prompt.
    ///
    /// A FILE and not a row because the prompt is a page of instructions with the
    /// seven tactic counters in it — the same reason `theme_scan_prompt_file` is
    /// a file. The write path refuses a name that does not resolve, and boot
    /// refuses to start if the named file has since stopped resolving.
    pub prompt_file: String,

    /// Which `llm_models` row judges one typed answer.
    ///
    /// ## Domain note: why a row and not an env var
    ///
    /// The read costs pennies and its quality is the whole feature. Roman will
    /// want to try a cheaper model against Marie's real answers and change his
    /// mind the same evening — a Settings edit, not a redeploy.
    pub model: String,

    /// The read's output cap. Deliberate headroom, not the sentence's budget —
    /// see the migration's note on the 2026-08-09 truncation.
    pub max_tokens: u32,

    /// The most words a read may use when it names a tactic.
    ///
    /// ## Domain note: this REFUSES, it does not truncate
    ///
    /// A reply above the cap produces no read at all. Half a sentence about
    /// testimony can invert its meaning, and the screen has an honest way to say
    /// nothing.
    pub max_words: u32,

    /// The most words that may follow the OK word. "Fine." plus a speech is
    /// still a speech.
    pub max_words_after_fine: u32,

    /// The exact word the model must produce for "nothing wrong with that".
    ///
    /// COUPLED to [`Self::prompt_file`], which teaches the model to write it.
    /// Both are stored precisely so both can be edited together, in one place, by
    /// one person — an operator who changes one and not the other gets every read
    /// marked as a fault.
    pub fine_token: String,

    /// The seven TACTIC_DECK_v1 card names, in card order 1–7.
    ///
    /// ## Domain note: why the deck stores a NUMBER and this holds the words
    ///
    /// A tactic is an index into a taxonomy, and the taxonomy is Roman's — the
    /// same seven cards Chuck coaches from. Storing "false premise" in every deck
    /// row would mean re-seeding every deck to rename a card, and would put this
    /// case's vocabulary into a table another Colossus project would inherit.
    pub tactic_names: Vec<String>,

    /// The IANA zone this case's days are counted in — see
    /// [`KEY_PRACTICE_CASE_TIMEZONE`]. Carried on this snapshot because every
    /// practice read that asks "was this today?" already has it in hand.
    pub case_timezone: String,
}

// KEYS: the stable identifiers, named here and listed in
// `settings_store::REQUIRED_KEYS` — which is the ONE boot check, so a parameter
// missing from it is a parameter nothing verifies. Renaming one is a migration,
// and until it runs the boot loader refuses to start.
/// The highest TACTIC_DECK_v1 card number.
//
// STRUCTURAL: the size of a fixed VOCABULARY, not a tunable. The seven
// cards are a taxonomy of cross-examination moves, the column's own CHECK is
// `BETWEEN 1 AND 7`, and the settings row `practice_tactic_names` carries
// exactly seven names. An eighth card is a migration plus a code change plus
// seven new sentences — never a value somebody raises on the Settings page,
// which would immediately let a question wear a tag the vocabulary cannot name.
//
// Held HERE, beside the vocabulary it counts, so the three places that fence a
// card number (the deck file's validator, the editor's edit path and its add
// path) cannot drift apart the way they had when this was three literals.
pub const TACTIC_CARD_MAX: i16 = 7;

pub const KEY_PRACTICE_READ_PROMPT_FILE: &str = "practice_read_prompt_file";
pub const KEY_PRACTICE_READ_MODEL: &str = "practice_read_model";
pub const KEY_PRACTICE_READ_MAX_TOKENS: &str = "practice_read_max_tokens";
pub const KEY_PRACTICE_READ_MAX_WORDS: &str = "practice_read_max_words";
pub const KEY_PRACTICE_READ_MAX_WORDS_AFTER_FINE: &str = "practice_read_max_words_after_fine";
pub const KEY_PRACTICE_READ_FINE_TOKEN: &str = "practice_read_fine_token";
pub const KEY_PRACTICE_TACTIC_NAMES: &str = "practice_tactic_names";

/// Every key this block reads.
///
/// ## Why a list of its own, like a wording block's
///
/// The eleven `*_WORDING_KEYS` lists already work this way: a block names the
/// keys it reads, and the boot check consults every list. This is the first
/// NON-wording block to follow the pattern, and it follows it for the same two
/// reasons — a key belongs beside the struct that reads it, and
/// `settings_store::REQUIRED_KEYS` had reached the point where one more surface
/// took that module past the 300-line limit.
///
/// The drift this could invite (a list nothing consults) is closed the same way
/// it is for the wording blocks: `settings_boot` counts it, and
/// `settings_store_tests` both walks it and asserts the total.
/// The IANA zone this case's days are counted in.
///
/// ## Why this is not on [`PracticeReadParams`]
///
/// That struct is what the READ is told. This is what a DAY is — it decides when
/// `answered today` becomes `last: Wed 19 Aug` on a deck row and when the
/// unfinished line says "today", and no model ever sees it. One field is not
/// worth a struct of its own on `Settings`, so it hangs off the practice params
/// module beside the other practice-wide values and is read as a plain string.
///
/// ## Domain note: the value is CASE data
///
/// `America/Detroit`, because that is where the witness practises. Rule 2 keeps
/// it in the store; and the comparison itself happens in Postgres, which already
/// carries the tz database — so a zone name it does not know fails the read
/// loudly instead of falling back to UTC, which is the bug this exists to fix.
pub const KEY_PRACTICE_CASE_TIMEZONE: &str = "practice_case_timezone";

pub const PRACTICE_PARAM_KEYS: &[&str] = &[
    KEY_PRACTICE_CASE_TIMEZONE,
    KEY_PRACTICE_READ_PROMPT_FILE,
    KEY_PRACTICE_READ_MODEL,
    KEY_PRACTICE_READ_MAX_TOKENS,
    KEY_PRACTICE_READ_MAX_WORDS,
    KEY_PRACTICE_READ_MAX_WORDS_AFTER_FINE,
    KEY_PRACTICE_READ_FINE_TOKEN,
    KEY_PRACTICE_TACTIC_NAMES,
];

/// ## Why this is `#[cfg(test)]`
///
/// Same reason `Settings::for_test` is: the gate means these values cannot exist
/// in a release binary AT ALL. Without it, `"claude-opus-5"` — a model name, which
/// is exactly the kind of value the configuration law exists to keep out of code —
/// would be compiled into the shipped product, one `unwrap_or_else` away from
/// becoming a silent fallback nobody chose.
#[cfg(test)]
impl PracticeReadParams {
    /// The fixture, for TESTS ONLY. Pinned to the migration by
    /// `settings_store_tests::the_fixtures_carry_the_values_the_migration_actually_seeds`.
    pub fn for_test() -> Self {
        PracticeReadParams {
            prompt_file: "practice_read_prompt_v2.md".to_string(),
            model: "claude-opus-5".to_string(),
            max_tokens: 1024,
            max_words: 25,
            max_words_after_fine: 6,
            fine_token: "Fine.".to_string(),
            tactic_names: TEST_TACTIC_NAMES.split(',').map(str::to_string).collect(),
            case_timezone: "America/Detroit".to_string(),
        }
    }
}

/// The seeded vocabulary, as one string — the shape the store holds it in.
///
/// Split rather than written as seven literals so the fixture cannot disagree
/// with the migration about a separator, which is the one way a comma-separated
/// row goes wrong. Gated with its only caller: this case's tactic vocabulary has
/// no business in a release binary.
#[cfg(test)]
pub const TEST_TACTIC_NAMES: &str =
    "broad generalization,half-truth,character jab,false premise,compound,authority borrow,echo";
