//! Reading the card bodies the exported list shows.
//!
//! READ-ONLY. The ranked gather returns ids and ranks; a human needs the words.

use std::collections::BTreeMap;

use sqlx::{PgPool, Row};

/// One card, as it appears on the page.
#[derive(Debug, Clone, Default)]
pub struct Card {
    pub evidence_id: String,
    /// The scenario's own C-number, when this card has one. `None` means the
    /// card is not in that scenario's numbered pool — which is exactly what the
    /// widening reaches, so it is common and not an error.
    pub c_number: Option<i32>,
    pub document_id: String,
    pub page: Option<i64>,
    pub title: String,
    pub quote: String,
    pub significance: String,
    pub about: Vec<String>,
}

// STRUCTURAL: SQL is wire vocabulary for the Postgres protocol. The mirror
// carries every field the list shows, so one read fills the whole page — the
// graph is not touched again.
const CARDS_SQL: &str = "\
    SELECT evidence_id, document_id, page, \
           coalesce(title, '') AS title, quote, \
           coalesce(significance, '') AS significance, about \
      FROM evidence_search \
     WHERE evidence_id = ANY($1::text[])";

// STRUCTURAL: the scenario's own numbering. `ordinal` IS the C-number a human
// says out loud ("C-54"), assigned per scenario, so the same card can be C-12
// in one scenario and unnumbered in another.
const ORDINALS_SQL: &str = "\
    SELECT graph_node_id, ordinal \
      FROM scenario_candidate_ordinals \
     WHERE scenario_id = $1::uuid";

/// Read every card named, plus the scenario's C-numbers.
///
/// Returns a map so the caller can look up in rank order without re-querying;
/// a card the mirror does not have is simply absent, and the caller reports the
/// gap rather than rendering a blank block.
///
/// # Errors
/// Returns the sqlx error if either statement fails.
pub async fn read_cards(
    mirror: &PgPool,
    pipeline: &PgPool,
    scenario_id: &str,
    ids: &[String],
) -> Result<BTreeMap<String, Card>, sqlx::Error> {
    // `ordinal` is INT4 in the store, so i32 — an i64 decode fails outright
    // rather than narrowing, which is the behaviour we want from sqlx here.
    let ordinals: BTreeMap<String, i32> = sqlx::query(ORDINALS_SQL)
        .bind(scenario_id)
        .fetch_all(pipeline)
        .await?
        .into_iter()
        .map(|row| Ok::<_, sqlx::Error>((row.try_get("graph_node_id")?, row.try_get("ordinal")?)))
        .collect::<Result<_, _>>()?;

    let mut cards = BTreeMap::new();
    for row in sqlx::query(CARDS_SQL).bind(ids).fetch_all(mirror).await? {
        let evidence_id: String = row.try_get("evidence_id")?;
        cards.insert(
            evidence_id.clone(),
            Card {
                c_number: ordinals.get(&evidence_id).copied(),
                document_id: row.try_get("document_id")?,
                page: row.try_get("page")?,
                title: row.try_get("title")?,
                quote: row.try_get("quote")?,
                significance: row.try_get("significance")?,
                about: row.try_get("about")?,
                evidence_id,
            },
        );
    }
    Ok(cards)
}

/// `C-54` when the card is numbered in this scenario, else its id.
///
/// The id is a fallback rather than a blank because an unnumbered card still
/// has to be findable — and under the widening most of them are unnumbered.
pub fn card_label(card: &Card) -> String {
    match card.c_number {
        Some(n) => format!("C-{n}"),
        None => card.evidence_id.clone(),
    }
}

#[cfg(test)]
#[path = "cards_tests.rs"]
mod tests;
