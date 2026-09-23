//! The practice read's judgment parameters (PRACTICE v0).
//!
//! Twelve stored values that decide what the read is TOLD, by WHICH model, and
//! what shape of reply this build will put in front of a witness.
//!
//! Four of them arrived with T1 (2026-08-20), when the read stopped being one
//! sentence and became three parts with a ceiling each.
//!
//! ## Why a nested block rather than seven more fields on `Settings`
//!
//! The reason that file gives for its eleven wording blocks, applied to numbers:
//! `Settings` is the parameters this system judges by, and a reader looking for a
//! confidence cutoff should not scroll past a witness surface's word caps to find
//! it. Twelve flat fields would also have taken `domain::settings` past the
//! 300-line limit (Rule 17) — which is the mechanical half of the same argument,
//! and the reason T1's four ceilings cost that file nothing at all.
//!
//! ## Why these are NOT wording
//!
//! Nobody reads them on a screen. `fine_token` is the closest call and it is
//! still not wording: the model writes it and the parser recognises it, and it
//! reaches Marie only as the first word of a sentence the model composed. It is
//! stored for a different reason — see its field note.

use crate::domain::llm_effort::Effort;

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

    /// The whole-reply word cap of the ONE-SENTENCE read (prompt v2).
    ///
    /// ## Domain note: nothing on the v3 path reads this, and it stays anyway
    ///
    /// v3 returns three parts, each with its own ceiling
    /// ([`Self::max_words_call`], [`Self::max_words_why`],
    /// [`Self::max_words_pointer`]), so there is no whole-reply cap left for this
    /// number to be. It is kept because [`Self::prompt_file`] can be pointed back
    /// at `practice_read_prompt_v2.md` — that is the T1 rollback, one settings
    /// edit and no file work — and the v2 path reads this row. **A row nothing
    /// reads today is the price of a working rollback** (Roman, 2026-08-20).
    ///
    /// Its old note, still true of the v2 path: a reply above the cap produces no
    /// read at all, because half a sentence about testimony can invert its
    /// meaning. v3 inverts that trade — see [`Self::max_words_call`].
    pub max_words: u32,

    /// The most words the read's CALL may use — the line naming what happened.
    ///
    /// ## Domain note: a CEILING, not a target, and it never discards
    ///
    /// This is the half of T1 that is a correctness fix rather than a feature. A
    /// 26-word reply under the old rule was refused and Marie saw nothing at all
    /// — one word over a cap, and a witness got no coaching. Over this ceiling
    /// the read is re-requested ONCE; a second overrun is stored and shown as
    /// returned, with the part and the count logged. Never truncated, because
    /// half a sentence about testimony can invert its meaning; never discarded,
    /// because a formatting slip is not her fault.
    pub max_words_call: u32,

    /// The most words the read's WHY may use — the reasoning, citing the record.
    /// May legitimately be empty when there is nothing to say beyond the call.
    pub max_words_why: u32,

    /// The most words ONE pointer may use.
    ///
    /// Domain note: a pointer names the move and never supplies the words. The
    /// cap is part of what keeps it from becoming a sentence Marie could speak
    /// verbatim in the first person — the anti-script rule — though the rule
    /// itself is asked by the prompt and cannot be pinned by a number.
    pub max_words_pointer: u32,

    /// The most pointers one read may carry.
    ///
    /// Domain note: the design DEFAULTS to one. Coaching that names one thing is
    /// acted on; coaching that names three is skimmed, and three ordered pointers
    /// can be the skeleton of her answer even when no single one supplies words.
    /// This is the hard ceiling, not the expectation.
    pub max_pointers: u32,

    /// The most words that may follow the OK word. "Fine." plus a speech is
    /// still a speech.
    ///
    /// Still read on the v3 path: the CALL is where the OK word appears, so a
    /// call that opens with it is capped by this rather than by
    /// [`Self::max_words_call`].
    pub max_words_after_fine: u32,

    /// The exact word the model must produce for "nothing wrong with that".
    ///
    /// COUPLED to [`Self::prompt_file`], which teaches the model to write it.
    /// Both are stored precisely so both can be edited together, in one place, by
    /// one person — an operator who changes one and not the other gets every read
    /// marked as a fault.
    ///
    /// ## Domain note: why v3 kept it rather than adding a `"fine": true` field
    ///
    /// The three-part reply could have carried a boolean saying whether the
    /// answer was fine. It does not, because a boolean can DISAGREE with the
    /// words beside it — a reply reading `{"call": "You let the braid stand",
    /// "fine": true}` has no correct interpretation, and the rail colour it
    /// produces is a coin toss. Deriving the verdict from the call's own first
    /// word means the sentence Marie reads and the colour beside it cannot come
    /// apart.
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

    /// Every login whose Done reviewing marks the review queue is counted
    /// against, and the only logins offered that button — see
    /// [`KEY_PRACTICE_REVIEWER_USERNAMES`].
    ///
    /// ## Domain note: a BENCH, not a person (ruled 2026-09-19)
    ///
    /// This was one login until CC_TASK_REVIEW_PAGE_v1. Chuck reviews Marie's
    /// answers; before trial, so does Roman, and there was no way to say so
    /// without handing the whole queue from one man to the other.
    ///
    /// ## ⚑ DISPLAY, not permission (ruled 2026-09-22)
    ///
    /// It does NOT decide who may press Done reviewing — that is
    /// `services::review_permission::may_review`, which also admits anyone in
    /// the admin group. Roman took himself off this list to keep his name off
    /// the war room and lost the button with it, which is the defect
    /// CC_TASK_REVIEW_PERMISSION_v1 fixed.
    ///
    /// What it still decides: whose names the dashboard and the deck bar print,
    /// and whose answers and notes do not count as work waiting for review
    /// (`review_cursor::awaiting_review`'s exclusion legs — SQL cannot read
    /// Authentik groups, so an unlisted administrator's own work waits until a
    /// Done is pressed).
    pub reviewer_usernames: Vec<String>,
    /// The reviewers' names as screens print them, in the SAME ORDER as
    /// [`Self::reviewer_usernames`] — see [`KEY_PRACTICE_REVIEWER_DISPLAY_NAMES`].
    ///
    /// The two lists are index-aligned and the boot check refuses a snapshot
    /// where they differ in length: a name list one short does not fail, it
    /// prints the wrong attorney's name beside a queue, which is the one
    /// failure here that looks like working software.
    pub reviewer_display_names: Vec<String>,

    /// The login of the witness the War Room's "new or changed" count is
    /// addressed to — see [`KEY_PRACTICE_WITNESS_USERNAME`].
    ///
    /// ## Domain note: the count's OWNER is the filter, not the viewer
    ///
    /// Marie's own note to Chuck badged Marie's own tile. The obvious remedy —
    /// "do not count what the READER wrote" — is unavailable twice over: this
    /// build has no is-the-reader-the-witness concept (CC's 2026-09-16 STOP),
    /// and the count is one GLOBAL number by ruling (2026-09-17), so a
    /// per-viewer filter would give three people three different truths about
    /// one deck — exactly the state v2.1.10 was written to end.
    ///
    /// So the filter is the number's OWNER. One login, one row, one meaning: a
    /// note is a message to somebody else, and a message to herself is not work
    /// waiting on her. Question EDITS are untouched and still count whoever made
    /// them, including hers.
    pub witness_username: String,

    /// How many UNREAD items a deck must hold before the "For you" page shows
    /// it as ONE row instead of one row per item — see
    /// [`KEY_PRACTICE_FOR_YOU_DECK_THRESHOLD`].
    ///
    /// ## Domain note: a threshold about ATTENTION, not about size
    ///
    /// Seventeen rows from one deck is not seventeen things to decide; it is
    /// one deck to sit down with. Below the threshold the items are worth
    /// naming individually, because each one is a separate errand.
    pub for_you_deck_threshold: u32,

    // ── Discuss with AI (CC_TASK_QUESTION_CHAT_v1) ────────────────────────────
    /// The model the dock starts on — [`KEY_PRACTICE_DISCUSS_DEFAULT_MODEL`].
    pub discuss_default_model: String,
    /// The most model replies one question's thread may hold — the spend guard.
    pub discuss_max_turns: u32,
    /// The dock's system prompt file, in the template directory.
    pub discuss_prompt_file: String,
    /// The output cap of one discussion reply.
    pub discuss_max_tokens: u32,

    // ── The thinking dials (CC_TASK_READ_V4_BUDGET_FIX_v1) ────────────────────
    /// How much the model may think before one answer read — `None` sends no
    /// `effort` key at all. See [`KEY_PRACTICE_READ_EFFORT`].
    pub effort: Option<Effort>,
    /// The same dial for the discussion dock.
    pub discuss_effort: Option<Effort>,
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
pub const KEY_PRACTICE_READ_MAX_WORDS_CALL: &str = "practice_read_max_words_call";
pub const KEY_PRACTICE_READ_MAX_WORDS_WHY: &str = "practice_read_max_words_why";
pub const KEY_PRACTICE_READ_MAX_WORDS_POINTER: &str = "practice_read_max_words_pointer";
pub const KEY_PRACTICE_READ_MAX_POINTERS: &str = "practice_read_max_pointers";
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

/// The Authentik usernames of everyone who reviews Marie's answers,
/// comma-separated (CC_TASK_REVIEW_PAGE_v1; one login until then).
///
/// ## Domain note: ONE queue, owned by a BENCH
///
/// "Answers requiring review" is still a single number every viewer sees. What
/// changed on 2026-09-19 is who owns it: any login on this list may press Done
/// reviewing, the press moves the one shared mark, and the count clears for
/// everyone. Nothing any of them writes counts as work waiting for them.
/// Case data (the attorneys), so a stored row — adding a reviewer is a Settings
/// edit, never a build.
pub const KEY_PRACTICE_REVIEWER_USERNAMES: &str = "practice_reviewer_usernames";

/// The reviewers' names as screens print them, comma-separated and in the same
/// order as [`KEY_PRACTICE_REVIEWER_USERNAMES`] — the owner chip and
/// `{reviewer}` in the review pill and bar. A second row rather than template
/// words, so adding an attorney is two edits and not a hunt through sentences.
pub const KEY_PRACTICE_REVIEWER_DISPLAY_NAMES: &str = "practice_reviewer_display_names";

/// The witness's own login — see [`PracticeReadParams::witness_username`].
///
/// Case data (who the witness is), so a stored row: a login compiled in would
/// make "which witness is this deck for?" a rebuild, and this system is already
/// two attorneys and one witness away from needing a second one.
pub const KEY_PRACTICE_WITNESS_USERNAME: &str = "practice_witness_username";

/// How much the model may THINK before writing one answer read.
///
/// ## Domain note: the read is a verdict, not a deliberation
///
/// Thinking tokens count against `practice_read_max_tokens`. At the API's default
/// (`high`, which is what sending no key means) a thinking block truncated the
/// first v4 read on 2026-09-17 — see the migration's header. The row's vocabulary
/// is the five levels plus `absent`, which restores "send no key" without a
/// deploy; `domain::llm_effort::parse_stored_effort` is the ONE reader.
pub const KEY_PRACTICE_READ_EFFORT: &str = "practice_read_effort";

/// The same dial for the Discuss with AI dock — same exposure, same remedy.
pub const KEY_PRACTICE_DISCUSS_EFFORT: &str = "practice_discuss_effort";

/// The model the "Discuss with AI" dock starts on (CC_TASK_QUESTION_CHAT_v1). A
/// setting, never a literal: the button says "Discuss with AI" and the chip prints
/// whatever this names (ruled amendments 1 and 2).
pub const KEY_PRACTICE_DISCUSS_DEFAULT_MODEL: &str = "practice_discuss_default_model";
/// The per-question cap on MODEL replies, across everyone — the runaway-spend
/// guard, the dock's analogue of the read's `MAX_ATTEMPTS` (GO ruling 5: 40).
pub const KEY_PRACTICE_DISCUSS_MAX_TURNS: &str = "practice_discuss_max_turns";
/// The dock's system prompt, beside the read's in the template directory.
pub const KEY_PRACTICE_DISCUSS_PROMPT_FILE: &str = "practice_discuss_prompt_file";
/// The output cap of one discussion reply.
pub const KEY_PRACTICE_DISCUSS_MAX_TOKENS: &str = "practice_discuss_max_tokens";

/// The deck-grouping threshold — see [`PracticeReadParams::for_you_deck_threshold`].
///
/// A stored row and not a constant, because where the line falls is a judgement
/// about one person's day: Roman moves it, restarts, and reads the page again.
/// Its `min_value` is 1 — at 0 every deck holding anything would collapse into a
/// single row and the page could never name an individual item at all.
pub const KEY_PRACTICE_FOR_YOU_DECK_THRESHOLD: &str = "practice_for_you_deck_threshold";

pub const PRACTICE_PARAM_KEYS: &[&str] = &[
    KEY_PRACTICE_CASE_TIMEZONE,
    KEY_PRACTICE_REVIEWER_USERNAMES,
    KEY_PRACTICE_REVIEWER_DISPLAY_NAMES,
    KEY_PRACTICE_WITNESS_USERNAME,
    KEY_PRACTICE_DISCUSS_DEFAULT_MODEL,
    KEY_PRACTICE_DISCUSS_MAX_TURNS,
    KEY_PRACTICE_DISCUSS_PROMPT_FILE,
    KEY_PRACTICE_DISCUSS_MAX_TOKENS,
    KEY_PRACTICE_FOR_YOU_DECK_THRESHOLD,
    KEY_PRACTICE_READ_EFFORT,
    KEY_PRACTICE_DISCUSS_EFFORT,
    KEY_PRACTICE_READ_PROMPT_FILE,
    KEY_PRACTICE_READ_MODEL,
    KEY_PRACTICE_READ_MAX_TOKENS,
    KEY_PRACTICE_READ_MAX_WORDS,
    KEY_PRACTICE_READ_MAX_WORDS_AFTER_FINE,
    KEY_PRACTICE_READ_MAX_WORDS_CALL,
    KEY_PRACTICE_READ_MAX_WORDS_WHY,
    KEY_PRACTICE_READ_MAX_WORDS_POINTER,
    KEY_PRACTICE_READ_MAX_POINTERS,
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
            prompt_file: "practice_read_prompt_v5.md".to_string(),
            model: "claude-opus-5".to_string(),
            max_tokens: 4096,
            max_words: 25,
            max_words_after_fine: 6,
            max_words_call: 12,
            max_words_why: 55,
            max_words_pointer: 20,
            max_pointers: 3,
            fine_token: "Fine.".to_string(),
            tactic_names: TEST_TACTIC_NAMES.split(',').map(str::to_string).collect(),
            case_timezone: "America/Detroit".to_string(),
            // ONE name each, because that is what the migration produces on a
            // store seeded by `simple_counts`: the bench is seeded FROM the
            // singular rows it replaces. Tests that need two reviewers build
            // the list themselves; this fixture is pinned to the migration.
            reviewer_usernames: vec!["cpenzien".to_string()],
            reviewer_display_names: vec!["Chuck".to_string()],
            // The login the migration seeds, and the same one `war_room_status`'s
            // live fixtures answer as — so a proof that her own note stops
            // badging her is reading the value this build actually ships.
            witness_username: "docmarie".to_string(),
            discuss_default_model: "claude-opus-5".to_string(),
            discuss_max_turns: 40,
            discuss_prompt_file: "practice_discuss_prompt_v1.md".to_string(),
            discuss_max_tokens: 4096,
            // The value the migration seeds. Five is the ruled default; a
            // test that needs the other side of the line sets its own.
            for_you_deck_threshold: 5,
            effort: Some(Effort::Low),
            discuss_effort: Some(Effort::Low),
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
