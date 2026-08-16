//! The exit codes are a runbook CONTRACT, so their VALUES are asserted, not just
//! their existence. A refactor that renumbered them would silently change what a
//! runbook step means; these tests make that a build failure instead.

use super::*;

#[test]
fn codes_have_their_ruled_values() {
    // Ruled 2026-08-14. Changing any of these changes what a runbook step means.
    assert_eq!(EXIT_OK, 0);
    assert_eq!(EXIT_BAD_INPUT, 1);
    assert_eq!(EXIT_CONNECTION, 2);
    assert_eq!(EXIT_UNIT_ABORTED, 3);
    assert_eq!(EXIT_UNSAFE_PLAN, 4);
    assert_eq!(EXIT_EXECUTION_FAILED, 5);
}

#[test]
fn no_two_codes_collide() {
    let mut seen: Vec<u8> = ALL_EXIT_CODES.iter().map(|(c, _)| *c).collect();
    let before = seen.len();
    seen.sort_unstable();
    seen.dedup();
    assert_eq!(
        seen.len(),
        before,
        "two exit codes share a number; the runbook cannot tell them apart"
    );
}

#[test]
fn the_registry_lists_every_defined_code() {
    let listed: Vec<u8> = ALL_EXIT_CODES.iter().map(|(c, _)| *c).collect();
    for code in [
        EXIT_OK,
        EXIT_BAD_INPUT,
        EXIT_CONNECTION,
        EXIT_UNIT_ABORTED,
        EXIT_UNSAFE_PLAN,
        EXIT_EXECUTION_FAILED,
    ] {
        assert!(
            listed.contains(&code),
            "exit code {code} is defined but missing from ALL_EXIT_CODES, so it \
             would not appear in any binary's --help"
        );
    }
}

#[test]
fn help_text_names_every_code() {
    let help = help_text();
    for (code, meaning) in ALL_EXIT_CODES {
        assert!(help.contains(&code.to_string()), "help omits code {code}");
        assert!(
            help.contains(meaning),
            "help omits the meaning of code {code}"
        );
    }
}
