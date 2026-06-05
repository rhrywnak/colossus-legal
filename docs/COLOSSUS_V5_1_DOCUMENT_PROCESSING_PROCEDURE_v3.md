# COLOSSUS LEGAL — v5.1 Document Processing Procedure (v3)

**Date:** 2026-06-05 (v3)
**Status:** AUTHORITATIVE. **v3 corrects v2:** the discovery corroboration bar is now FIXED, not an open issue — the discovery pass-2 template is revised to `discovery_response_pass2_v5_2.md` and the schema CORROBORATES description is corrected in place (§2, §4, §10); the "do NOT process discovery" gate is lifted and replaced with a validation-reprocess gate (§10); deployment example updated for the v5_2 files (§6).
**v2 corrected v1:** deployment is manual scp (not Ansible, §6); profile-rename is now a directive (§4); brief/court_ruling are v4-only and NOT v5.1-ready (§9); added the complaint-PUBLISHED-first ordering + approved-only injection + no-same-case-filter operational notes; recorded the discovery validation result and the corroboration-bar caveat (§10).
**Earlier v1 status note follows.** AUTHORITATIVE. Supersedes the v4-era `DOCUMENT_TYPE_ONBOARDING_RUNBOOK_v1`
(scp deployment), the v5-era `COLOSSUS_DOC_PROCESSING_CHECKLIST` (May 10 naming/state), and
the v4 deployment steps in `EXTRACTION_TEMPLATE_CONSTRUCTION_GUIDE_v2`. Those remain valid for
*template authoring principles* but NOT for naming, file location, or deployment.
**Grounded in:** CC read-only inventory of the live, tested complaint v5.1 path on `main`
@ beta.327 (registry.rs, llm_extract.rs, llm_extract_pass2.rs, ingest.rs,
pipeline_registry.yaml). The complaint v5.1 set is the reference standard — every new type
matches it.

---

## 1. How the pipeline is wired (registry-driven)

Processing is driven by a **registry**, not by hardcoded filenames.

- `pipeline_registry.yaml` is read at startup from env `PIPELINE_REGISTRY_FILE`
  (registry.rs:244). It maps each `document_type` → a `profile_file`, and names four
  directories.
- The **profile** is the hub: it names the schema, pass-1 template, pass-2 template, system
  prompt, and (optional) global-rules file. Everything is reached *through the profile*.
- At startup, `registry.validate()` rejects the config if any referenced profile is missing,
  names collide, or `is_default` count ≠ 1.

**No template or schema filename is derived from `document_type` in code.** All five artifacts
are referenced by explicit, freely-versionable names in the profile YAML. This corrects the
earlier belief that a template "cannot be versioned" — see §4 for the one real naming
constraint (it's the profile, not a template).

---

## 2. The artifact set per document type (5 files + shared system prompt)

For each document type, authored under `/data/documents/`:

| Artifact | Example (complaint) | Located by | Versionable? |
|----------|--------------------|------------|--------------|
| Profile | `complaint_v5_1.yaml` | registry `document_type → profile_file` | see §4 |
| Schema | `complaint_schema_v5_1.yaml` | profile `schema_file` | yes (convention `<type>_schema_v5_1.yaml`) |
| Pass-1 template | `complaint_pass1_v5_1.md` | profile `template_file` | yes (`<type>_pass1_v5_1.md`) |
| Pass-2 template | `complaint_pass2_v5_1.md` | profile `pass2_template_file` | yes (`<type>_pass2_v5_1.md`) |
| System prompt | `legal_extraction_system.md` | profile `system_prompt_file` | **shared, version-neutral** (all 8 profiles point at this one file) |
| Global rules | (none for complaint) | profile `global_rules_file` | optional; if omitted, `{{global_rules}}` renders empty |

Directories (Ansible-managed — see §6):
```
/data/documents/profiles
/data/documents/extraction_schemas
/data/documents/extraction_templates
/data/documents/system_prompts      ← system prompt lives in its OWN dir, not with templates
```

---

## 3. Template placeholders — what's real, and the complaint vs. discovery divergence

The `<!-- AUTHORING_NOTE -->` block at the top of each template is stripped before the LLM
sees it. The real substitution sites are the body placeholders.

**Pass-1 (complaint) body order:** `{{schema_json}}` → `{{global_rules}}` →
`{{admin_instructions}}` → `{{context}}` → `{{document_text}}`

**Pass-2 (complaint) body order:** `{{entities_json}}` → `{{schema_json}}` →
`{{global_rules}}` → `{{document_text}}`
→ Pass-2 has **no** `{{admin_instructions}}` and **no** `{{context}}` site.

Runtime substitution:

| Placeholder | Substituted with |
|-------------|------------------|
| `{{schema_json}}` | serialized schema JSON |
| `{{global_rules}}` | global-rules file text, else `""` (complaint: empty) |
| `{{admin_instructions}}` | per-document admin instructions, else `""` |
| `{{context}}` | **always `None` → `""` in BOTH passes today** (cross-doc entities are inlined into `{{entities_json}}` instead) |
| `{{document_text}}` | full document text (full mode) or per-chunk `{{chunk_text}}` |
| `{{entities_json}}` (pass-2) | pass-1 local entities + cross-doc (`ctx:`-prefixed) + authored Tier-1, serialized |

**Two facts that matter for the other types:**

1. **Complaint pass-2 includes `{{document_text}}`. Discovery pass-2 deliberately omits it.**
   This is a real, intentional design divergence, not a bug — discovery pass-2 reasons over
   the pass-1 entities + cross-doc context rather than re-reading the full document. Any
   evaluation of discovery must respect that this was a choice.

2. **`{{context}}` is inert system-wide** (hardcoded empty). Cross-document linking happens
   through `{{entities_json}}`, not `{{context}}`. So a pass-2 template that says "skip if
   `{{context}}` is empty" would wrongly suppress cross-doc work — the cross-doc data is in
   `{{entities_json}}`. This is the discovery concern to verify, reframed correctly.

---

## 4. Naming — the ONE real constraint (it's the profile filename)

Schema and both templates follow a consistent convention across all types
(`<type>_schema_v5_1.yaml`, `<type>_pass1_v5_1.md`, `<type>_pass2_v5_1.md`). **The
inconsistency is the profile filename:**

| Role | complaint | discovery_response | affidavit |
|------|-----------|--------------------|-----------|
| Profile file | `complaint_v5_1.yaml` | `discovery_response.yaml` | `affidavit.yaml` |
| Scheme | `<type>_<version>` ✅ | bare `<type>` ⚠️ | bare `<type>` ⚠️ |

**Why it matters (code consequence):** the upload "profile version" override knob builds
`{document_type}_{version}` (upload.rs:79-87). For complaint that resolves to
`complaint_v5_1.yaml` ✅. For discovery/affidavit it would seek
`discovery_response_v5_1.yaml` / `affidavit_v5_1.yaml`, which don't exist → load error.
**Discovery and affidavit are reachable only via the registry-default path, not the explicit
version knob.** This is the concrete place the bare profile name "cannot be versioned."

**Enforced by code** (`registry.validate()`): every referenced profile must exist; names
unique; exactly one `is_default`; profile `name:` expected to match filename stem.
**Convention only:** the schema/template/system-prompt/global-rules filenames — nothing forces
the `_v5_1` pattern.

**DIRECTIVE (decided 2026-06-02):** all profiles use the complaint scheme — `<type>_v5_1.yaml`
— so the upload version knob resolves uniformly. The existing bare-named discovery and affidavit
profiles (`discovery_response.yaml`, `affidavit.yaml`) are to be **renamed** to
`discovery_response_v5_1.yaml` and `affidavit_v5_1.yaml`, with their registry `profile_file`
entries updated to match (renaming a profile requires updating the registry entry that points
at it, or `registry.validate()` will reject the config at startup). New types follow the same
scheme from the start.

---

## 5. Provenance is automatic — no template work needed

Ingest (ingest.rs:522-552) stamps every relationship uniformly with `source_document_id`,
`extraction_run_id` (`run-{run_id}`), and `created_at`, and sets node provenance. Templates
stay silent on per-edge provenance; the schema carries a `relationship_properties` block
citing data-model §5.4. **This is document-type-agnostic — any new type inherits it for free.**
So the v5.1 provenance requirement requires zero per-template work; it's handled at ingest.

---

## 6. Deployment — manual scp (the live path)

Config files (profile, schema, both templates, and — if changed — the shared system prompt)
are deployed to DEV by **manual file copy**, NOT by Ansible. Ansible automation for pipeline
config deployment remains an open gap (a known follow-up). The Dockerfile does not ship these
files; they are read from the DEV filesystem at runtime.

The deployment chain (Roman runs):

```bash
# 1. files are already in the repo (committed on the working branch)
# 2. repo -> dev-app1 tmp  (example: the v5_2 discovery files)
scp ~/Projects/colossus-legal/backend/extraction_templates/discovery_response_pass2_v5_2.md core@10.10.100.220:/tmp/
scp ~/Projects/colossus-legal/backend/extraction_schemas/discovery_response_schema_v5_1.yaml core@10.10.100.220:/tmp/
# 3. dev-app1 tmp -> live location
ssh core@10.10.100.220 "sudo cp /tmp/discovery_response_pass2_v5_2.md /mnt/data/legal-docs/extraction_templates/ && sudo cp /tmp/discovery_response_schema_v5_1.yaml /mnt/data/legal-docs/extraction_schemas/"
# 4. verify md5 matches the repo
md5sum ~/Projects/colossus-legal/backend/extraction_templates/discovery_response_pass2_v5_2.md
ssh core@10.10.100.220 "md5sum /mnt/data/legal-docs/extraction_templates/discovery_response_pass2_v5_2.md"
```

NOTE: the profile's `pass2_template_file` for `discovery_response` must be updated to name
`discovery_response_pass2_v5_2.md` (the template filename changed; the schema filename did not).
That profile edit + the file placement in the repo is a CC task.

**Live directory roots (confirmed):** templates and schemas under
`/mnt/data/legal-docs/extraction_templates/` and `/mnt/data/legal-docs/extraction_schemas/`;
profiles under the profiles dir; system prompts under their own dir. (CC's registry inventory
referenced `/data/documents/...` as the registry's configured layout — when in doubt, confirm
the live path with `ssh core@10.10.100.220 "ls <path>"` before copying, as the roots have
differed across notes.)

Templates/schemas/profiles are read at runtime per request — **no container restart, no rebuild**
for a config-file change. Code changes DO require rebuild + deploy via Semaphore.

## 7. The 8-step pipeline (one document, end to end)

| # | Step (progress) | Consumes | Does |
|---|-----------------|----------|------|
| 1 | extract_text (5→10%) | the PDF | reads PDF; OCR fallback (Surya/tesseract); writes `document_text`; auto-detects `document_type`, syncs `schema_file` |
| 2 | llm_extract_pass1 (10→60%) | pass-1 template, schema, system prompt, (global rules) | substitutes placeholders; LLM call (full or per-chunk); writes pass-1 entities |
| 3 | llm_extract_pass2 (60→70%) | pass-2 template, schema, pass-1 entities (+cross-doc +Tier-1) | requires pass-1 complete; builds `entities_json` (`{{context}}` empty); LLM call; writes relationships |
| 4 | verify (70→80%) | schema grounding modes + `document_text` | grounds each item; sets `grounding_status`; computes `grounding_pct` |
| 5 | auto_approve (80→82%) | profile `auto_approve_grounded` | auto-approves grounded entities; residue → Review tab |
| 6 | ingest (82→90%) | verified items | writes nodes + relationships to Neo4j; **stamps relationship provenance**; creates `DERIVED_FROM`; sets node provenance |
| 7 | index (90→95%) | Neo4j nodes | embeds + writes vectors to Qdrant |
| 8 | completeness (95→100%) | Neo4j graph | final checks (Document node present, expected nodes landed) |

Each step has operator-facing `recovery_hints` keyed by error substring
(pipeline_registry.yaml:146-230).

---

## 8. Create → deploy → process flow (current, accurate)

1. **Author** profile + schema + pass-1/pass-2 templates (+ optional global_rules), matching
   the complaint v5.1 set as the reference standard. System prompt is shared
   (`legal_extraction_system.md`) — do not author a new one unless deliberately diverging.
2. **Deploy** the files into `/data/documents/{profiles,extraction_schemas,
   extraction_templates,system_prompts}` via Ansible (confirm play/path per §6).
3. **Register**: add a `document_types:` entry (name → `profile_file`, `sort_order`) to
   `pipeline_registry.yaml`.
4. **Restart backend** — `registry.validate()` runs at startup and rejects the config if any
   referenced profile is missing, names collide, or `is_default` count ≠ 1.
5. **Upload** a document of that type; the 8-step pipeline runs; verify via Neo4j Browser +
   Review tab.

---

## 9. State of the three target types (from CC inventory, to scope against)

- **discovery_response** — schema + templates exist (v5.1). Open: the pass-2 `{{context}}`
  "skip if empty" wording (re-framed per §3.2 — verify it doesn't suppress cross-doc links
  carried in `{{entities_json}}`); confirm the deliberate no-`{{document_text}}` design is
  intended; then end-to-end test. Profile is bare-named (§4).
- **affidavit** — schema + templates substantially complete. Open: pass-2 missing
  `{{admin_instructions}}` site; pass-1 missing AUTHORING_NOTE; profile inconsistencies
  (bare name, `chunking_mode: full`, `is_default`); then end-to-end test.
- **summary_judgment** — does not exist. Net-new: all 5 artifacts + registry entry +
  `document_type` string. No existing material to start from.
- **brief, court_ruling** — only **v4** artifacts exist (`pass2_brief_v4.md`,
  `pass2_court_ruling_v4.md`, etc.). These are NOT v5.1-ready. A prior audit marked their
  pass-2 cross-doc wiring "correct" — that referred ONLY to keying off the entity list (not the
  dead `{{context}}`); it does NOT mean they are usable. They require full v5.1 schema + template
  + profile work before processing, like summary_judgment. Do not treat them as ready.
- **motion** — absorbed into brief (prior decision); no separate artifacts. NOTE: the registry
  still references a `motion` profile that has no YAML on disk — a known registry gap (the
  `every_registry_entry_has_a_matching_profile_yaml_on_disk` test failure).

Order to process: discovery_response first (highest evidence density / most cross-doc edges to
complaint allegations), affidavit next, summary_judgment last (net-new build).

---

## 10. Operational notes & validation status (v2)

**Processing order matters — complaint first.** Evidence-anchoring pass-2 pulls cross-document
entities (complaint Allegations, other docs' Evidence) only from documents already in status
PUBLISHED. So the complaint must be PUBLISHED before any discovery/affidavit/etc. is processed,
or that document's pass-2 sees zero complaint entities and produces no cross-doc links. If a
document is processed out of order, re-run its pass-2 after the prerequisite publishes.

**Approved-only injection.** Only `approved` allegations are injected as `ctx:` entries. A
complaint allegation that failed verbatim grounding and was never manually approved is excluded
(and is also absent from Neo4j) — consistent, but worth knowing when reconciling counts.

**No same-case filter.** The cross-doc injection unions across ALL published documents in the
database. Correct for the single-case (Awad) deployment; a multi-case instance would need a
case filter.

**Validation status (2026-06-02):** discovery_response v5.1 is validated end-to-end —
George Phillips discovery (77 cross-doc edges) and CFS interrogatory response (64 CORROBORATES,
2 CONTRADICTS) processed; evidence reaches Elements through CORROBORATES→Allegation→BEARS_ON;
the Proof Matrix renders supporting counts, coverage-derived status, and per-allegation evidence
with source-PDF click-through (beta.328).

**RESOLVED in v5_2 — corroboration bar raised (was the TOP open item).** v5.1's discovery pass-2
template created CORROBORATES for answers that merely *related to* an allegation, including
non-substantive responses — objections, privilege assertions, disclaimers of knowledge,
"documents speak for themselves" deflections — inflating supporting counts (e.g. Element 1.2
showed 101, padded with boilerplate non-answers; ~37% of the 141 edges were non-answers by the
pipeline's own classification). The fix is deployed in `discovery_response_pass2_v5_2.md` plus an
in-place correction to the `discovery_response_schema_v5_1.yaml` CORROBORATES description. The
bar is grounded in the Michigan Court Rules and case law: an objection is stated in lieu of an
answer (MCR 2.309(B)); an evasive or incomplete answer is treated as a failure to answer
(MCR 2.313(A)(4)); a discovery answer is an evidentiary (contestable) admission, not conclusive
proof (Radtke v Miller, 453 Mich 413). Under v5_2: only `admission`/`partial_admission` with a
quotable conceding span produce CORROBORATES; `evasive`/`objection`/`referral` produce no edge;
`denial` routes to CONTRADICTS/REBUTS only. The output JSON format is unchanged — semantics-only,
no parser/ingest code impact.

**VALIDATION GATE (replaces the old "do NOT process" gate).** Before processing further discovery
for proof, reprocess the existing George Phillips and CFS documents under v5_2 and confirm the
counts drop as predicted: the ~51 evasive/objection/referral edges should disappear, ~79
admission/firm-partial edges should survive, and the 5 hedged partials (statement_type=
partial_admission + evidence_strength=sworn_party_evasion) surface as borderline for human review.
Verify the survivors are genuine corroboration before trusting the new bar. NOTE: the non-answers
are still extracted as Evidence and remain in the graph — a pattern of evasion may itself be
evidence for Abuse of Process / bias, a deferred modeling question. See the corroboration-bar and
proof-verification design notes.

---

*End of procedure v3. This is the authoritative reference for v5.1 document processing.
Template authoring *principles* still come from EXTRACTION_TEMPLATE_CONSTRUCTION_GUIDE_v2;
naming, file location, deployment, and the runtime flow come from THIS document.*
