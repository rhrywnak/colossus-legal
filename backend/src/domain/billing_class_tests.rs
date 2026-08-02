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

/// Local sorts before billed, and the ordering is a decision rather than a
/// side effect of variant declaration order.
#[test]
fn local_sorts_before_billed() {
    assert!(BillingClass::Local.sort_key() < BillingClass::Billed.sort_key());
}
