//! Unit tests for [`super`] — the billing vocabulary.

use super::*;

#[test]
fn every_known_class_round_trips() {
    for class in [BillingClass::Local, BillingClass::Billed] {
        assert_eq!(
            BillingClass::try_from(class.code()).expect("its own token"),
            class
        );
    }
}

/// An unknown token is refused BY NAME, and the message says what the refusal
/// costs the human — the model does not appear in the picker.
#[test]
fn an_unknown_class_is_refused_and_says_what_that_means() {
    let Err(error) = BillingClass::try_from("free-tier") else {
        panic!("an unknown billing class must be refused");
    };
    let message = error.to_string();
    assert!(message.contains("free-tier"), "the bad token: {message}");
    assert!(message.contains("local"), "the vocabulary: {message}");
    assert!(message.contains("billed"), "the vocabulary: {message}");
}

/// Only the class that costs money carries a label.
///
/// A suffix on every row is a suffix nobody reads; the one that changes what a
/// click COSTS is the one that gets words.
#[test]
fn only_the_billed_class_is_labelled() {
    assert_eq!(BillingClass::Local.suffix(), None);
    assert_eq!(BillingClass::Billed.suffix(), Some("(API — billed)"));
}

/// BOTH classes speak in the confirmation, unlike the picker.
///
/// The two vocabularies answer different questions at different moments, and
/// the distinction is the point: `suffix` leaves the free case undecorated
/// because a label on every row is a label nobody reads, while this is read at
/// the instant a human commits to spending, where "is this the free one?" is
/// exactly the question being asked. A parenthesis that appears only when the
/// answer is bad teaches the reader to skim past it.
///
/// Asserted here and not left to `chat_models_tests`: that file pins the
/// ASSEMBLED label, so a slip in this method would surface there as a broken
/// sentence about a model rather than as a wrong statement about a class.
#[test]
fn the_confirmation_states_the_cost_for_both_classes() {
    assert_eq!(BillingClass::Local.confirm_suffix(), "(local · $0)");
    assert_eq!(BillingClass::Billed.confirm_suffix(), "(API — billed)");
}

/// The free class says `$0` in the confirmation and stays silent in the picker.
///
/// The one property that would be lost if someone "tidied" the two methods into
/// one: they must not agree about `Local`.
#[test]
fn the_two_vocabularies_disagree_about_local_on_purpose() {
    assert_eq!(BillingClass::Local.suffix(), None);
    assert!(BillingClass::Local.confirm_suffix().contains("$0"));
    // …and they DO agree about billed, because there is one right way to warn
    // that a call is metered and two phrasings of it would be a drift waiting
    // to happen.
    assert_eq!(
        BillingClass::Billed.suffix(),
        Some(BillingClass::Billed.confirm_suffix())
    );
}

/// Local sorts before billed, and the ordering is a decision rather than a
/// side effect of variant declaration order.
#[test]
fn local_sorts_before_billed() {
    assert!(BillingClass::Local.sort_key() < BillingClass::Billed.sort_key());
}
