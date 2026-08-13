// =============================================================================
// backend/src/domain/wording_war_room.rs — the words the WAR ROOM speaks
// =============================================================================
//
// The eleventh stored-string block (task 396, P3b). It carries the Trial Prep
// dashboard's own sentences — the subtitle under the heading and the three metric
// tiles — and nothing else.
//
// ## Why this block is arriving late, and what it is fixing
//
// Ruling R2 of 2026-08-10 (task R2 §3) renamed the dashboard's subtitle, renamed
// the "Drafted / in review" tile to "Draft", and killed the "pattern analysis
// pending" chip, on the stated grounds that "every one of these is a row value,
// so Roman can retune any of them later with zero builds". The R2 batch shipped
// nine `scenario_identity_*` rows and the scan model row; NONE of §3 was
// migrated. Measured on DEV 2026-08-13: no war-room row exists in `app_settings`
// at all, and both sentences are still compiled-in literals in
// `TrialPrepDashboardPage.tsx` and `trialPrepHelpers.ts`. So the rows were never
// created, not created-and-bypassed — and this block is them.
//
// ## Domain note: the subtitle's rename is a correction of a claim, not a style
//
// "System-generated cross-examination scenarios" said the machine produced them.
// It did not: a human writes the attack, the scan gathers candidates, a human
// rules every one. A subtitle that credits the system for a human's judgment is
// the same honesty defect as an unlabelled placeholder number, one line higher up
// the page.

/// The stored strings the Trial Prep dashboard renders.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WarRoomWording {
    /// The sentence under the "Trial Prep — War Room" heading, saying who built
    /// what.
    pub subtitle: String,
    /// The label on the total-scenarios tile.
    pub metric_scenarios_label: String,
    /// The label on the ready-scenarios tile.
    pub metric_ready_label: String,
    /// The label on the not-yet-ready tile.
    ///
    /// Domain note: R2 shortened this from "Drafted / in review" because the tile
    /// counts ONE thing — scenarios that are not Ready — and a slashed pair of
    /// words invites a reader to look for two numbers in one figure.
    pub metric_draft_label: String,
}

// KEYS: the stable identifiers. Renaming one is a migration, and until it runs
// the boot loader refuses to start.
pub(crate) const KEY_SUBTITLE: &str = "war_room_subtitle";
pub(crate) const KEY_METRIC_SCENARIOS: &str = "war_room_metric_scenarios_label";
pub(crate) const KEY_METRIC_READY: &str = "war_room_metric_ready_label";
pub(crate) const KEY_METRIC_DRAFT: &str = "war_room_metric_draft_label";

/// Every war-room key this build reads, so a missing one is caught at boot BY
/// NAME rather than as an unlabelled tile.
pub const WAR_ROOM_WORDING_KEYS: &[&str] = &[
    KEY_SUBTITLE,
    KEY_METRIC_SCENARIOS,
    KEY_METRIC_READY,
    KEY_METRIC_DRAFT,
];

/// Build a [`WarRoomWording`] from the stored rows, or say which key is wrong.
///
/// Same generic-closure shape as the ten sibling builders — see
/// [`crate::domain::wording_model_params::build_model_params_wording`].
///
/// # Errors
/// Returns whatever `read` returns for the first key that is missing, of the
/// wrong declared kind, or blank.
pub fn build_war_room_wording<E>(
    read: impl Fn(&str) -> Result<String, E>,
) -> Result<WarRoomWording, E> {
    Ok(WarRoomWording {
        subtitle: read(KEY_SUBTITLE)?,
        metric_scenarios_label: read(KEY_METRIC_SCENARIOS)?,
        metric_ready_label: read(KEY_METRIC_READY)?,
        metric_draft_label: read(KEY_METRIC_DRAFT)?,
    })
}

#[cfg(test)]
#[path = "wording_war_room_tests.rs"]
pub(crate) mod seed_tests;

#[cfg(test)]
mod tests {
    use super::*;

    fn echo(key: &str) -> Result<String, std::convert::Infallible> {
        Ok(key.to_string())
    }

    /// Every field reads the key it claims to — the three tile labels especially,
    /// because they are three short strings of the same shape and a copy-paste
    /// between them would put one tile's word on another with nothing failing.
    #[test]
    fn every_field_reads_its_own_key() {
        let w = build_war_room_wording(echo).expect("infallible read");
        assert_eq!(w.subtitle, KEY_SUBTITLE);
        assert_eq!(w.metric_scenarios_label, KEY_METRIC_SCENARIOS);
        assert_eq!(w.metric_ready_label, KEY_METRIC_READY);
        assert_eq!(w.metric_draft_label, KEY_METRIC_DRAFT);
    }

    /// A key the builder reads but the list omits would be missing from
    /// `REQUIRED_KEYS`, so a blank tile would reach the screen instead of a named
    /// boot refusal.
    #[test]
    fn the_key_list_covers_every_field() {
        let w = build_war_room_wording(echo).expect("infallible read");
        let read_keys = [
            w.subtitle,
            w.metric_scenarios_label,
            w.metric_ready_label,
            w.metric_draft_label,
        ];
        assert_eq!(read_keys.len(), WAR_ROOM_WORDING_KEYS.len());
        for key in read_keys {
            assert!(
                WAR_ROOM_WORDING_KEYS.contains(&key.as_str()),
                "{key} is read by the builder but missing from WAR_ROOM_WORDING_KEYS",
            );
        }
    }

    #[test]
    fn the_keys_are_distinct() {
        let mut sorted = WAR_ROOM_WORDING_KEYS.to_vec();
        sorted.sort_unstable();
        let count = sorted.len();
        sorted.dedup();
        assert_eq!(sorted.len(), count, "two war-room wording keys collide");
    }
}
