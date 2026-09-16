//! Tests for `services::war_room_progress` — the status card's numbers.
//!
//! The Postgres families' SQL is proved against a scratch database in
//! `pipeline_repository::war_room_status_live_tests`; everything here is the pure
//! half: that the evidence counts are the scenario page's counts, and that the
//! fold never turns a missing read into a zero.

use chrono::{TimeZone, Utc};

use super::*;
use crate::domain::fact_status::FactStatus;
use crate::dto::scenario_card::{CardProposal, ProposalSource, ScenarioCard};
use crate::services::scenario_human_links::link_progress;
use crate::services::scenario_human_links::tests::{linked, machine_linked, stuck_card, wording};

fn id(n: u128) -> Uuid {
    Uuid::from_u128(n)
}

fn with_status(mut card: ScenarioCard, status: FactStatus) -> ScenarioCard {
    card.status = status;
    card
}

fn proposed(mut card: ScenarioCard) -> ScenarioCard {
    card.proposed = Some(CardProposal {
        role_label: Some("Scan: supports".to_string()),
        duplicate_count: 1,
        duplicate_label: None,
    });
    card
}

/// A payload shaped the way `get_scenario_cards` serves it: the assembler's
/// `link_progress`, and the route's `proposal_source` count.
fn served_payload() -> ScenarioCardsResponse {
    let pool = vec![
        stuck_card("ev-1", Vec::new()),                    // stuck, unlinked
        with_status(linked("ev-2"), FactStatus::Included), // stuck, human-linked
        with_status(machine_linked("ev-3"), FactStatus::Included), // never stuck
        proposed(stuck_card("ev-4", Vec::new())),          // stuck, proposed
        proposed(machine_linked("ev-5")),                  // proposed
    ];
    let set_aside = vec![with_status(linked("ev-6"), FactStatus::Dropped)];
    let link_progress = link_progress(pool.iter().chain(set_aside.iter()), &wording());
    let mut response = ScenarioCardsResponse {
        pool,
        set_aside,
        link_progress,
        no_target_notice: None,
        never_scanned_notice: None,
        proposal_source: None,
    };
    // Exactly the route's attachment (`api::scenario_cards`).
    let proposed_count = count_proposed(&response);
    response.proposal_source = Some(ProposalSource {
        run_id: id(99),
        model_id: "a-model".to_string(),
        started_at: Utc
            .with_ymd_and_hms(2026, 9, 11, 9, 0, 0)
            .single()
            .expect("a real instant"),
        proposed_count,
    });
    response
}

/// "N of M" out of the served sentence, read the way a person reads it.
fn numbers_in(line: &str) -> (usize, usize) {
    let words: Vec<&str> = line.split_whitespace().collect();
    let n = words[0].parse().expect("the line opens with a number");
    let m = words[2].parse().expect("the third word is a number");
    (n, m)
}

/// **The card is honest against the scenario page.**
///
/// The War Room's three evidence numbers are compared with what the card queue
/// SERVES for the same payload: the words of its progress line, the count on its
/// proposal source, and the Included cards its fact list shows. Mutation-proved
/// (report): adding one to any count in `evidence_counts` turns this red.
#[test]
fn dashboard_counts_equal_cards_route_counts() {
    let served = served_payload();
    let counts = evidence_counts(&served);

    let (linked, total) = numbers_in(served.link_progress.as_deref().expect("a stuck pile"));
    assert_eq!((counts.linked, counts.stuck), (linked, total));
    assert_eq!(
        counts.candidates_to_rule,
        served
            .proposal_source
            .as_ref()
            .expect("proposals")
            .proposed_count
    );
    let listed_as_included = served
        .pool
        .iter()
        .filter(|c| c.status == FactStatus::Included)
        .count();
    assert_eq!(counts.facts_included, listed_as_included);

    // And the literal numbers, so a payload helper that drifted with the code
    // could not make both sides agree on something wrong.
    assert_eq!(
        counts,
        EvidenceCounts {
            facts_included: 2,
            candidates_to_rule: 2,
            linked: 2,
            stuck: 4,
        }
    );
}

fn deck_row(scenario_id: Uuid) -> DeckCountsRow {
    DeckCountsRow {
        scenario_id,
        questions: 0,
        built_on: None,
        answered: 0,
        chuck_answered: 0,
        chuck_total: 0,
        defense_answered: 0,
        defense_total: 0,
    }
}

fn rows_for(ids: &[Uuid]) -> FamilyRows {
    FamilyRows {
        scans: Vec::new(),
        prep: ids
            .iter()
            .map(|&scenario_id| PrepCountsRow {
                scenario_id,
                talking_points: 0,
                watch_items: 0,
            })
            .collect(),
        deck: ids.iter().map(|&s| deck_row(s)).collect(),
        changed: ids
            .iter()
            .map(|&scenario_id| ChangedCountRow {
                scenario_id,
                changed: 0,
            })
            .collect(),
    }
}

/// A scenario with no deck, no scan and no augmentation is a card of zeroes —
/// every number present — and `last_scan` is `None`, the never-run fact.
#[test]
fn a_scenario_with_nothing_yet_is_all_zeroes_and_never_scanned() {
    let bare = id(1);
    let evidence = HashMap::from([(bare, EvidenceCounts::default())]);
    let folded = fold_progress(&[bare], &evidence, rows_for(&[bare])).expect("folds");
    assert_eq!(folded.get(&bare), Some(&ScenarioProgress::default()));
    assert_eq!(folded[&bare].last_scan, None);
}

/// Every family's numbers land in the field that names them.
#[test]
fn each_family_lands_in_its_own_field() {
    let s = id(7);
    let evidence = HashMap::from([(
        s,
        EvidenceCounts {
            facts_included: 10,
            candidates_to_rule: 33,
            linked: 4,
            stuck: 99,
        },
    )]);
    let built = Utc
        .with_ymd_and_hms(2026, 9, 14, 12, 0, 0)
        .single()
        .expect("instant");
    let rows = FamilyRows {
        scans: vec![LastScanRow {
            scenario_id: s,
            model_name: "Qwen3.8".to_string(),
            started_at: built,
            relevant: 43,
            total: 273,
        }],
        prep: vec![PrepCountsRow {
            scenario_id: s,
            talking_points: 5,
            watch_items: 4,
        }],
        deck: vec![DeckCountsRow {
            scenario_id: s,
            questions: 17,
            built_on: Some(built),
            answered: 3,
            chuck_answered: 1,
            chuck_total: 7,
            defense_answered: 2,
            defense_total: 10,
        }],
        changed: vec![ChangedCountRow {
            scenario_id: s,
            changed: 3,
        }],
    };
    let p = fold_progress(&[s], &evidence, rows)
        .expect("folds")
        .remove(&s)
        .expect("the card");
    assert_eq!((p.facts_included, p.candidates_to_rule), (10, 33));
    assert_eq!(
        p.matrix_linked,
        MatrixLinked {
            linked: 4,
            total: 99
        }
    );
    assert_eq!(
        p.last_scan,
        Some(LastScan {
            model_name: "Qwen3.8".to_string(),
            when: built,
            relevant: 43,
            total: 273
        })
    );
    assert_eq!((p.talking_points, p.watch_items), (5, 4));
    assert_eq!(
        p.deck,
        DeckSummary {
            questions: 17,
            built_on: Some(built)
        }
    );
    assert_eq!(
        p.answered,
        AnsweredSplit {
            total: 3,
            of: 17,
            chuck_answered: 1,
            chuck_total: 7,
            defense_answered: 2,
            defense_total: 10
        }
    );
    assert_eq!(p.marie_changed, 3);
}

/// A family that forgot a scenario is an error naming both — never a zero card.
#[test]
fn a_missing_family_row_is_an_error_not_a_zero() {
    let (a, b) = (id(1), id(2));
    let evidence = HashMap::from([
        (a, EvidenceCounts::default()),
        (b, EvidenceCounts::default()),
    ]);
    let mut rows = rows_for(&[a, b]);
    rows.deck.retain(|r| r.scenario_id == a);
    assert_eq!(
        fold_progress(&[a, b], &evidence, rows),
        Err(ProgressError::MissingRow {
            family: "deck",
            scenario_id: b
        })
    );
}

/// No evidence counts for a scenario is the same error, from the other read.
#[test]
fn missing_evidence_is_an_error_not_a_zero() {
    let a = id(1);
    assert_eq!(
        fold_progress(&[a], &HashMap::new(), rows_for(&[a])),
        Err(ProgressError::MissingRow {
            family: "evidence",
            scenario_id: a
        })
    );
}

/// A negative count is refused by name rather than wrapped into four billion.
#[test]
fn a_negative_count_is_refused_by_name() {
    let a = id(1);
    let evidence = HashMap::from([(a, EvidenceCounts::default())]);
    let mut rows = rows_for(&[a]);
    rows.changed[0].changed = -1;
    assert_eq!(
        fold_progress(&[a], &evidence, rows),
        Err(ProgressError::NotACount {
            field: "marie_changed",
            scenario_id: a,
            value: -1
        })
    );
}

/// The operator reads these in the log: both name what failed and where.
#[test]
fn progress_errors_name_the_family_field_and_scenario() {
    let missing = ProgressError::MissingRow {
        family: "deck",
        scenario_id: id(1),
    }
    .to_string();
    assert_eq!(
        missing,
        format!("the deck read returned no row for scenario {}", id(1))
    );
    let not_a_count = ProgressError::NotACount {
        field: "marie_changed",
        scenario_id: id(1),
        value: -1,
    }
    .to_string();
    assert_eq!(
        not_a_count,
        format!(
            "marie_changed for scenario {} is -1, which is not a count",
            id(1)
        )
    );
}
