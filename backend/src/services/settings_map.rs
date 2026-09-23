//! Which block of the configuration a stored key belongs to (§2b).
//!
//! The admin Settings page shows 860-odd parameters. A flat list of 860 rows is
//! not a page anyone can use, so the rows are bundled into AREAS ("Practice",
//! "Timeline") and, inside each area, into BLOCKS ("Printing", "Analysis
//! report"). This module is where that bundling is declared, and it is the ONLY
//! place in the system where it is declared.
//!
//! ## Why the bundling is BY REFERENCE, never by re-listing keys
//!
//! Every block below points at a `*_KEYS` const that already exists — the same
//! list the wording builder walks and the seeding test pins to the migration.
//! Not one key is re-typed here. A second copy of `CHRONOLOGY_WORDING_KEYS`
//! would drift the first time a key was added to one and not the other, and the
//! drift would show up as a row silently missing from the page: present in the
//! store, editable by nobody, because no block claimed it.
//!
//! `settings_map_tests` asserts the property that makes this safe — every
//! declared key lands in exactly one block, and every block a module declares is
//! bundled here.
//!
//! ## Why the LABELS live on this side of the wire
//!
//! "Analysis report" is a sentence a human reads, and everything user-visible on
//! this page arrives composed (see `dto::settings`). The browser receives the
//! bundling as data; it holds no copy of it and cannot compute one. That is what
//! lets the page itself stay free of anything specific to this case or even to
//! this application — another Colossus app serving the same contract ships its
//! own areas here and changes not one line of the page.
//!
//! ## Rust Learning: `&'static [&'static str]`
//!
//! A `Block` borrows its key list rather than owning a `Vec<String>`. `'static`
//! means "lives for the whole program", which is exactly true of a `const` array
//! baked into the binary: there is nothing to allocate, nothing to clone, and a
//! `Block` is three pointers wide. The lifetime is not a restriction here so much
//! as a promise — these lists outlive every request that reads them.

use std::collections::HashMap;
use std::sync::OnceLock;

use super::settings_store::REQUIRED_KEYS;
use crate::domain::{
    practice_params::PRACTICE_PARAM_KEYS, wording::WORDING_KEYS,
    wording_accusation::ACCUSATION_WORDING_KEYS, wording_authoring::AUTHORING_WORDING_KEYS,
    wording_card_grammar::CARD_GRAMMAR_WORDING_KEYS,
    wording_chronology_keys::CHRONOLOGY_WORDING_KEYS, wording_env_banner::ENV_BANNER_WORDING_KEYS,
    wording_fact_card::FACT_CARD_WORDING_KEYS, wording_for_you::FOR_YOU_WORDING_KEYS,
    wording_matrix::MATRIX_WORDING_KEYS, wording_model_params::MODEL_PARAMS_WORDING_KEYS,
    wording_practice::PRACTICE_WORDING_KEYS,
    wording_practice_discuss::PRACTICE_DISCUSS_WORDING_KEYS,
    wording_practice_editor::PRACTICE_EDITOR_WORDING_KEYS,
    wording_practice_flow::PRACTICE_FLOW_WORDING_KEYS,
    wording_practice_list::PRACTICE_LIST_WORDING_KEYS,
    wording_practice_print::PRACTICE_PRINT_WORDING_KEYS,
    wording_practice_report::PRACTICE_REPORT_WORDING_KEYS,
    wording_practice_review::PRACTICE_REVIEW_WORDING_KEYS,
    wording_practice_row::PRACTICE_ROW_WORDING_KEYS, wording_rehearsal::REHEARSAL_WORDING_KEYS,
    wording_rehearsal_chrome::REHEARSAL_CHROME_KEYS, wording_scan::SCAN_WORDING_KEYS,
    wording_scenario_authoring::SCENARIO_AUTHORING_WORDING_KEYS,
    wording_war_room::WAR_ROOM_WORDING_KEYS,
    wording_war_room_summary::WAR_ROOM_SUMMARY_WORDING_KEYS,
};

/// One openable group inside an area.
#[derive(Debug, Clone, Copy)]
pub struct Block {
    /// Stable identifier — what the page's URL and its tests name.
    pub id: &'static str,
    /// The heading a human reads.
    pub label: &'static str,
    /// The existing const this block IS. Never a fresh list.
    pub keys: &'static [&'static str],
}

/// One entry in the page's left-hand rail.
#[derive(Debug, Clone, Copy)]
pub struct Area {
    pub id: &'static str,
    pub label: &'static str,
    pub blocks: &'static [Block],
}

/// Where one key sits: the area and block that claim it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Placement {
    pub area_id: &'static str,
    pub block_id: &'static str,
}

/// The area that holds stored rows no block claims.
///
/// Its rows are not a category of setting — they are a DEFECT, and the page says
/// so. See [`UNDECLARED_AREA_NOTE`].
pub const UNDECLARED_AREA_ID: &str = "undeclared";
/// The single block inside that area.
pub const UNDECLARED_BLOCK_ID: &str = "undeclared_dead";
/// The rail label, ruled 2026-09-19.
pub const UNDECLARED_AREA_LABEL: &str = "Undeclared — read by nothing";
/// Said under the heading, because a row that cannot be found is worse than a
/// row that is plainly labelled dead.
pub const UNDECLARED_AREA_NOTE: &str =
    "No constant in this build declares these rows and no code path reads them. \
     They are dead: editing one changes nothing anywhere. Retiring them is a \
     separate task — they are listed here so they stay visible until it happens.";

/// Every area, in the order the rail shows them.
///
/// ## Domain note: the order is the shape of the product, not an alphabet
///
/// Practice is first because it is where the work happens and where two thirds
/// of the parameters live. "Core" — the handful of numbers that decide how the
/// scenario function itself behaves — sits near the bottom because it is the one
/// area a human should need least often, and "Undeclared" is last because
/// nothing in it does anything.
pub const AREAS: &[Area] = &[
    Area {
        id: "practice",
        label: "Practice",
        blocks: &[
            Block {
                id: "practice_params",
                label: "Parameters — limits, caps, the reviewers",
                keys: PRACTICE_PARAM_KEYS,
            },
            Block {
                id: "practice_flow",
                label: "Practice flow — the walk, one question at a time",
                keys: PRACTICE_FLOW_WORDING_KEYS,
            },
            Block {
                id: "practice_row",
                label: "Question rows — the deck list",
                keys: PRACTICE_ROW_WORDING_KEYS,
            },
            Block {
                id: "practice_editor",
                label: "Editor — Edit the deck",
                keys: PRACTICE_EDITOR_WORDING_KEYS,
            },
            Block {
                id: "question_chat_params",
                label: "Discussion — model, prompt, narrative and limits",
                keys: crate::domain::chat_params::QUESTION_CHAT_PARAM_KEYS,
            },
            Block {
                id: "question_chat",
                label: "Discussion panel — the words beside a question",
                keys: crate::domain::wording_question_chat::QUESTION_CHAT_WORDING_KEYS,
            },
            Block {
                id: "practice_discuss",
                label: "Discuss with AI (the earlier dock)",
                keys: PRACTICE_DISCUSS_WORDING_KEYS,
            },
            Block {
                id: "practice_print",
                label: "Printing",
                keys: PRACTICE_PRINT_WORDING_KEYS,
            },
            Block {
                id: "practice_list",
                label: "Answer lists",
                keys: PRACTICE_LIST_WORDING_KEYS,
            },
            Block {
                id: "for_you",
                label: "For you — one person's list, across every deck",
                keys: FOR_YOU_WORDING_KEYS,
            },
            Block {
                id: "practice_review",
                label: "Answer review page",
                keys: PRACTICE_REVIEW_WORDING_KEYS,
            },
            Block {
                id: "practice_report",
                label: "Analysis report",
                keys: PRACTICE_REPORT_WORDING_KEYS,
            },
            Block {
                id: "practice_wording",
                label: "General practice wording",
                keys: PRACTICE_WORDING_KEYS,
            },
        ],
    },
    Area {
        id: "timeline",
        label: "Timeline",
        blocks: &[Block {
            id: "chronology",
            label: "Chronology — the timeline page end to end",
            keys: CHRONOLOGY_WORDING_KEYS,
        }],
    },
    Area {
        id: "scenarios",
        label: "Scenarios",
        blocks: &[
            Block {
                id: "scenario_authoring",
                label: "Authoring a scenario",
                keys: SCENARIO_AUTHORING_WORDING_KEYS,
            },
            Block {
                id: "accusation",
                label: "Accusations and their elements",
                keys: ACCUSATION_WORDING_KEYS,
            },
            Block {
                id: "fact_card",
                label: "Fact cards — the witness's own words",
                keys: FACT_CARD_WORDING_KEYS,
            },
        ],
    },
    Area {
        id: "rehearsal",
        label: "Rehearsal",
        blocks: &[
            Block {
                id: "rehearsal",
                label: "The rehearsal itself",
                keys: REHEARSAL_WORDING_KEYS,
            },
            Block {
                id: "rehearsal_chrome",
                label: "Rehearsal chrome — the frame around it",
                keys: REHEARSAL_CHROME_KEYS,
            },
        ],
    },
    Area {
        id: "general_wording",
        label: "General wording",
        blocks: &[Block {
            id: "wording",
            label: "Words shared across every page",
            keys: WORDING_KEYS,
        }],
    },
    Area {
        id: "war_room",
        label: "War Room",
        blocks: &[
            Block {
                id: "war_room",
                label: "The board",
                keys: WAR_ROOM_WORDING_KEYS,
            },
            Block {
                id: "war_room_summary",
                label: "The summary strip",
                keys: WAR_ROOM_SUMMARY_WORDING_KEYS,
            },
        ],
    },
    Area {
        id: "scan_and_models",
        label: "Scan & models",
        blocks: &[
            Block {
                id: "scan",
                label: "Scanning documents for candidates",
                keys: SCAN_WORDING_KEYS,
            },
            Block {
                id: "model_params",
                label: "Model parameters",
                keys: MODEL_PARAMS_WORDING_KEYS,
            },
        ],
    },
    Area {
        id: "matrix",
        label: "Matrix",
        blocks: &[Block {
            id: "matrix",
            label: "The coverage matrix",
            keys: MATRIX_WORDING_KEYS,
        }],
    },
    Area {
        id: "warnings",
        label: "Warnings",
        blocks: &[Block {
            id: "env_banner",
            // The rows only; WHETHER the bar shows is the browser's own runtime
            // config, and no Settings edit can turn it off (CC_TASK_ENV_BANNER_v1).
            label: "The test-system bar — its words, and where it links",
            keys: ENV_BANNER_WORDING_KEYS,
        }],
    },
    Area {
        id: "core",
        label: "Core",
        blocks: &[Block {
            id: "core",
            label: "The numbers the scenario function itself reads",
            keys: REQUIRED_KEYS,
        }],
    },
    Area {
        id: "other",
        label: "Other",
        blocks: &[
            Block {
                id: "card_grammar",
                label: "Card grammar — how one card reads",
                keys: CARD_GRAMMAR_WORDING_KEYS,
            },
            Block {
                id: "authoring",
                label: "Authoring — entities and relationships",
                keys: AUTHORING_WORDING_KEYS,
            },
        ],
    },
];

/// The key → placement index, built once on first use.
///
/// ## Rust Learning: `OnceLock<T>`
///
/// A `OnceLock` is a cell that is empty until something puts a value in it, and
/// is then immutable and shared forever. `get_or_init` runs the closure exactly
/// once even if several threads race into it — the losers block and then read
/// the winner's value. It is the modern, std-only replacement for the old
/// `lazy_static!` macro, and it is the right shape here because the map is
/// derived purely from `const` data: computing it at every request would be 860
/// pointless string hashes, and computing it at boot would make an ordinary
/// lookup depend on startup order.
fn index() -> &'static HashMap<&'static str, Placement> {
    static INDEX: OnceLock<HashMap<&'static str, Placement>> = OnceLock::new();
    INDEX.get_or_init(|| {
        let mut map = HashMap::new();
        for area in AREAS {
            for block in area.blocks {
                for key in block.keys {
                    // Last writer would win silently on a duplicate. It cannot
                    // happen — `settings_map_tests` fails the build if a key is
                    // claimed twice — and the test exists BECAUSE this insert
                    // cannot report it.
                    map.insert(
                        *key,
                        Placement {
                            area_id: area.id,
                            block_id: block.id,
                        },
                    );
                }
            }
        }
        map
    })
}

/// Where a stored key belongs.
///
/// A key no block claims resolves to the undeclared area rather than to `None`:
/// every stored row must reach the page, and one that reached nothing would be a
/// row a human could neither find nor fix. The honest answer is a placement that
/// says out loud that nothing declares it.
pub fn locate(key: &str) -> Placement {
    index().get(key).copied().unwrap_or(Placement {
        area_id: UNDECLARED_AREA_ID,
        block_id: UNDECLARED_BLOCK_ID,
    })
}

/// Every key any block declares — the union, for tests and for callers that need
/// to ask what this build knows about.
pub fn declared_keys() -> impl Iterator<Item = &'static str> {
    index().keys().copied()
}

#[cfg(test)]
#[path = "settings_map_tests.rs"]
mod tests;
