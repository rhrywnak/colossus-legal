//! Tests for `scenario_fact_cards` (FACT_CARD_v2 §1).
//!
//! These pin the SQL's SHAPE and the read boundary rather than behaviour against
//! a live database, matching `evidence_allegation_links_tests`: the properties
//! that matter here — scenario scoping, one row per pair, one field per write,
//! and a ledger that cannot be edited — are readable from the statements, and
//! reading them needs no Postgres.

use super::*;

/// The upsert this module builds for one field.
fn upsert_sql(field: CardField) -> String {
    let column = field.code();
    let cast = field.sql_cast();
    format!(
        "INSERT INTO scenario_fact_cards \
             (scenario_id, graph_node_id, {column}, {column}_authored_by, \
              authored_by, created_at, edited_at) \
         VALUES ($1, $2, $3{cast}, $4, $4, $5, $5) \
         ON CONFLICT (scenario_id, graph_node_id) DO UPDATE SET \
             {column}             = EXCLUDED.{column}, \
             {column}_authored_by = EXCLUDED.{column}_authored_by, \
             authored_by          = EXCLUDED.authored_by, \
             edited_at            = EXCLUDED.edited_at"
    )
}

// ─── The key, and the scope ──────────────────────────────────────────────────

/// THE SCENARIO-SCOPED TEST. A card belongs to one attack, not to a statement.
///
/// The same words are proof of cooperation under one scenario and background
/// under another. A key on the node alone would put one scenario's ANSWER — the
/// sentence a witness says out loud — on another scenario's rehearsal page.
#[test]
fn a_card_is_keyed_by_the_scenario_and_the_statement() {
    for field in CardField::EDITABLE {
        let sql = upsert_sql(*field);
        assert!(
            sql.contains("ON CONFLICT (scenario_id, graph_node_id)"),
            "the conflict target must be the pair: {sql}"
        );
    }
    assert!(CARD_COLUMNS.contains("scenario_id"));
    assert!(INSERT_CARD_EVENT_SQL.contains("scenario_id"));
}

/// Re-editing a field updates it; it never stores a second row.
///
/// Two rows for one pair would be two answers to one question, on the surface
/// whose whole job is to give a witness one.
#[test]
fn re_editing_a_field_updates_the_card_rather_than_duplicating_it() {
    let sql = upsert_sql(CardField::Answer);
    assert!(sql.contains("INSERT INTO scenario_fact_cards"));
    assert!(sql.contains("DO UPDATE SET"));
    assert!(sql.contains("answer             = EXCLUDED.answer"));
}

/// ONE FIELD PER WRITE. An edit touches its own column and no other.
///
/// This is what makes the draft mark per-field: a whole-row write would re-stamp
/// all five authorships from one edit, and a card with an edited Answer could no
/// longer show a still-drafted Watch out.
#[test]
fn writing_one_field_touches_no_other_field() {
    for field in CardField::EDITABLE {
        let sql = upsert_sql(*field);
        for other in CardField::EDITABLE {
            if other == field {
                continue;
            }
            assert!(
                !sql.contains(&format!("{} =", other.code())),
                "writing {} must not touch {}: {sql}",
                field.code(),
                other.code()
            );
        }
    }
}

/// Every field's write stamps its OWN authorship column.
#[test]
fn every_field_write_stamps_its_own_authorship_column() {
    for field in CardField::EDITABLE {
        let sql = upsert_sql(*field);
        assert!(
            sql.contains(&format!("{}_authored_by", field.code())),
            "{} must stamp its own author: {sql}",
            field.code()
        );
    }
}

/// `created_at` and `edited_at` share ONE bind on insert.
///
/// Two separate binds would let them differ by a round trip, and "created at
/// 12:00:00.001, edited at 12:00:00.004" reads as an edit that never happened.
#[test]
fn a_new_card_gets_one_timestamp_for_both_columns() {
    let sql = upsert_sql(CardField::Title);
    assert!(
        sql.contains("VALUES ($1, $2, $3, $4, $4, $5, $5)"),
        "the author and both timestamps must share their binds: {sql}"
    );
}

/// `created_at` survives an edit; `edited_at` moves.
#[test]
fn an_edit_keeps_the_original_created_at() {
    let sql = upsert_sql(CardField::WatchOut);
    let branch = &sql[sql.find("DO UPDATE SET").expect("a conflict branch")..];
    assert!(
        !branch.contains("created_at"),
        "created_at must not be touched on conflict: {branch}"
    );
    assert!(branch.contains("edited_at"));
}

/// The two non-TEXT columns are cast, and the three TEXT ones are not.
///
/// One code path writes all five fields, and the cast is the whole of the
/// difference. Postgres refuses a TEXT bind into `backs_position` without one —
/// measured, on the first `--apply` against a real file — and NOT casting the
/// JSON one would store a quoted string that `supports_list` could never read.
#[test]
fn the_statement_carries_each_columns_own_cast() {
    assert!(upsert_sql(CardField::Supports).contains("$3::jsonb"));
    assert!(upsert_sql(CardField::BacksPosition).contains("$3::integer"));
    for field in [CardField::Title, CardField::WatchOut, CardField::Answer] {
        assert!(
            !upsert_sql(field).contains("::"),
            "{} is a TEXT column and needs no cast",
            field.code()
        );
    }
}

// ─── The ledger ──────────────────────────────────────────────────────────────

/// The ledger records which field, what it became, who and when.
#[test]
fn the_ledger_records_the_field_the_value_the_actor_and_the_time() {
    for column in [
        "scenario_id",
        "graph_node_id",
        "field",
        "value",
        "actor",
        "at",
    ] {
        assert!(
            INSERT_CARD_EVENT_SQL.contains(column),
            "the ledger must carry {column}: {INSERT_CARD_EVENT_SQL}"
        );
    }
}

/// The ledger is append-only: never updated, never deleted from.
///
/// It is the only record that a machine drafted a sentence and a human replaced
/// it — an UPDATE on the card table destroys the draft, and this is what
/// survives it.
#[test]
fn nothing_in_this_module_updates_or_deletes_the_ledger() {
    let source = include_str!("scenario_fact_cards.rs");
    assert!(!source.contains("UPDATE scenario_fact_card_events"));
    assert!(!source.contains("DELETE FROM scenario_fact_card_events"));
}

/// Nothing in this module deletes a CARD either.
///
/// Removing a fact from a scenario removes its `scenario_fact_refs` row; the card
/// stays, so re-including the fact does not lose the answer somebody wrote for
/// it. The scenario's own deletion takes it, via `ON DELETE CASCADE`.
#[test]
fn nothing_in_this_module_deletes_a_card() {
    let source = include_str!("scenario_fact_cards.rs");
    assert!(!source.contains("DELETE FROM scenario_fact_cards"));
}

// ─── The read boundary ───────────────────────────────────────────────────────

fn record_with(supports: Option<serde_json::Value>) -> CardRecord {
    let at = DateTime::<Utc>::from_timestamp(1_757_000_000, 0).expect("a valid instant");
    CardRecord {
        scenario_id: Uuid::nil(),
        graph_node_id: "doc-x:evidence:8a240a75".to_string(),
        title: Some("The court ordered the $50,000 back".to_string()),
        backs_position: Some(1),
        supports,
        watch_out: None,
        answer: None,
        title_authored_by: Some(crate::domain::fact_card::MACHINE_AUTHOR.to_string()),
        backs_position_authored_by: Some("roman".to_string()),
        supports_authored_by: None,
        watch_out_authored_by: None,
        answer_authored_by: None,
        authored_by: "roman".to_string(),
        created_at: at,
        edited_at: at,
    }
}

/// A well-formed supports list parses.
#[test]
fn a_well_formed_supports_list_parses() {
    use crate::domain::fact_card::CardStance;
    let record = record_with(Some(serde_json::json!([
        {"allegation_id": "45984d77", "stance": "supports"},
        {"allegation_id": "08c0731b", "stance": "rebuts"}
    ])));
    let list = record.supports_list().expect("the list parses");
    assert_eq!(list.len(), 2);
    assert_eq!(list[0].allegation_id, "45984d77");
    assert_eq!(list[1].stance, CardStance::Rebuts);
}

/// An ABSENT column is an empty list, not an error.
///
/// A card that names no accusation is the ordinary state — §2 renders it as an
/// em dash — and treating it as a failure would refuse the whole deck over the
/// most common shape in it.
#[test]
fn an_absent_supports_column_is_an_empty_list() {
    assert!(record_with(None)
        .supports_list()
        .expect("absent is not an error")
        .is_empty());
}

/// Malformed JSON is refused, and the refusal NAMES the card.
///
/// This is the whole argument for keeping `supports` a raw `Value` in `FromRow`.
/// sqlx would have said "could not decode column 'supports'"; an operator would
/// then have to find which of forty cards it meant.
#[test]
fn an_unreadable_supports_list_is_refused_and_names_the_card() {
    let record = record_with(Some(serde_json::json!([{"allegation_id": "x"}])));
    let err = record
        .supports_list()
        .expect_err("a missing stance is unreadable");
    let message = err.to_string();
    assert!(message.contains("8a240a75"), "names the card: {message}");
    assert!(
        message.contains("scenario"),
        "names the scenario: {message}"
    );
}

/// An unknown STANCE token is refused rather than defaulted.
///
/// A card whose stance could not be read must not render as "Supports": that
/// would tell a witness a statement helps the accusation against her when nobody
/// knows which way it cuts.
#[test]
fn an_unknown_stance_token_is_refused() {
    let record = record_with(Some(serde_json::json!([
        {"allegation_id": "x", "stance": "mentions"}
    ])));
    assert!(record.supports_list().is_err());
}

/// An extra key is refused — `deny_unknown_fields` on the stored shape.
#[test]
fn an_extra_key_in_a_supports_entry_is_refused() {
    let record = record_with(Some(serde_json::json!([
        {"allegation_id": "x", "stance": "supports", "weight": 3}
    ])));
    assert!(record.supports_list().is_err());
}

/// Each field reports its OWN author.
///
/// The draft mark is read from this. A lookup that returned the row's
/// `authored_by` for every field would clear every mark the moment one field was
/// edited — which is the defect the per-field columns exist to prevent.
#[test]
fn every_field_reports_its_own_author() {
    let record = record_with(None);
    assert_eq!(
        record.field_author(CardField::Title),
        Some(crate::domain::fact_card::MACHINE_AUTHOR)
    );
    assert_eq!(record.field_author(CardField::BacksPosition), Some("roman"));
    assert_eq!(record.field_author(CardField::Answer), None);
}

/// The ledger-only field has no author column and says so.
#[test]
fn the_whole_card_token_has_no_field_author() {
    assert_eq!(record_with(None).field_author(CardField::Card), None);
}

/// The projection carries every column `CardRecord` decodes.
///
/// The derive maps BY NAME, so a column dropped from the projection is a runtime
/// decode failure at the first read — on a live page, not in CI.
#[test]
fn the_projection_covers_every_decoded_field() {
    for column in [
        "scenario_id",
        "graph_node_id",
        "title",
        "backs_position",
        "supports",
        "watch_out",
        "answer",
        "title_authored_by",
        "backs_position_authored_by",
        "supports_authored_by",
        "watch_out_authored_by",
        "answer_authored_by",
        "authored_by",
        "created_at",
        "edited_at",
    ] {
        assert!(
            CARD_COLUMNS.contains(column),
            "the projection is missing {column}: {CARD_COLUMNS}"
        );
    }
}
