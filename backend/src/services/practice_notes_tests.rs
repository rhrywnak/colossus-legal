// Tests for `domain::services::practice_notes`.
//
// ## What used to be here
//
// Five tests over the note-composing functions — the struck line, the scenario
// partition, and the "new since her last sitting" count. Roman's ruling of
// 2026-08-23 retired notes from the interface, those functions went with the
// panels that called them, and their tests went with the functions. The
// `practice_notes` TABLE is untouched and keeps every row it has.
//
// `attribution` remains because the deck EDITOR signs its changes with it.

use super::*;

fn user(username: &str, display: &str) -> AuthUser {
    AuthUser {
        username: username.to_string(),
        email: String::new(),
        display_name: display.to_string(),
        groups: vec![],
    }
}

/// The id is the username; the printed name is the display name.
#[test]
fn attribution_takes_the_id_and_the_name_from_the_login() {
    let (id, name) = attribution(&user("chuck", "Chuck"));
    assert_eq!(id, "chuck");
    assert_eq!(name, "Chuck");
}

/// A blank display name falls back to the username, never to an empty author.
///
/// Authentik can hold an account with no name set. A note whose author renders
/// as nothing is a note nobody can answer — and this is the one place a name
/// could have arrived empty, because everything else on these tables is either
/// a stored row or something a human typed.
#[test]
fn a_blank_display_name_falls_back_to_the_username() {
    let (id, name) = attribution(&user("marie", "   "));
    assert_eq!(id, "marie");
    assert_eq!(name, "marie", "never an empty author");
}

/// Nothing here consults a stored list, and nothing can refuse a real user.
///
/// ANTI-REGRESSION: the two settings rows this replaced
/// (`practice_note_authors`, `practice_editor_authors`) are deleted by the
/// hotfix migration. If somebody reintroduces a vocabulary check, a signed-in
/// user whose Authentik display name is spelled differently from the row would
/// be silently unable to write a note — which is the class of fault this whole
/// task exists to remove.
#[test]
fn any_signed_in_user_is_attributable() {
    for (u, d) in [
        ("roman", "Roman"),
        ("chuck", "Chuck"),
        ("marie", "Marie"),
        ("j.doe", "J. Doe"),
    ] {
        let (id, name) = attribution(&user(u, d));
        assert_eq!(id, u);
        assert_eq!(name, d);
    }
}
