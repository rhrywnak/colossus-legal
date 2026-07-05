// =============================================================================
// backend/src/dto/scenario_crud.rs
// =============================================================================
//
// Wire DTOs for the scenario CRUD HTTP surface (authored-state store, task 1.1).
//
// Kept DELIBERATELY separate from the Neo4j `dto/scenario.rs` (which carries the
// graph-read fact/page shapes). These are the Postgres `scenarios` authored-state
// shapes — a saved *lens* over the case, not case content. Same domain word,
// different layer; two files so the two never blur.
//
// These are pure wire types — no dependency on the repository record type. The
// `ScenarioRecord` → `ScenarioDto` mapping lives in the handler module
// (`api/scenarios.rs`), the same place `claims.rs` keeps its `to_dto`, so the
// dto layer stays a leaf.
// =============================================================================

use serde::{Deserialize, Serialize};

use crate::domain::actor_role::ActorRole;

/// One scenario as the wire sees it.
///
/// Mirrors `ScenarioRecord` (the Postgres row) with two wire adaptations:
/// `scenario_id` is the row's `Uuid` rendered as a string, and
/// `anchor_allegation_ids` is flattened from the row's `Option<Vec<String>>` to a
/// plain `Vec<String>` (`None` → `[]`) so the client never has to distinguish
/// "null" from "empty" for a list it only ever iterates. `created_at` /
/// `updated_at` are intentionally omitted for this chunk — the form does not need
/// them.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScenarioDto {
    /// The database-minted `Uuid`, rendered as a string for the wire.
    pub scenario_id: String,
    pub name: String,
    pub direction: String,
    pub status: String,
    pub case_slug: String,
    pub feeds_count_id: Option<String>,
    /// Flattened from `Option<Vec<String>>` — `None`/absent both become `[]`.
    pub anchor_allegation_ids: Vec<String>,
    /// The authored definition body, stored and returned as opaque JSON (its
    /// shape is validated at render time, not here — see `scenario_store`).
    pub definition: serde_json::Value,
}

/// The create-scenario request body.
///
/// `name` and `direction` are required; everything else is optional with a
/// server-applied default (`status` → `"draft"`, `definition` → `{}`). Note that
/// `case_slug` is NOT here on purpose — the handler sources it from the URL path,
/// so a request can never write a scenario into a case other than the one its URL
/// names.
///
/// ## Rust Learning: `Option<T>` fields are optional without `#[serde(default)]`
///
/// serde already treats a missing key as `None` for an `Option<T>` field, so
/// these need no `#[serde(default)]`. `deny_unknown_fields` still rejects keys
/// the struct does NOT declare — a typo'd field fails loudly rather than being
/// silently ignored (Standing Rule 1).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScenarioCreateRequest {
    pub name: String,
    pub direction: String,
    /// Absent → the handler defaults to `"draft"`.
    pub status: Option<String>,
    pub feeds_count_id: Option<String>,
    pub anchor_allegation_ids: Option<Vec<String>>,
    /// Absent → the handler defaults to an empty JSON object `{}` (the column is
    /// NOT NULL; SQL null is never written).
    pub definition: Option<serde_json::Value>,
}

// ─── Typed definition body (the jsonb boundary) ─────────────────────────────

/// The definition schema version this build understands.
///
/// A stored definition read back with a `schema_v` GREATER than this was written
/// by a newer build that may have added keys this code cannot interpret. A reader
/// (B2's seed path) should compare against this constant and SURFACE the mismatch
/// — e.g. fall back to the case default — never silently coerce a newer body into
/// the shape it happens to parse today.
// CONST: the definition schema version this build understands — a build-time
// coupling invariant, NOT a deployment knob. It cannot vary per environment;
// changing it requires a simultaneous schema change and a matching migration /
// reader update (B2). So it cannot live in YAML/env (Standing Rule 2 does not
// apply — same rationale as `SCENARIO_COLUMNS`).
//
// v2 (D1): the authoring form was rebuilt. `seed_phrases`, `anti_seed_phrases`
// and `notes` were retired (Amendment 1 — never rendered, or superseded); `target`
// became a party node id chosen from live vocabulary (was free text); `wielders`
// became a list of `{party_id, actor_role}` (was a flat `Vec<String>`). A stored
// v1 body therefore no longer parses as this shape — which is intended: the reader
// treats a v1/`{}`/unparseable body as "not yet authored (v2)" and the form opens
// blank. There is deliberately NO migration (existing rows are `{}` bar disposable
// dev-test rows) — `schema_v` is the guard, not a data backfill.
pub const CURRENT_SCHEMA_V: u32 = 2;

/// One party named as a **wielder** of a scenario's attack, with the role it
/// plays in the accusation chain.
///
/// ## Rust Learning: a nested DTO validated by a domain enum
///
/// `party_id` is a graph node id chosen from the live bias vocabulary in the UI —
/// never free text (D1's core rule: a user can never name a party the graph does
/// not know). `actor_role` is an [`ActorRole`], so an unknown role token fails to
/// deserialize the whole `Wielder` (and thus the whole definition) at the parse
/// boundary — the loud failure Standing Rule 1 requires, rather than a silently
/// accepted junk role.
///
/// Like [`ScenarioDefinition`] this omits `#[serde(deny_unknown_fields)]` for
/// forward compatibility: a newer build that adds a key to a wielder entry must
/// still deserialize on older code.
///
/// Domain note: a scenario can list SEVERAL wielders — e.g. an accusation that CFS
/// originated, a judge later repeated, and a report adopted. The single-actor v1
/// model (`wielders[0]`) could not represent that chain; the list can.
// serde: allows unknown fields because of forward-compatibility with newer
// `schema_v` builds — a wielder entry written by a newer build that added a key
// must still deserialize on older code (same stance as `ScenarioDefinition`; the
// `actor_role` enum is still the loud gate for a bad role token).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Wielder {
    /// Graph party node id (from `available-filters` actors), never free text.
    pub party_id: String,
    /// This party's role in the accusation chain — validated against the versioned
    /// [`ActorRole`] lookup at parse time.
    pub actor_role: ActorRole,
}

/// A scenario's authored **definition** — the themed body that (in Phase B) seeds
/// its candidate-facts panel toward this scenario's attack.
///
/// Rebuilt in D1 (schema v2): the retired keys (`seed_phrases`,
/// `anti_seed_phrases`, `notes`) are gone; `target` is now a party node id and
/// `wielders` a list of [`Wielder`] entries. See [`CURRENT_SCHEMA_V`] for the
/// v1→v2 story.
///
/// ## Rust Learning: parse-don't-validate at the jsonb boundary
///
/// The storage column stays `jsonb` (a `serde_json::Value`) on purpose — the
/// authored shape still churns, so pinning it to typed SQL columns would force a
/// migration on every tweak. But an opaque `Value` flowing through the rest of
/// the code is a validation liability: a malformed definition would only blow up
/// deep at render time, far from where it entered. So we PARSE the `Value` into
/// this typed struct exactly once, at the API boundary, and pass the *typed*
/// value inward. "Parse, don't validate": once you hold a `ScenarioDefinition`,
/// every field is guaranteed present-or-defaulted — there is no lingering "is
/// this valid?" question downstream. On the write path the struct serializes
/// straight back into the same `Value` column, so storage is unchanged (this is
/// why B1 does NOT touch the migration, `ScenarioRecord.definition`, or
/// `insert_scenario`).
///
/// ## Forward compatibility: unknown keys are ALLOWED and IGNORED
///
/// This struct intentionally omits `#[serde(deny_unknown_fields)]` (unlike the
/// request DTOs below). A row written by a newer build — a higher `schema_v` that
/// added a key — must still deserialize on older code rather than hard-failing,
/// so an unrecognized key is silently dropped by serde, NOT rejected. `schema_v`
/// is the real gate for shape changes: a reader compares it against
/// [`CURRENT_SCHEMA_V`] and decides to surface "this definition is newer than I
/// understand" instead of trusting a partial parse. A *request* typo should fail
/// loudly (hence `deny_unknown_fields` on `ScenarioCreateRequest` /
/// `ScenarioUpdateRequest`); a *stored* forward-version row should not.
// serde: allows unknown fields because of forward-compatibility with newer
// `schema_v` builds — a stored row written by a newer build must still
// deserialize on older code; `schema_v` is the shape-change gate, not
// `deny_unknown_fields`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioDefinition {
    /// The attack this scenario answers, in the wielder's framing. REQUIRED — a
    /// definition with no attack has nothing to seed candidate facts from, so its
    /// absence must fail at this boundary (see [`ScenarioDefinition::from_value`]).
    pub attack_text: String,
    /// Plain-language gloss of what the attack actually asserts. Optional.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attack_meaning: Option<String>,
    /// Who the attack is ABOUT — a party node id from the live vocabulary
    /// (`available-filters` subjects), never free text. Optional (a definition may
    /// not yet name a target).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,
    /// Who makes or repeats this attack — a list of `{party_id, actor_role}`
    /// entries. Absent → `[]` (a definition may legitimately not name a wielder
    /// yet). See [`Wielder`].
    #[serde(default)]
    pub wielders: Vec<Wielder>,
    /// The definition schema version this body was authored under. REQUIRED so a
    /// reader can compare it against [`CURRENT_SCHEMA_V`] and surface a
    /// newer-than-understood (or older, now-retired) definition rather than
    /// silently mis-parsing it.
    pub schema_v: u32,
}

impl ScenarioDefinition {
    /// Serialize this typed definition into the opaque jsonb `Value` the
    /// `scenarios.definition` column stores. The write path (the `PUT` handler)
    /// calls this before handing the `Value` to `update_scenario`, keeping the
    /// store layer JSON-only and symmetric with `insert_scenario`.
    ///
    /// ## Rust Learning: why this returns `Result` and does not `unwrap`
    ///
    /// `serde_json::to_value` is fallible in general (a custom `Serialize` impl
    /// can error, e.g. a map with non-string keys). This all-scalar /
    /// `Vec<Wielder>` shape never actually triggers that — but we propagate the
    /// `Result` rather than `unwrap`, so that IF serialization ever failed it
    /// would be an observable error, not a panic (Standing Rule 1).
    ///
    /// # Errors
    /// Returns the `serde_json::Error` if serialization fails (does not occur for
    /// this shape in practice).
    pub fn to_value(&self) -> Result<serde_json::Value, serde_json::Error> {
        serde_json::to_value(self)
    }

    /// Parse an opaque column `Value` back into a typed definition — the LOUD
    /// boundary the read/seed path (B2) relies on.
    ///
    /// This is where a malformed definition fails FAST instead of at render time.
    /// In particular `{}` — the value on every un-authored scenario row today — is
    /// NOT a valid `ScenarioDefinition`: `attack_text` and `schema_v` are required
    /// (no `#[serde(default)]`), so `from_value(json!({}))` returns `Err`. That
    /// `Err` is the **"not yet defined"** state, not a bug: B2 treats a
    /// parse-`Err` / empty definition as "fall back to the case default subject,"
    /// so surfacing it as `Err` here is exactly the intended contract.
    ///
    /// # Errors
    /// Returns the `serde_json::Error` if `value` is missing a required key
    /// (`attack_text` / `schema_v`) or has a field of the wrong JSON type.
    pub fn from_value(value: serde_json::Value) -> Result<Self, serde_json::Error> {
        serde_json::from_value(value)
    }
}

/// The update-scenario request body (`PUT /cases/:slug/scenarios/:scenario_id`).
///
/// **Partial update:** every field is `Option<T>`, and an ABSENT field means
/// "leave this column unchanged." The store merges provided fields over the
/// existing row in SQL via `COALESCE` (see `update_scenario` in
/// `scenario_store.rs`).
///
/// Two deliberate omissions:
/// - `direction` is NOT updatable. A scenario's offense/defense stance is its
///   identity, not a mutable attribute — flipping it would make the scenario a
///   different thing — so it is set once at create and never here.
/// - `case_slug` is NOT here — same reason as [`ScenarioCreateRequest`]: the URL
///   path is the only source of the case, so a request can never move a scenario
///   between cases.
///
/// `definition` is the TYPED [`ScenarioDefinition`], not a raw `Value`: a
/// malformed definition body is rejected by the JSON extractor as a 400 before
/// the handler runs (the loud boundary). `deny_unknown_fields` still applies at
/// the request level — a typo'd top-level field fails loudly (Standing Rule 1).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScenarioUpdateRequest {
    /// Absent → name unchanged.
    pub name: Option<String>,
    /// Absent → status unchanged. Validated against the CHECK vocabulary in the
    /// handler when present, same as create.
    pub status: Option<String>,
    pub feeds_count_id: Option<String>,
    pub anchor_allegation_ids: Option<Vec<String>>,
    /// Absent → definition unchanged. Present → the ENTIRE definition blob is
    /// replaced (not deep-merged) with this typed body — see the whole-value
    /// replace note on `update_scenario`.
    pub definition: Option<ScenarioDefinition>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// Build a fully-populated v2 definition for the round-trip tests.
    fn full_definition() -> ScenarioDefinition {
        ScenarioDefinition {
            attack_text: "Marie is obstructive and uncooperative".to_string(),
            attack_meaning: Some("paints her as refusing every reasonable request".to_string()),
            target: Some("person-marie-awad".to_string()),
            wielders: vec![
                Wielder {
                    party_id: "org-cfs".to_string(),
                    actor_role: ActorRole::Originated,
                },
                Wielder {
                    party_id: "person-judge-tighe".to_string(),
                    actor_role: ActorRole::Repeated,
                },
            ],
            schema_v: CURRENT_SCHEMA_V,
        }
    }

    // ── Write-path conversion: ScenarioDefinition -> Value ──────────────

    #[test]
    fn to_value_produces_expected_json_shape() {
        // A partly-empty definition: only the required fields plus one wielder.
        let def = ScenarioDefinition {
            attack_text: "Marie hired too many attorneys".to_string(),
            attack_meaning: None,
            target: None,
            wielders: vec![Wielder {
                party_id: "person-george-phillips".to_string(),
                actor_role: ActorRole::Originated,
            }],
            schema_v: CURRENT_SCHEMA_V,
        };
        let value = def.to_value().expect("serialize to value");

        assert_eq!(
            value["attack_text"],
            json!("Marie hired too many attorneys")
        );
        // Each wielder is an object with a party id + a snake_case role token.
        assert_eq!(
            value["wielders"],
            json!([{ "party_id": "person-george-phillips", "actor_role": "originated" }])
        );
        assert_eq!(value["schema_v"], json!(CURRENT_SCHEMA_V));
        // None optionals are OMITTED (skip_serializing_if), not written as null —
        // so "absent" and "null" stay distinct in the stored jsonb (Rule 1).
        assert!(
            value.get("attack_meaning").is_none(),
            "None optional is omitted"
        );
        assert!(value.get("target").is_none());
        // The retired v1 keys must never be emitted by a v2 write.
        assert!(value.get("seed_phrases").is_none(), "seed_phrases retired");
        assert!(
            value.get("anti_seed_phrases").is_none(),
            "anti_seed_phrases retired"
        );
        assert!(value.get("notes").is_none(), "notes retired");
    }

    // ── Read-path conversion: Value -> ScenarioDefinition ───────────────

    #[test]
    fn round_trips_through_value() {
        let def = full_definition();
        let value = def.to_value().expect("serialize");
        let parsed = ScenarioDefinition::from_value(value).expect("parse back");

        assert_eq!(parsed.attack_text, "Marie is obstructive and uncooperative");
        assert_eq!(
            parsed.attack_meaning.as_deref(),
            Some("paints her as refusing every reasonable request")
        );
        assert_eq!(parsed.target.as_deref(), Some("person-marie-awad"));
        assert_eq!(
            parsed.wielders,
            vec![
                Wielder {
                    party_id: "org-cfs".to_string(),
                    actor_role: ActorRole::Originated,
                },
                Wielder {
                    party_id: "person-judge-tighe".to_string(),
                    actor_role: ActorRole::Repeated,
                },
            ]
        );
        assert_eq!(parsed.schema_v, CURRENT_SCHEMA_V);
    }

    #[test]
    fn from_value_defaults_absent_wielders_to_empty() {
        // A minimal valid definition: only the two required keys. `wielders` is
        // absent and must default to [] via #[serde(default)], not error.
        let value =
            json!({ "attack_text": "The $50,000 was a gift", "schema_v": CURRENT_SCHEMA_V });
        let parsed = ScenarioDefinition::from_value(value).expect("minimal definition parses");

        assert_eq!(parsed.attack_text, "The $50,000 was a gift");
        assert!(parsed.wielders.is_empty());
        assert!(parsed.target.is_none());
        assert_eq!(parsed.schema_v, CURRENT_SCHEMA_V);
    }

    #[test]
    fn from_value_rejects_unknown_actor_role() {
        // Standing Rule 1 at the definition boundary: a wielder carrying a role
        // this build does not define fails the WHOLE parse — a junk role can never
        // be silently stored. (The v1→v2 rebuild's core promise: no junk parties,
        // no junk roles.)
        let value = json!({
            "attack_text": "Sanctions were selectively pursued",
            "schema_v": CURRENT_SCHEMA_V,
            "wielders": [{ "party_id": "org-cfs", "actor_role": "invented_role" }]
        });
        assert!(
            ScenarioDefinition::from_value(value).is_err(),
            "a wielder with an unknown actor_role must not parse"
        );
    }

    #[test]
    fn from_value_rejects_v1_wielder_shape() {
        // A stored v1 body has `wielders` as bare strings. Under v2 that shape can
        // no longer parse (a string is not a {party_id, actor_role} object) — which
        // is the intended guard: the reader treats a v1 body as "not yet authored
        // under v2" and the form opens blank, rather than mis-reading old data.
        let value = json!({
            "attack_text": "Marie is obstructive",
            "schema_v": 1,
            "wielders": ["CFS", "George Phillips"]
        });
        assert!(
            ScenarioDefinition::from_value(value).is_err(),
            "a v1 flat-string wielders list must not parse as v2"
        );
    }

    #[test]
    fn from_value_rejects_empty_object() {
        // `{}` is the value on every un-authored scenario row today. It is NOT a
        // valid definition (attack_text + schema_v are required) — it is the
        // "not yet defined" state. B2 relies on this Err to fall back to the case
        // default, so Err here is intended, not a bug.
        assert!(
            ScenarioDefinition::from_value(json!({})).is_err(),
            "empty object must not parse as a definition"
        );
    }

    #[test]
    fn from_value_rejects_missing_attack_text() {
        // attack_text is the one field with no default — a definition without it
        // has nothing to seed from, so it must fail loudly at this boundary.
        let value = json!({ "schema_v": CURRENT_SCHEMA_V });
        assert!(
            ScenarioDefinition::from_value(value).is_err(),
            "a definition missing attack_text must not parse"
        );
    }

    #[test]
    fn from_value_rejects_missing_schema_v() {
        // schema_v is required too — a body with no version cannot be checked
        // against CURRENT_SCHEMA_V, so it is rejected rather than assumed current.
        let value = json!({ "attack_text": "Bias — who gained?" });
        assert!(
            ScenarioDefinition::from_value(value).is_err(),
            "a definition missing schema_v must not parse"
        );
    }

    #[test]
    fn from_value_ignores_unknown_keys_forward_compat() {
        // Forward compatibility: a row written by a newer schema_v that added a
        // key must still deserialize on older code — the unknown key is dropped,
        // not rejected (no deny_unknown_fields on ScenarioDefinition). schema_v is
        // preserved so a reader can detect the newer version.
        let value = json!({
            "attack_text": "Sanctions were never selectively pursued",
            "schema_v": 2,
            "future_key_from_a_newer_build": ["anything"]
        });
        let parsed =
            ScenarioDefinition::from_value(value).expect("unknown keys are ignored, not rejected");

        assert_eq!(
            parsed.attack_text,
            "Sanctions were never selectively pursued"
        );
        assert_eq!(parsed.schema_v, 2);
    }
}
