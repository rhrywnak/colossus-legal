//! One ping path (Law 22), held by a scan of the source tree.
//!
//! ## Why a disk scan and not a scripted round trip
//!
//! `send` takes the whole `AppState` (pools, graph, registry), and no unit test in
//! this crate can build one — the only builders are the stale integration files
//! under `tests/`. What must hold is structural: exactly ONE place calls the
//! provider's pre-warm and sets the shared last-ping time, and both callers go
//! through it. A scan says that directly, and fails the day a second path appears.

use super::*;

/// Every non-test `.rs` file under `src/`, with its text.
fn production_sources() -> Vec<(String, String)> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut out = Vec::new();
    let mut stack = vec![root];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir).expect("src is readable") {
            let path = entry.expect("a directory entry").path();
            let name = path.to_string_lossy().to_string();
            if path.is_dir() {
                stack.push(path);
            } else if name.ends_with(".rs") && !name.ends_with("_tests.rs") {
                let text = std::fs::read_to_string(&path).expect("a readable source file");
                out.push((name, text));
            }
        }
    }
    out
}

fn files_containing(needle: &str) -> Vec<String> {
    production_sources()
        .into_iter()
        .filter(|(_, text)| text.contains(needle))
        .map(|(name, _)| name)
        .collect()
}

#[test]
fn only_this_module_sends_a_prewarm_or_sets_the_last_ping() {
    for needle in [".prewarm(&", "keepwarm_last_ping.set("] {
        let found = files_containing(needle);
        assert_eq!(found.len(), 1, "{needle} appears in {found:?}");
        assert!(
            found[0].ends_with("chat_keepwarm_ping.rs"),
            "{needle} in {found:?}"
        );
    }
}

#[test]
fn the_button_and_the_pinger_both_go_through_it() {
    let button = files_containing("ping(state, Trigger::Button)");
    assert!(
        button
            .iter()
            .any(|f| f.ends_with("chat_keepwarm_button.rs")),
        "{button:?}"
    );
    let pinger = files_containing("send(state, prepared, Trigger::Automatic)");
    assert!(
        pinger.iter().any(|f| f.ends_with("chat_keepwarm_task.rs")),
        "{pinger:?}"
    );
}

#[test]
fn each_trigger_has_its_own_log_name() {
    assert_eq!(Trigger::Button.name(), "button");
    assert_eq!(Trigger::Automatic.name(), "automatic");
}
