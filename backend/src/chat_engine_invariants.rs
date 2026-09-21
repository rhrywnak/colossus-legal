//! Disk/code consistency test (CLAUDE.md rule 21): the chat-engine crate stays
//! CASE-BLIND.
//!
//! The reusability checkpoint (Standing Rule 11) asks whether colossus-ai could use
//! `colossus-chat` with zero code changes. Review alone cannot keep that true as
//! the crate grows; this test reads every source file in the crate and fails on
//! any word that belongs to THIS case or THIS product. The vocabulary is the
//! test's own fixture — adding a person to the case means adding them here.

#[cfg(test)]
mod tests {
    use std::path::Path;

    /// STRUCTURAL: a repo-internal source path, not deployment configuration.
    const CRATE_SRC: &str = "crates/colossus-chat/src";

    /// Words that would mean the crate knows which case it is serving. Matched
    /// case-insensitively. "colossus-legal" is allowed only as the name of the
    /// repo the shared code moved OUT of, in its provenance notes.
    const CASE_WORDS: &[&str] = &[
        "marie",
        "chuck",
        "roman",
        "phillips",
        "awad",
        "penzien",
        "catholic",
        "cfs",
        "witness",
        "practice",
        "scenario",
        "testimony",
        "deposition",
        "interrogator",
        "court",
        "attorney",
        "probate",
        "estate",
        "trial",
    ];

    fn sources(dir: &Path, out: &mut Vec<(String, String)>) {
        let entries =
            std::fs::read_dir(dir).unwrap_or_else(|e| panic!("{} is readable: {e}", dir.display()));
        for entry in entries {
            let path = entry.expect("a directory entry is readable").path();
            if path.is_dir() {
                sources(&path, out);
            } else if path.extension().and_then(|e| e.to_str()) == Some("rs") {
                let text = std::fs::read_to_string(&path).expect("a source file is readable");
                out.push((path.display().to_string(), text));
            }
        }
    }

    #[test]
    fn the_chat_crate_names_no_case_and_no_product() {
        let mut files = Vec::new();
        sources(Path::new(CRATE_SRC), &mut files);
        assert!(
            files.len() >= 8,
            "the crate's sources were not found at {CRATE_SRC}"
        );
        for (path, text) in files {
            // The provenance note ("Moved here from `colossus-legal`'s …") names the
            // repo the code came from, not the case, and is allowed.
            let lower = text.to_lowercase().replace("colossus-legal", "");
            for word in CASE_WORDS {
                assert!(
                    !lower.contains(word),
                    "{path} contains `{word}` — the chat engine must stay case-blind; \
                     move the case-specific part to the consuming crate"
                );
            }
        }
    }
}
