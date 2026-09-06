//! The three input shapes `load_fact_cards` reads, and what it makes of them.
//!
//! Everything here is pure: parse, validate, count. The database is `write.rs`'s
//! problem, and keeping the two apart is what lets the dry run be a real dry run
//! — it does the whole of THIS file and none of that one.
//!
//! ## Why three shapes and not one
//!
//! They were produced by three different jobs against three different questions,
//! and the instruction names all three files. Job B drafted the five sentences;
//! Job D ranked a pool and picked ten from it; Job D also wrote talking points.
//! A single struct with everything optional would parse all three and tell us
//! nothing about which one we were holding.
//!
//! ## Why every struct here is `deny_unknown_fields`
//!
//! These files are written by ANOTHER job, and this loader writes what it reads
//! into a human-authored table. A key nobody declared is a sentence somebody
//! wrote that nothing is storing — the exact silent loss Standing Rule 1 is
//! about — so an undeclared key refuses the file by name and line number rather
//! than passing through. The cost is that a new reporting field upstream stops
//! the loader until it is declared here; that is the intended direction, because
//! a dry run refusing loudly is cheap and a dropped answer is not.
//!
//! Several declared fields are therefore parsed and NOT stored. Each says so and
//! says why, and `--candidates` reports the unstored pick reasons in its counts.

use anyhow::{bail, Context, Result};
use serde::Deserialize;

use colossus_legal_backend::domain::fact_card::{CardStance, SUPPORTS_CAP};

/// One accusation a drafted card names.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SupportEntry {
    /// Job B writes the SHORT form — the graph id's hash suffix, `"45984d77"`.
    /// Resolved to the full node id before anything is stored.
    pub allegation_id: String,
    pub stance: CardStance,
}

/// One card, as Job B wrote it (`B_S*.jsonl`, `D_B_S-*.jsonl`).
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DraftedCard {
    pub title: String,
    /// The talking point this card backs, by 1-based position. `null` on most.
    #[serde(default)]
    pub backs: Option<i32>,
    #[serde(default)]
    pub supports: Vec<SupportEntry>,
    #[serde(default)]
    pub watch_out: Option<String>,
    /// Job B's own field name. It is a DRAFT and says so in its value too —
    /// most begin with the literal "DRAFT:".
    #[serde(default)]
    pub answer_draft: Option<String>,
    /// The Evidence node this card is about — a full graph id.
    pub card_id: String,
    /// The card's human handle in the file, `"C98"`.
    ///
    /// Parsed so a malformed line is still refused by the reader, and DELIBERATELY
    /// not stored: the C-code is minted per scenario by
    /// `scenario_candidate_ordinals` and a second copy on the card would be a
    /// second answer to "what is this card called".
    #[serde(default)]
    #[allow(
        dead_code,
        reason = "parsed to validate the line; the C-code is minted per scenario"
    )]
    pub c_code: Option<String>,
    /// Job B's own report that it had to drop an accusation past the cap.
    ///
    /// Parsed so the shape is checked, and not acted on: the file is the record
    /// of what the model decided, and `checked_supports` refuses a file that
    /// exceeds the cap anyway.
    #[serde(default)]
    #[allow(
        dead_code,
        reason = "parsed to validate the line; the cap is enforced by checked_supports"
    )]
    pub over_supports_cap: bool,
    /// The accusations Job B named and then could not match to the list it was
    /// given. Declared and not acted on for the same reason as the cap report:
    /// the file is the record of what the drafter decided, and `plan_cards`
    /// re-resolves every accusation against the LIVE table anyway — reporting
    /// what it could not match itself.
    #[serde(default)]
    #[allow(dead_code, reason = "plan_cards re-resolves against the live table")]
    pub dropped_not_in_list: Vec<serde_json::Value>,
    /// Job B's slots for the statement's date and speaker. Both are null in every
    /// file written so far, and both are read from the GRAPH when the card
    /// renders — a card must not be able to name a speaker the record does not.
    #[serde(default)]
    #[allow(
        dead_code,
        reason = "the source line is read from the graph, never the file"
    )]
    pub date: Option<serde_json::Value>,
    #[serde(default)]
    #[allow(
        dead_code,
        reason = "the source line is read from the graph, never the file"
    )]
    pub speaker: Option<serde_json::Value>,
}

impl DraftedCard {
    /// The card's title, refused if it runs past the drafting limit.
    ///
    /// ## Domain note: fourteen words, enforced on the MACHINE only
    ///
    /// §1 caps a title at fourteen words. The database does NOT — a human editing
    /// a title to fifteen is making an editorial choice on their own surface, and
    /// a refusal there would be the wrong place to argue with them. This is where
    /// the cap belongs: on the drafts, before they are stored.
    ///
    /// # Errors
    /// Returns the title and its word count when it is too long.
    pub fn checked_title(&self, limit: usize) -> Result<&str> {
        let words = word_count(&self.title);
        if words > limit {
            bail!(
                "card {} has a {words}-word title and the limit is {limit}: {:?}",
                self.card_id,
                self.title
            );
        }
        Ok(&self.title)
    }

    /// The accusations this card names, refused if it names too many.
    ///
    /// Job B was given the same cap and reports `over_supports_cap` when it had
    /// to drop one, so a file that exceeds it here is a file produced against a
    /// different rule — which is worth stopping for, not truncating past.
    ///
    /// # Errors
    /// Returns the card id and the count when the cap is exceeded.
    pub fn checked_supports(&self) -> Result<&[SupportEntry]> {
        if self.supports.len() > SUPPORTS_CAP {
            bail!(
                "card {} names {} accusations and the cap is {SUPPORTS_CAP}",
                self.card_id,
                self.supports.len()
            );
        }
        Ok(&self.supports)
    }
}

/// How many WORDS a title runs to.
///
/// ## Domain note: a standalone em dash is punctuation, not a word
///
/// Job B writes titles like "It came back to the estate — never to my father",
/// and a naive `split_whitespace` counts the dash as the fifteenth word of a
/// fourteen-word sentence. That would refuse a real file over a counting
/// artifact — measured: it refused `B_S11.jsonl` on the first run.
///
/// So a token counts only when it carries at least one alphanumeric character.
/// Hyphenated words and possessives still count once; a lone dash, bullet or
/// ellipsis counts as nothing, which is what a reader would say if asked.
fn word_count(title: &str) -> usize {
    title
        .split_whitespace()
        .filter(|token| token.chars().any(char::is_alphanumeric))
        .count()
}

/// One scenario's talking points (`D_talking_points.jsonl`, and the
/// `talking_points` block inside a candidates file).
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TalkingPointsFile {
    pub scenario_code: String,
    pub scenario_id: uuid::Uuid,
    pub talking_points: Vec<TalkingPoint>,
    /// The run's own report of itself — which model wrote the points, how many
    /// cards it was shown, what it thought was wrong. Declared so the file parses
    /// and not stored: it is a record of the DRAFTING, and the card carries the
    /// draft, not the drafting.
    #[serde(default, flatten)]
    #[allow(dead_code, reason = "declared so an undeclared key still refuses")]
    pub run_report: RunReport,
}

/// The reporting block every Job D file carries around its payload.
///
/// ## Rust Learning: `#[serde(flatten)]` on a shared block
///
/// `flatten` splices this struct's keys into the PARENT's object rather than
/// nesting them under a `run_report` key — which is what the files actually look
/// like. It lets the two file shapes share one declaration of the seven fields
/// they have in common, instead of repeating them and letting the copies drift.
///
/// Note the one interaction to remember: a `flatten`ed field turns off
/// `deny_unknown_fields` on its own struct (serde cannot tell whose key an
/// unknown one is), so the PARENT keeps the denial and this block is the
/// declared home for everything the parent does not name itself.
// serde: allows unknown fields because this struct is `#[serde(flatten)]`'d into
// its parents, and serde cannot apply `deny_unknown_fields` to a flattened type —
// it has no way to tell whose key an unknown one is. The parents
// (`TalkingPointsFile`, `CandidatesFile`) carry the denial, so an undeclared key
// is still refused at the top level; this struct is where the shared keys are
// DECLARED, not where the denial lives.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct RunReport {
    #[serde(default)]
    #[allow(
        dead_code,
        reason = "the scenario's own name; the store already has it"
    )]
    pub name: Option<String>,
    #[serde(default)]
    #[allow(
        dead_code,
        reason = "which model drafted; a record of the run, not the card"
    )]
    pub model: Option<String>,
    #[serde(default)]
    #[allow(dead_code, reason = "how many cards the drafter was shown")]
    pub cards_shown: Option<i64>,
    #[serde(default)]
    #[allow(dead_code, reason = "the gather file the ranker read")]
    pub gather_file: Option<String>,
    #[serde(default)]
    #[allow(dead_code, reason = "how big the pool was")]
    pub pool_size: Option<i64>,
    #[serde(default)]
    #[allow(dead_code, reason = "how deep the ranker read")]
    pub read_depth: Option<i64>,
    /// What the drafting job itself flagged. Declared and not acted on: these are
    /// the DRAFTER's complaints about its own output, and this loader's job is to
    /// carry the output, not to re-judge it. They stay in the file for a human.
    #[serde(default)]
    #[allow(
        dead_code,
        reason = "the drafter's own complaints; read by a human, not here"
    )]
    pub validation_problems: Vec<serde_json::Value>,
}

/// One talking point.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TalkingPoint {
    /// 1-based, as the file writes it and as the card's `backs_position` names it.
    pub position: i32,
    pub text: String,
    /// Job D's own note of which cards it thought backed this point, in three
    /// forms. Declared and NOT stored, deliberately: `backs_position` on the card
    /// is the one arrow between a card and a point, and a second list here would
    /// be a second answer to the same question — free to disagree with the first
    /// the moment a human re-points a card.
    #[serde(default)]
    #[allow(dead_code, reason = "backs_position on the card is the single arrow")]
    pub backed_by: Vec<serde_json::Value>,
    #[serde(default)]
    #[allow(dead_code, reason = "backs_position on the card is the single arrow")]
    pub backed_by_c_codes: Vec<serde_json::Value>,
    #[serde(default)]
    #[allow(dead_code, reason = "backs_position on the card is the single arrow")]
    pub backed_by_card_ids: Vec<serde_json::Value>,
    /// Why the drafter chose those cards. Parsed, unstored — the same class as a
    /// pick's `reason` (see [`Pick`]).
    #[serde(default)]
    #[allow(dead_code, reason = "no field on the point; reported, not stored")]
    pub why_these_cards: Option<String>,
}

/// One scenario's ten picks (`D_S-*_candidates.jsonl`).
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CandidatesFile {
    pub scenario_code: String,
    pub scenario_id: uuid::Uuid,
    pub picks: Vec<Pick>,
    /// The candidates file carries the scenario's talking points too. They are
    /// loaded by `--talking-points` from `D_talking_points.jsonl`, which is the
    /// one file that mode reads, so the copy here is declared and not used —
    /// loading the same points from two files is how two versions of one point
    /// end up in the store.
    #[serde(default)]
    #[allow(
        dead_code,
        reason = "--talking-points reads D_talking_points.jsonl only"
    )]
    pub talking_points: Vec<TalkingPoint>,
    #[serde(default, flatten)]
    #[allow(dead_code, reason = "declared so an undeclared key still refuses")]
    pub run_report: RunReport,
}

/// One picked candidate.
///
/// ## Domain note: a pick carries a `reason`, and this loader does not store it
///
/// Job D wrote one sentence per pick explaining why it chose that card. There is
/// no field for it on `scenario_fact_cards` — §1's five are the card's own
/// sentences, and "why the ranker picked this" is a different claim. Mapping it
/// onto `watch_out` would put a ranking note where a witness expects to read how
/// the other side will use the fact. So it is parsed, COUNTED, and reported
/// unstored rather than silently dropped or silently mis-filed.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Pick {
    /// 1-based display order — the position the fact takes in the scenario.
    pub pick: i32,
    pub graph_node_id: String,
    pub title: String,
    #[serde(default)]
    pub reason: Option<String>,
    /// The C-code the ranker had for this card, and where it sat in the gather.
    /// Declared, not stored: the C-code is minted per scenario by
    /// `scenario_candidate_ordinals` (same reason as [`DraftedCard::c_code`]), and
    /// the ranker's own position is superseded by `pick`, which is the order
    /// Roman will actually see and re-order.
    #[serde(default)]
    #[allow(
        dead_code,
        reason = "the C-code is minted per scenario, not carried in"
    )]
    pub c_code: Option<String>,
    #[serde(default)]
    #[allow(dead_code, reason = "superseded by `pick`, the order the human sees")]
    pub gather_rank: Option<i64>,
    #[serde(default)]
    #[allow(dead_code, reason = "superseded by `pick`, the order the human sees")]
    pub k: Option<i64>,
}

/// Read a JSONL file into one struct per line.
///
/// ## Rust Learning: a generic over `DeserializeOwned`
///
/// `T: DeserializeOwned` means "any type serde can build without borrowing from
/// the input" — necessary because each line is a temporary `&str` that does not
/// outlive the loop. The three shapes above all satisfy it, so one reader serves
/// all three and the line number in the error is written once.
///
/// # Errors
/// Returns the path and the 1-based line number of the first line that will not
/// parse. A file half-read is not loaded at all: the caller stops.
pub fn read_jsonl<T: serde::de::DeserializeOwned>(path: &std::path::Path) -> Result<Vec<T>> {
    let text =
        std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    text.lines()
        .enumerate()
        .filter(|(_, line)| !line.trim().is_empty())
        .map(|(index, line)| {
            serde_json::from_str::<T>(line)
                .with_context(|| format!("{}:{}", path.display(), index + 1))
        })
        .collect()
}

#[cfg(test)]
#[path = "model_tests.rs"]
mod tests;
