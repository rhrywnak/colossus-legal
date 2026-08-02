// =============================================================================
// backend/src/domain/confidence_band.rs — the one seam for banding a score
// =============================================================================
//
// The July card read "contradicts · 70%". The stance half is fixed by
// `card_language`; this fixes the other half. A raw percentage invites a human to
// treat a model's self-report as a measurement — 70% of what? — so the card shows
// a BAND, and the raw score never crosses the wire.
//
// ## Where the cutoffs come from — task 1.6 landed
//
// They are STORED, not compiled. `band_for_score` takes a `&Settings` — the
// snapshot loaded from `app_settings` at boot and re-loaded whenever a human
// edits a parameter — and reads the two cutoffs from it. Changing where the High
// band starts is a Settings-page edit, not a rebuild (v2 §2b).
//
// Until 1.6, two `const`s lived here behind this same function, marked
// `TODO(1.6)`. That seam is what made this task a one-line change in this file
// instead of a hunt through the payload builder; the seam has now been repointed
// and the constants are gone.
//
// ## Why a `&Settings` parameter and not a global
//
// This function is called from `build_card`, which the §7 completeness test
// asserts against and whose doc comment says "Pure — no I/O". Reading a global
// here would make that claim false, and a unit test with no boot would find the
// global empty — leaving only a panic or a compiled-in fallback, which is the
// exact defect §2b bans. Taking the snapshot as an argument keeps the function
// pure and every branch testable with an ordinary struct literal.

use serde::{Deserialize, Serialize};

use super::settings::Settings;

/// How confident the scan reported itself to be, as a band.
///
/// ## Domain note: why NOT strong / moderate / weak
///
/// Those three are the RETIRED element-verdict vocabulary (canon §9). Reusing
/// them here would put one word on two meanings — "strong" would mean both "this
/// element is well supported" and "the model was fairly sure" — which is the
/// precise failure the naming canon exists to prevent. High/medium/low says only
/// what it means: a statement about the MODEL, never about the evidence.
///
/// ## Rust Learning: `#[serde(rename_all = "snake_case")]` on a closed enum
///
/// Each variant maps to its snake_case wire token. The enum is closed, so a band
/// this build does not define cannot be deserialized into it — the same
/// loud-boundary discipline as the sibling vocabularies in this module family.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConfidenceBand {
    /// At or above the high cutoff.
    High,
    /// At or above the medium cutoff, below the high one.
    Medium,
    /// Below the medium cutoff.
    Low,
    /// No model ever scored this item.
    ///
    /// ## Domain note: unscored is NOT low
    ///
    /// A hand-curated candidate, or one no scan has reached, has no score at all.
    /// Rendering that as "low confidence" would be the system inventing a
    /// judgment it never made — indistinguishable on the card from a model that
    /// looked and was unconvinced. Standing Rule 1: different states, different
    /// observables.
    Unscored,
}

impl ConfidenceBand {
    /// The full vocabulary — the "extensible list in code."
    pub const ALL: &'static [ConfidenceBand] = &[
        ConfidenceBand::High,
        ConfidenceBand::Medium,
        ConfidenceBand::Low,
        ConfidenceBand::Unscored,
    ];

    /// The stable wire token (matches the serde `snake_case` name).
    pub fn code(self) -> &'static str {
        match self {
            ConfidenceBand::High => "high",
            ConfidenceBand::Medium => "medium",
            ConfidenceBand::Low => "low",
            ConfidenceBand::Unscored => "unscored",
        }
    }

    /// The plain-trial label the card shows.
    ///
    /// Each says explicitly that it describes the SCAN, not the evidence — a card
    /// reading a bare "High" would be read as a claim about the fact.
    pub fn label(self) -> &'static str {
        match self {
            ConfidenceBand::High => "Scan was highly confident",
            ConfidenceBand::Medium => "Scan was fairly confident",
            ConfidenceBand::Low => "Scan was not very confident",
            ConfidenceBand::Unscored => "Not scored by a scan",
        }
    }
}

/// THE SEAM. Band a raw score, or report that there is no score.
///
/// This is the only function in the product that turns a number into a band, and
/// the only reader of the two stored cutoffs.
///
/// ## Rust Learning: matching on `Option<f32>` before comparing
///
/// `None` is handled first and separately, so an absent score can never fall
/// through a numeric comparison. Note that `f32` has no total ordering (`NaN` is
/// not comparable), which is exactly why this is a chain of `>=` tests rather
/// than a `match` on ranges: a `NaN` score fails every comparison and lands in
/// `Low`, which is the conservative direction — it can never be promoted to
/// `High` by a malformed value.
pub fn band_for_score(score: Option<f32>, settings: &Settings) -> ConfidenceBand {
    let Some(score) = score else {
        return ConfidenceBand::Unscored;
    };
    if score >= settings.confidence_band_high {
        ConfidenceBand::High
    } else if score >= settings.confidence_band_medium {
        ConfidenceBand::Medium
    } else {
        ConfidenceBand::Low
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The seeded snapshot: high 0.80, medium 0.50 — the values the migration
    /// stores, and the ones these cutoff assertions are written against.
    fn settings() -> Settings {
        Settings::for_test()
    }

    #[test]
    fn an_absent_score_is_unscored_never_low() {
        // The distinction that keeps the card honest: nobody looked, versus a
        // model looked and was unconvinced.
        assert_eq!(band_for_score(None, &settings()), ConfidenceBand::Unscored);
        assert_ne!(band_for_score(None, &settings()), ConfidenceBand::Low);
    }

    #[test]
    fn the_cutoffs_are_inclusive_at_their_lower_edge() {
        // Pinned so a later `>` / `>=` slip is visible: a score sitting exactly on
        // a cutoff belongs to the HIGHER band.
        assert_eq!(
            band_for_score(Some(0.80), &settings()),
            ConfidenceBand::High
        );
        assert_eq!(
            band_for_score(Some(0.50), &settings()),
            ConfidenceBand::Medium
        );
    }

    #[test]
    fn scores_band_across_the_range() {
        assert_eq!(band_for_score(Some(1.0), &settings()), ConfidenceBand::High);
        assert_eq!(
            band_for_score(Some(0.95), &settings()),
            ConfidenceBand::High
        );
        assert_eq!(
            band_for_score(Some(0.79), &settings()),
            ConfidenceBand::Medium
        );
        assert_eq!(
            band_for_score(Some(0.51), &settings()),
            ConfidenceBand::Medium
        );
        assert_eq!(band_for_score(Some(0.49), &settings()), ConfidenceBand::Low);
        assert_eq!(band_for_score(Some(0.0), &settings()), ConfidenceBand::Low);
    }

    #[test]
    fn the_july_card_score_no_longer_renders_as_a_percentage() {
        // C-222 showed "70%". The same score now produces a band and a sentence
        // about the SCAN — the number never reaches the card.
        let band = band_for_score(Some(0.70), &settings());
        assert_eq!(band, ConfidenceBand::Medium);
        assert!(!band.label().contains('%'));
        assert!(!band.label().contains("70"));
    }

    #[test]
    fn a_malformed_score_cannot_be_promoted() {
        // NaN fails every `>=`, so it lands in Low. The conservative direction:
        // a broken score must never present as high confidence.
        assert_eq!(
            band_for_score(Some(f32::NAN), &settings()),
            ConfidenceBand::Low
        );
    }

    #[test]
    fn out_of_range_scores_still_band_without_panicking() {
        // The model is supposed to emit [0.0, 1.0]; nothing enforces it upstream.
        // Banding must not panic or invent a fifth state.
        assert_eq!(band_for_score(Some(1.5), &settings()), ConfidenceBand::High);
        assert_eq!(band_for_score(Some(-0.2), &settings()), ConfidenceBand::Low);
    }

    #[test]
    fn band_tokens_match_serde() {
        for &band in ConfidenceBand::ALL {
            assert_eq!(serde_json::json!(band), serde_json::json!(band.code()));
        }
    }

    #[test]
    fn no_band_label_uses_retired_verdict_vocabulary() {
        // strong / weak / moderate are the retired element-verdict words. Reusing
        // them for confidence would put one word on two meanings.
        for &band in ConfidenceBand::ALL {
            let lowered = band.label().to_lowercase();
            for banned in ["strong", "weak", "moderate"] {
                assert!(
                    !lowered.contains(banned),
                    "band label {:?} reuses the retired verdict word {banned:?}",
                    band.label()
                );
            }
        }
    }

    #[test]
    fn every_label_says_it_is_about_the_scan() {
        // A bare "High" would read as a claim about the EVIDENCE. Each label names
        // its subject so the human cannot mistake whose confidence it is.
        for &band in ConfidenceBand::ALL {
            assert!(
                band.label().contains("Scan") || band.label().contains("scan"),
                "band label {:?} does not say whose confidence it reports",
                band.label()
            );
        }
    }
}
