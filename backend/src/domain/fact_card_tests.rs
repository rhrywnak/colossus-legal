// Tests for `domain::fact_card`.
//
// Three closed vocabularies and one cap. What is asserted hardest is the two
// things that reach a database column and the one that maps between two
// vocabularies — the places where a wrong answer is stored rather than seen.

use super::*;

/// Every field token round-trips.
#[test]
fn every_field_code_parses_back_to_itself() {
    for field in CardField::ALL {
        assert_eq!(CardField::try_from(field.code()), Ok(*field));
    }
}

/// THE COLUMN-NAME TEST. A field's token IS its column name.
///
/// The writer builds `"{code}_authored_by"` from this, so a token that drifted
/// from its column would produce a SQL error at write time on a surface a human
/// is waiting on — or, worse, would match a different column.
#[test]
fn the_five_editable_tokens_are_the_column_names() {
    let codes: Vec<&str> = CardField::EDITABLE.iter().map(|f| f.code()).collect();
    assert_eq!(
        codes,
        vec!["title", "backs_position", "supports", "watch_out", "answer"]
    );
}

/// `card` is a ledger word and is NOT editable.
///
/// A client that could PUT it would replace five fields in one write, and the
/// ledger would record only that something happened.
#[test]
fn the_whole_card_token_is_not_editable() {
    assert!(!CardField::Card.is_editable());
    assert!(!CardField::EDITABLE.contains(&CardField::Card));
    for field in CardField::EDITABLE {
        assert!(field.is_editable(), "{} must be editable", field.code());
    }
}

/// It IS readable, because the loader writes it into the ledger.
#[test]
fn the_whole_card_token_still_parses() {
    assert_eq!(CardField::try_from("card"), Ok(CardField::Card));
}

/// An unknown field is refused by name, and the refusal lists what is known.
#[test]
fn an_unknown_field_is_refused_by_name() {
    let err = CardField::try_from("point").expect_err("'point' is not a field");
    assert_eq!(err.token, "point");
    assert!(err.to_string().contains("point"));
    assert!(err.to_string().contains("watch_out"));
}

/// The empty string is refused like any other unknown token.
#[test]
fn the_empty_field_token_is_refused() {
    assert!(CardField::try_from("").is_err());
}

/// The editable list has no duplicates and covers every non-ledger variant.
#[test]
fn the_editable_list_is_every_variant_except_the_ledger_word() {
    let mut editable: Vec<&str> = CardField::EDITABLE.iter().map(|f| f.code()).collect();
    let count = editable.len();
    editable.sort_unstable();
    editable.dedup();
    assert_eq!(editable.len(), count, "two editable fields collide");
    assert_eq!(CardField::ALL.len(), CardField::EDITABLE.len() + 1);
}

/// EVERY FIELD'S CAST MATCHES ITS COLUMN TYPE.
///
/// The defect this exists for: `backs_position` is an INTEGER column, the writer
/// binds TEXT, and without a cast Postgres refuses the statement. Every
/// SQL-shape test passed — they compare statements, and the statement was
/// well-formed — and it failed on the first `--apply` against a real file.
#[test]
fn each_field_carries_the_cast_its_column_type_needs() {
    assert_eq!(CardField::BacksPosition.sql_cast(), "::integer");
    assert_eq!(CardField::Supports.sql_cast(), "::jsonb");
    for text_field in [CardField::Title, CardField::WatchOut, CardField::Answer] {
        assert_eq!(
            text_field.sql_cast(),
            "",
            "{} is a TEXT column bound from TEXT",
            text_field.code()
        );
    }
}

/// A cast is either empty or begins with `::` — a malformed one would produce a
/// statement Postgres cannot parse at all.
#[test]
fn every_cast_is_empty_or_well_formed() {
    for field in CardField::ALL {
        let cast = field.sql_cast();
        assert!(
            cast.is_empty() || cast.starts_with("::"),
            "{} has a malformed cast {cast:?}",
            field.code()
        );
    }
}

/// Every stance token round-trips.
#[test]
fn every_stance_code_parses_back_to_itself() {
    for stance in CardStance::ALL {
        assert_eq!(CardStance::try_from(stance.code()), Ok(*stance));
    }
}

/// The two stored stance tokens, pinned by literal.
///
/// The JSON column holds them, the graph's own `r.stance` holds them, and the 59
/// drafted cards on disk hold them. A prettier synonym would make those files
/// unreadable — it did, measured, on the first dry run against `B_S5.jsonl`.
#[test]
fn the_stance_tokens_are_supports_and_rebuts() {
    assert_eq!(CardStance::Supports.code(), "supports");
    assert_eq!(CardStance::Rebuts.code(), "rebuts");
    assert_eq!(CardStance::ALL.len(), 2);
}

/// An unknown stance is refused by name.
#[test]
fn an_unknown_stance_is_refused_by_name() {
    let err = CardStance::try_from("mentions").expect_err("not a stance");
    assert_eq!(err.token, "mentions");
    assert!(err.to_string().contains("supports/rebuts"));
}

/// THE MAP THAT COULD BE BACKWARDS.
///
/// A card that SUPPORTS an accusation against Marie is a HAZARD to her, so its
/// link cut is `against`; one that REBUTS the accusation helps her, so its cut
/// is `supports`. The two vocabularies share the word "supports" with opposite
/// subjects, which is precisely why this is a named function with a test rather
/// than an inline match at the call site.
#[test]
fn a_supporting_stance_is_a_hazard_and_a_rebutting_one_is_ammunition() {
    use crate::domain::link_cut::LinkCut;
    assert_eq!(CardStance::Supports.to_link_cut(), LinkCut::Against);
    assert_eq!(CardStance::Rebuts.to_link_cut(), LinkCut::Supports);
}

/// The map is total and injective — two stances that mapped to one cut would
/// make the link table unable to tell a weapon from a landmine.
#[test]
fn the_two_stances_map_to_two_different_cuts() {
    let cuts: Vec<&str> = CardStance::ALL
        .iter()
        .map(|s| s.to_link_cut().code())
        .collect();
    let mut unique = cuts.clone();
    unique.sort_unstable();
    unique.dedup();
    assert_eq!(unique.len(), cuts.len(), "two stances share a cut");
}

/// A machine author reads as a draft; a person does not.
#[test]
fn a_machine_author_is_a_draft_and_a_person_is_not() {
    assert!(CardAuthor(MACHINE_AUTHOR).is_draft());
    assert!(!CardAuthor("roman").is_draft());
    assert!(!CardAuthor("chuck").is_draft());
}

/// A SECOND drafting job still reads as a draft.
///
/// The prefix is what is matched, not the whole token. A card that stopped
/// showing the mark because the job name changed would silently claim a human had
/// been through it — on a deck a witness reads.
#[test]
fn a_later_drafting_job_still_reads_as_a_draft() {
    assert!(CardAuthor("machine:job_c_v2").is_draft());
    assert!(CardAuthor(MACHINE_AUTHOR_PREFIX).is_draft());
}

/// A username that merely CONTAINS the prefix is not a draft.
#[test]
fn a_name_containing_the_prefix_is_not_a_draft() {
    assert!(!CardAuthor("not-a-machine:job").is_draft());
}

/// The loader's token carries the prefix, so the two constants cannot drift.
#[test]
fn the_loader_token_carries_the_machine_prefix() {
    assert!(MACHINE_AUTHOR.starts_with(MACHINE_AUTHOR_PREFIX));
}

/// The cap is two, and it is the number Job B was given.
///
/// Pinned by literal because the input files on disk were produced against it:
/// their `over_supports_cap` flag says a third accusation was DROPPED, so a build
/// that read three would be reading a list nothing ever wrote.
#[test]
fn a_card_names_at_most_two_accusations() {
    assert_eq!(SUPPORTS_CAP, 2);
}
