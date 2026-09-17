//! Unit tests for [`super::next_hand_key`] — the numbering rule, without a store.

use super::{next_hand_key, HAND_ADDED_PREFIX};

/// A deck that has never had a hand-added question starts at `x1`.
#[test]
fn the_first_minted_key_on_a_deck_is_x1() {
    assert_eq!(next_hand_key::<String>(&[]), "x1");
}

/// A minted key NEVER collides with a key the deck file could write. (M)
///
/// ## Why this is the test that matters
///
/// `seed_practice_deck --update` matches stored rows to file questions BY KEY
/// (`practice::seed_update`), so a hand-added question holding `g6` would be
/// rewritten with the text of whatever the architect writes as `g6` next — one
/// question's words replaced by another's, silently, on a deck Marie practises
/// from. The file's namespace is `g`/`c`/`r`; ours is `x`, and the two cannot
/// meet.
#[test]
fn a_minted_key_is_never_one_the_deck_file_could_write() {
    let file_keys = ["g1", "g2", "g57", "c1", "c48", "r1", "r20"];
    let minted = next_hand_key(&file_keys);

    assert_eq!(minted, "x1", "the file's own keys are none of our business");
    assert!(minted.starts_with(HAND_ADDED_PREFIX));
    assert!(
        !file_keys.contains(&minted.as_str()),
        "a minted key must be unreachable from the file's namespace"
    );
}

/// Minting continues from the highest `x` already on the deck. (M)
///
/// Not `count + 1`: a deck that minted `x1`, `x2`, `x3` and had `x2` re-keyed
/// into the file by hand would otherwise mint `x3` again and hit the UNIQUE
/// constraint — a refused add for a question that is perfectly addable.
#[test]
fn minting_continues_past_the_highest_key_not_the_count() {
    assert_eq!(next_hand_key(&["x1", "x2", "x3"]), "x4");
    // x2 has left; the next one is still x4.
    assert_eq!(next_hand_key(&["x1", "x3"]), "x4");
    // And it is numeric order, not string order — "x9" beats "x10" as text.
    assert_eq!(next_hand_key(&["x9", "x10"]), "x11");
}

/// A mixed deck counts only its own namespace.
#[test]
fn a_mixed_deck_numbers_from_its_x_keys_alone() {
    assert_eq!(next_hand_key(&["g1", "x1", "c2", "x2", "r1"]), "x3");
}

/// Something that merely LOOKS like ours is not ours.
///
/// `strip_prefix` alone would accept `xenon`; the `parse` is what refuses it.
/// Without that second half, one odd stored key would make every future mint
/// fall back to `x1` and collide.
#[test]
fn a_key_that_is_not_a_number_after_the_prefix_is_ignored() {
    assert_eq!(next_hand_key(&["xenon", "x-1", "x", "x2a"]), "x1");
    assert_eq!(next_hand_key(&["xenon", "x4"]), "x5");
}

/// Stored values are trimmed before they are read.
///
/// The column is `TEXT` with no trim constraint, and a key that arrived with a
/// trailing space would otherwise be invisible to the parse and collide on the
/// next mint.
#[test]
fn surrounding_whitespace_never_hides_a_key() {
    assert_eq!(next_hand_key(&[" x7 "]), "x8");
}
