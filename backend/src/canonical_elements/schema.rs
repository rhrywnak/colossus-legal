//! Serde types for the canonical Element YAML schema.
//!
//! These mirror the structure of the four `count_N_*.yaml` files. The schema
//! is **generic** — it encodes any Michigan civil cause of action, not the
//! Awad case specifically. Case data lives entirely in the YAML; this code is
//! structural only (Standing Rule 2, reusability check).
//!
//! ## Rust Learning: `#[serde(deny_unknown_fields)]`
//!
//! By default serde silently ignores YAML keys that don't map to a struct
//! field. That is a silent-failure trap: a typo'd or stray key would be
//! dropped without complaint. `deny_unknown_fields` makes any unexpected key
//! a hard parse error instead, so the boundary between "valid schema" and
//! "author mistake" is code-enforced (Standing Rule 1). Every struct here
//! carries it.
//!
//! ## Rust Learning: `Option<T>` for optional fields
//!
//! A field of type `Option<T>` deserializes to `None` when the key is absent
//! *or* explicitly `null` in YAML. We use it for genuinely optional fields
//! (e.g. `statutory_anchor`, which is `null` for many Elements). Required
//! fields use the bare type, so a missing key is a parse error.

use serde::{Deserialize, Serialize};

/// One `count_N_*.yaml` file: a single `LegalCount` and everything attached
/// to it.
///
/// The count-specific sections (`breach_theories`, `improper_act_theories`,
/// `declarations_sought`) are top-level siblings of `count`/`elements`, and
/// only one of them is populated per file:
/// - Count I  → `breach_theories`
/// - Count IV → `improper_act_theories`
/// - Count III → `declarations_sought`
///
/// ## Rust Learning: `#[serde(default)]` on a `Vec`
///
/// With `default`, an absent section deserializes to an empty `Vec` rather
/// than a parse error. That keeps the schema forward-compatible: a Count that
/// has no theories simply omits the key.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CountFile {
    /// The `LegalCount` metadata (its properties are updated, not created —
    /// the node is expected to already exist in the graph).
    pub count: CountMetadata,

    /// The canonical Elements of this Count, in pleading order.
    pub elements: Vec<ElementDef>,

    /// Theories of *how* a breach Element was satisfied (Count I only).
    #[serde(default)]
    pub breach_theories: Vec<TheoryDef>,

    /// Theories of *how* the improper-act Element was satisfied (Count IV).
    #[serde(default)]
    pub improper_act_theories: Vec<TheoryDef>,

    /// Substantive declarations the Count asks the court to issue (Count III).
    #[serde(default)]
    pub declarations_sought: Vec<DeclarationDef>,
}

/// The `count:` block — metadata for the `LegalCount` node.
///
/// `controlling_authorities` and `doctrinal_requirements` are nested *inside*
/// this block in the YAML (not top-level). They are JSON-encoded onto the
/// `LegalCount` node rather than modeled as separate nodes — see the loader's
/// LegalCount update logic.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CountMetadata {
    /// Stable key used to MATCH the existing `LegalCount` node (1–4).
    pub count_number: u32,

    /// Human-readable name, used only for the change report header.
    /// The existing `LegalCount.title` is intentionally **not** overwritten.
    pub count_name: String,

    /// Template identifier, e.g. `"breach_of_fiduciary_duty_michigan"`.
    pub template_name: String,

    /// Standard of proof, e.g. `"preponderance"` / `"clear_and_convincing"`.
    pub burden_of_proof: String,

    /// Michigan Civil Jury Instruction reference. `null` where none applies
    /// (e.g. breach of fiduciary duty is governed by case law, not an M Civ JI).
    pub m_civ_ji_reference: Option<String>,

    /// Flags that a human (Chuck) must confirm a doctrinal choice. Defaults to
    /// `None` (not flagged); set `true` on Count II.
    pub chuck_review_required: Option<bool>,

    /// Explanation accompanying `chuck_review_required` (Count II only).
    pub chuck_review_note: Option<String>,

    /// Free-text structural note (Count III only — explains that its Elements
    /// are jurisdictional prerequisites, not tort elements). Persisted as the
    /// `special_note` property on `LegalCount` so no authored content is lost.
    pub special_note: Option<String>,

    /// Controlling cases/statutes/rules. JSON-encoded onto the LegalCount as
    /// `controlling_authorities_json`.
    pub controlling_authorities: Vec<AuthorityDef>,

    /// Doctrinal pleading requirements (Count IV only). JSON-encoded onto the
    /// LegalCount as `doctrinal_requirements_json`.
    #[serde(default)]
    pub doctrinal_requirements: Vec<DoctrinalRequirementDef>,
}

/// One controlling authority (case, statute, jury instruction, or court rule).
///
/// ## Rust Learning: `#[serde(skip_serializing_if = "Option::is_none")]`
///
/// This type is both *deserialized* from YAML and *serialized* back to a JSON
/// string for the `controlling_authorities_json` LegalCount property. The skip
/// attribute keeps that JSON clean: a `None` field is omitted rather than
/// emitted as `"court": null`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuthorityDef {
    /// Full citation string.
    pub citation: String,

    /// `"case" | "statute" | "jury_instruction" | "court_rule"`.
    pub authority_type: String,

    /// Issuing court, when applicable (statutes have none).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub court: Option<String>,

    /// Year of decision, when applicable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub year: Option<u32>,

    /// What role this authority plays for the Count.
    pub role: String,
}

/// A doctrinal pleading requirement (Count IV — abuse of process specificity,
/// improper-act-after-issuance, corroborating-act-beyond-motive).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DoctrinalRequirementDef {
    /// Requirement key, e.g. `"specificity"`.
    pub requirement: String,

    /// What the requirement demands.
    pub description: String,

    /// Whether the case as pleaded satisfies it.
    pub satisfied_in_awad: bool,

    /// Evidence supporting the satisfaction claim.
    pub satisfaction_evidence: String,
}

/// A canonical Element of a Count → an `Element` node.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ElementDef {
    /// Globally-unique stable id, e.g. `"element-1-1"`. MERGE key.
    pub id: String,

    /// 1-based order within the Count, for stable display.
    pub order_in_count: u32,

    /// Short canonical name of the Element.
    pub element_name: String,

    /// Display title (often equal to `element_name`).
    pub title: String,

    /// Which theory variant this Element belongs to, e.g. `"silent_fraud"`
    /// or `"common_law_fraud"` (Count II only; `None` elsewhere).
    pub theory_variant: Option<String>,

    /// What the plaintiff must prove for this Element.
    pub what_plaintiff_must_prove: String,

    /// Primary controlling authority citation (singular — distinct from the
    /// Count's plural `controlling_authorities` list).
    pub controlling_authority: String,

    /// Statutory anchor, where one applies (`null` for many Elements).
    pub statutory_anchor: Option<String>,

    /// Case-specific reasoning tying the Element to the pleaded facts.
    pub case_specific_notes: Option<String>,
}

/// A theory of *how* a multi-act Element was satisfied. Shared by Count I
/// breach theories and Count IV improper-act theories.
///
/// Domain note: these are theories of breach/impropriety, **not** separate
/// Elements of the cause of action. Michigan treats them as alternative ways
/// to satisfy one Element (`In re Conservatorship of Murray` for breach;
/// parallel reasoning under `Friedman v Dozorc` for abuse of process). They
/// become `BreachTheory` / `ImproperActTheory` nodes, keyed by `key`.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TheoryDef {
    /// Stable key within the Count, e.g. `"loyalty"`. MERGE key.
    pub key: String,

    /// What the theory means.
    pub definition: String,

    /// Statutory basis. Present for breach theories; absent (`None`) for
    /// improper-act theories.
    pub statutory_anchor: Option<String>,

    /// Worked examples from the case facts.
    pub awad_examples: String,
}

/// A substantive declaration the court is asked to issue (Count III) →
/// a `DeclarationSought` node, keyed by `id`.
///
/// Domain note: declarations are the *relief* sought, not Elements. An
/// `operative: false` declaration is preserved for historical traceability
/// (e.g. a theory pleaded against a since-dismissed defendant).
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeclarationDef {
    /// Stable id, e.g. `"declaration-3-a"`. MERGE key.
    pub id: String,

    /// The declaration text.
    pub declaration: String,

    /// Legal basis for the declaration.
    pub legal_basis: String,

    /// Whether the declaration is operative in the current case posture.
    pub operative: bool,

    /// Why a non-operative declaration was dropped (set only when
    /// `operative == false`).
    pub inoperative_reason: Option<String>,
}
