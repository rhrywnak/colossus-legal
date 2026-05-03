# Pipeline Configuration Audit — Gap Report
**Date:** 2026-05-02
**Scope:** Pass-1 + Pass-2 + Config Panel + JSONB audit logging + profile resolution
**Branch:** feature/intelligent-chunking
**Author:** Claude Code (read-only audit, no code modified)

---

## 1. Profile YAML Field Inventory

Every field declared in `ProcessingProfile` (`backend/src/pipeline/config.rs:27-134`)
or accepted by its YAML loader is enumerated below. "Loaded?" means the YAML
deserializer reads it. "Resolvable Override?" means
`get_pipeline_config_overrides` (`backend/src/repositories/pipeline_repository/mod.rs:486`)
can carry a per-document override and `resolve_config` (`config.rs:332`) honors
it. "UI Exposed?" means the Configuration Panel
(`frontend/src/components/pipeline/ConfigurationPanel.tsx`) renders an
editable widget for the field. "Logged in JSONB?" means the field is part of
`ResolvedConfig` (`config.rs:200-248`) and therefore survives into the
`extraction_runs.processing_config` snapshot via
`write_processing_config_snapshot` (`pipeline/steps/llm_extract.rs:1497-1532`).

| Field | Loaded? | Resolvable Override? | UI Exposed? | Logged in JSONB? | Notes |
|---|---|---|---|---|---|
| `name` | yes — `config.rs:29` | yes — written as `profile_name` (`config.rs:422`) via override `profile_name` (`mod.rs:464,503`) | yes — Profile dropdown (`ConfigurationPanel.tsx:609-619`) | yes — `ResolvedConfig.profile_name` (`config.rs:202`) serialized | No `profile_hash` or `profile_content` ever computed → see Gap 4. |
| `display_name` | yes — `config.rs:30` | no — never copied into overrides | no | no | Display-only, not part of audit. |
| `description` | yes — `config.rs:32` | no | no | no | Display-only. |
| `schema_file` | yes — `config.rs:35` | **NO** — no `pipeline_config.schema_file` override column for the *profile-level* file (the `pipeline_config.schema_file` column exists but is base, not override; `patch_pipeline_config_overrides` does not write it; `PatchConfigInput::From` drops `schema_file`; see `config_handler.rs:58-60`) | partially — the panel renders a Schema dropdown (`ConfigurationPanel.tsx:653-667`) but the PATCH builder explicitly skips it (`ConfigurationPanel.tsx:507-510` "schema_file is not yet a pipeline_config override column — skip it silently"). The user-visible widget is silently inert on save. | yes — `ResolvedConfig.schema_file` (`config.rs:228`) written to JSONB | UI claim ≠ persistence — Gap 7. |
| `template_file` | yes — `config.rs:38` | yes — column `template_file` (`mod.rs:467,506`) | yes — Template dropdown (`ConfigurationPanel.tsx:641-651`) | yes — `ResolvedConfig.template_file` + `template_hash` (`config.rs:208-209,1505`) |  |
| `system_prompt_file` | yes — `config.rs:40` | yes — column `system_prompt_file` (`mod.rs:468,507`) | **NO** — interface field omitted from `Overrides` (`ConfigurationPanel.tsx:182-195`); no JSX widget; the `diffConfigFromProfile` helper explicitly notes "no UI surface yet" (`ConfigurationPanel.tsx:215-218`) | yes — `ResolvedConfig.system_prompt_file` + `system_prompt_hash` (`config.rs:216-221,1506`) | Operator cannot change which system prompt is used; Gap 8. |
| `global_rules_file` | yes — `config.rs:55` | **NO** — no override column, no override field on `PipelineConfigOverrides` (`config.rs:255-291`), comment states "Global rules are profile-level only — no per-document override path" (`config.rs:431-433`) | **NO** | yes — `ResolvedConfig.global_rules_file` (`config.rs:227`) — but **content hash is NOT computed** (compare to system_prompt_hash + template_hash). Substituted into prompt at `llm_extract.rs:330-341`. | No `global_rules_hash` → audit cannot prove which version of `global_rules_v4.md` ran. Gap 5. |
| `pass2_template_file` | yes — `config.rs:66` | **NO** — not in `PipelineConfigOverrides`, not in `PatchConfigInput`, comment at `config.rs:213-215` calls it "a profile-level authoring concern, not an operator knob" | **NO** — `ProcessingProfile` TS interface in `frontend/src/services/configApi.ts:65-84` does not declare the field at all; the JSON returned by `/profiles` carries it but the type elides it; no widget | yes — `ResolvedConfig.pass2_template_file` (`config.rs:215`) | Pass-2 template choice is invisible in UI. Gap 1. |
| `extraction_model` | yes — `config.rs:69` | yes — column `extraction_model` (`mod.rs:465,504`) | yes — Model dropdown (`ConfigurationPanel.tsx:625-635`) | yes — `ResolvedConfig.model` (`config.rs:203`) |  |
| `pass2_extraction_model` | yes — `config.rs:79` | yes — column `pass2_extraction_model` (`mod.rs:466,505`) | yes — Pass 2 Model dropdown when run_pass2 is checked (`ConfigurationPanel.tsx:740-752`) | yes — `ResolvedConfig.pass2_model` (`config.rs:206`) |  |
| `synthesis_model` | yes — `config.rs:81` | **NO** — not in `PipelineConfigOverrides`, not in DB | **NO** — TS type carries it (`configApi.ts:75`) but no widget | **NO** — not on `ResolvedConfig` at all | Field is loaded from YAML and discarded. Currently `null` in every shipped profile, but a value would silently vanish. Gap 9. |
| `chunking_mode` | yes — `config.rs:85` | yes — column `chunking_mode` (`mod.rs:469,508`) | yes — Chunking dropdown (`ConfigurationPanel.tsx:673-683`); legacy field, three modes (`full`/`structured`/`chunked`) | yes — `ResolvedConfig.chunking_mode` (`config.rs:229`) | Note: `chunking_config["mode"]` (newer) wins at runtime (`llm_extract.rs:1571-1580`); the UI-controlled legacy field can be a no-op. Gap 6. |
| `chunk_size` | yes — `config.rs:87` | yes — column `chunk_size` (`mod.rs:470,509`) | **NO widget** — declared in `Overrides` (`ConfigurationPanel.tsx:190`) but no `<input>` in JSX | yes — `ResolvedConfig.chunk_size` (`config.rs:230`) | Gap 8. |
| `chunk_overlap` | yes — `config.rs:89` | yes — column `chunk_overlap` (`mod.rs:471,510`) | **NO widget** — same as chunk_size | yes — `ResolvedConfig.chunk_overlap` (`config.rs:231`) | Gap 8. |
| `chunking_config` (whole map) | yes — `config.rs:110` (`HashMap<String,Value>`) | **NO** — `get_pipeline_config_overrides` hardcodes `chunking_config: None` (`mod.rs:518`); comment "pipeline_config columns for chunking_config/context_config arrive in Group 3's migration" | **NO** — TS `ProcessingProfile` does not declare it (`configApi.ts:65-84`); `PatchConfigInput` does not declare it (`configApi.ts:137-160`); upload handler hardcodes `chunking_config: None` (`upload.rs:447`) | yes — `ResolvedConfig.chunking_config` (`config.rs:237`) is serialized | This is the field whose **silent drop was discovered yesterday**. The fix in commit 5cb0b7d added `chunking_config` to brief and court_ruling profile YAMLs; it is now loaded and reaches the LLM, but **no operator can override it from the UI** and **no per-document override column exists**. Gap 1. |
| `chunking_config.mode` | (sub-key) | (no — see parent) | no | yes (as nested key) |  |
| `chunking_config.strategy` | (sub-key) | (no) | no | yes |  |
| `chunking_config.units_per_chunk` | (sub-key) | (no) | no | yes |  |
| `chunking_config.unit_overlap` | (sub-key) | (no) | no | yes |  |
| `chunking_config.request_timeout_secs` | (sub-key) | (no) | no | yes |  |
| `context_config` | yes — `config.rs:118` | **NO** — same hardcode as chunking_config (`mod.rs:519`) | **NO** | yes — `ResolvedConfig.context_config` (`config.rs:242`) | Same shape problem as chunking_config. Gap 1. |
| `max_tokens` | yes — `config.rs:122` | yes — column `max_tokens` (`mod.rs:472,511`) | yes — Max Tokens input (`ConfigurationPanel.tsx:689-696`) | yes — `ResolvedConfig.max_tokens` (`config.rs:243`) |  |
| `temperature` | yes — `config.rs:124` | yes — column `temperature` (`mod.rs:473,512`) | yes — Temperature input (`ConfigurationPanel.tsx:702-710`) | yes — `ResolvedConfig.temperature` (`config.rs:244`) |  |
| `auto_approve_grounded` | yes — `config.rs:128` | **NO** — not in `PipelineConfigOverrides` | **NO** — not in TS, not in panel | yes — `ResolvedConfig.auto_approve_grounded` (`config.rs:245`) — value is taken straight from profile (`config.rs:443`) | Gap 9. |
| `run_pass2` | yes — `config.rs:130` | yes — column `run_pass2` (`mod.rs:474,513`) | yes — checkbox (`ConfigurationPanel.tsx:716-725`) | yes — `ResolvedConfig.run_pass2` (`config.rs:246`) |  |
| `is_default` | yes — `config.rs:133` | no — fallback selector only (`config.rs:178-180`) | no | no | Affects `default.yaml` discovery, not per-doc behavior. |

---

## 2. Pass-2 Code Path

### Where it lives

`backend/src/pipeline/steps/llm_extract_pass2.rs` — single file, 550 lines.

Two layers:

- **FSM adapter** — `LlmExtractPass2` struct + `Step` impl
  (`llm_extract_pass2.rs:71-119`). Reached automatically from
  `next_step_after_pass1` (`llm_extract.rs:1424-1437`) when
  `resolved.run_pass2 == true`.
- **Free-function orchestrator** — `run_pass2_extraction`
  (`llm_extract_pass2.rs:137-472`). Called by both the FSM adapter and
  any direct API/CLI trigger.

### What it does (step-by-step)

1. Idempotency: short-circuit on existing COMPLETED `pass_number=2` row
   (`llm_extract_pass2.rs:144-150`, query at `pass2_already_complete`
   `llm_extract_pass2.rs:482-496`).
2. Loads `pipeline_config`, `documents`, `document_text` (full text, joined
   across pages) (`llm_extract_pass2.rs:152-187`).
3. Resolves profile + per-document overrides via the **same**
   `resolve_config` used by Pass 1 (`llm_extract_pass2.rs:189-198`).
4. Requires `resolved.pass2_template_file.is_some()` (`llm_extract_pass2.rs:201-205`).
5. Loads pass-1 entities from `extraction_items` for this document
   (`load_pass1_entities`, `llm_extract_pass2.rs:220`).
6. Loads cross-document context entities from previously-PUBLISHED docs
   (`load_cross_document_context`, `llm_extract_pass2.rs:278-279`).
7. Resolves model: `pass2_model` override → profile `pass2_extraction_model`
   → fall back to `model` (`llm_extract_pass2.rs:232-247`).
8. Loads `pass2_template_file` from `template_dir`, computes `template_hash`
   (`llm_extract_pass2.rs:252-255`).
9. Loads `system_prompt_file` if any, computes `system_prompt_hash`
   (`llm_extract_pass2.rs:257-269`).
10. Builds the entities list (local + cross-doc) and serializes to
    `entities_json` (`llm_extract_pass2.rs:291-296`).
11. Substitutes placeholders **with hardcoded literals**
    (`llm_extract_pass2.rs:316-320`):
    - `{{schema_json}}` ← `schema_json`
    - `{{entities_json}}` ← `entities_json`
    - `{{context}}` ← `""` (literal empty)
    - `{{document_text}}` ← `full_text`
    - **NOT substituted**: `{{global_rules}}`, `{{admin_instructions}}` —
      the comment at `llm_extract_pass2.rs:312-315` acknowledges this:
      "still intentionally unfilled — mirroring pass-1's current behavior;
      that gap is tracked separately." Pass 1 *does* substitute both as of
      commit 09d187c — Pass 2 is now stale relative to Pass 1.
12. Inserts a separate `extraction_runs` row with `pass_number = 2`
    (upsert keyed on `(document_id, pass_number)` —
    `extraction_runs_doc_pass_unique`, see migration
    `20260422113610_add_unique_constraint_on_extraction_runs_document_and_pass.sql`).
    Call site: `llm_extract_pass2.rs:332-356`.
13. LLM call via `call_with_rate_limit_retry` (shared helper)
    (`llm_extract_pass2.rs:386-397`).
14. Parse + store relationships only (`store_pass2_relationships`,
    `llm_extract_pass2.rs:411-416`).
15. Computes cost (`compute_cost`), calls `complete_extraction_run` with
    raw_output, input_tokens, output_tokens, cost_usd, status COMPLETED
    (`llm_extract_pass2.rs:418-435`).
16. Calls `write_processing_config_snapshot` (the same helper as Pass 1)
    against the pass-2 run row (`llm_extract_pass2.rs:438-445`).

### What it captures

In the JSONB `processing_config` column on the pass-2 row:
- All fields of `ResolvedConfig` (same as pass 1 — see Section 3 below).
- `template_hash` is overwritten via `resolved_with_hash.template_hash`
  before serialization (`llm_extract.rs:1505`) — for pass 2 this is the
  **pass-2 template's** hash, since the orchestrator passes the pass-2
  template hash into the helper (`llm_extract_pass2.rs:438-444`).
- `system_prompt_hash` is set from the pass-2 system prompt hash
  (same path).

In the dedicated columns of `extraction_runs` (per `insert_extraction_run`,
`extraction.rs:113-196`):
- `assembled_prompt` — the full pass-2 prompt is **passed in directly at
  insert time** (pass 2 has no per-chunk loop, so no follow-up UPDATE is
  required) — `llm_extract_pass2.rs:341`.
- `template_name = pass2_template_file` (`llm_extract_pass2.rs:342`).
- `template_hash = sha2_hex(pass2_template_text)` (`llm_extract_pass2.rs:343`).
- `model_name = pass2_model_id` (`llm_extract_pass2.rs:339` — comment
  explicitly names the bug it prevents: "Record the model actually used for
  pass 2, not the pass-1 model").
- `temperature`, `max_tokens_requested`, `admin_instructions`, schema
  content (passed through the same insert helper).
- `input_tokens`, `output_tokens`, `cost_usd`, `completed_at`, `status`,
  `raw_output` — set by `complete_extraction_run`.

### What it drops / does not capture

- **No `prior_context` value passed at insert** — pass 2 sets the
  `prior_context` column to `None` (`llm_extract_pass2.rs:351`). That
  column is the F3 audit slot for the cross-document context that pass 2
  loaded at step 6 above. The cross-doc entities **are inlined into
  `entities_json` and sent to the LLM**, but the audit log does not record
  *which* cross-doc entities were included. There is no way after the fact
  to know which prior documents informed a pass-2 run.
- **No `rules_name` / `rules_hash`** — the F3 reproducibility columns for
  the rules fragment (added by migration
  `20260410_f3_reproducibility_columns.sql`) exist but are passed `None`
  by both pass-1 and pass-2 (`llm_extract.rs:357-358`,
  `llm_extract_pass2.rs:344-345`). The rules content reaches the LLM
  for pass 1 but is never recorded.
- **No global_rules / admin_instructions in the assembled pass-2 prompt** —
  see step 11 above. So the pass-2 `assembled_prompt` column does not
  contain those rules, making it inconsistent with pass 1.
- **No `chunk_count` / `chunks_succeeded` / `chunks_failed`** — pass 2 is
  single-call, so these are NULL on the pass-2 row. That is correct.
- **No `chunk_metadata` / `extraction_chunks` rows** — pass 2 does not
  insert any per-chunk audit. Correct (single call).
- **Schema content is captured** — `schema_content = serde_json::to_value(&schema)`
  at `llm_extract_pass2.rs:347`. ✓

---

## 3. JSONB Audit Trail — `processing_config`

### The serialization site

`backend/src/pipeline/steps/llm_extract.rs:1497-1532` —
`write_processing_config_snapshot`. The function:

1. Clones the `ResolvedConfig`,
2. Overwrites `template_hash` and `system_prompt_hash` with values
   computed at runtime by the caller,
3. Serializes the cloned struct with `serde_json::to_value`,
4. `UPDATE extraction_runs SET processing_config = $1 WHERE id = $2`.

A serialization or DB failure is logged as `tracing::warn!` and **does
not fail the run** (`llm_extract.rs:1510-1531`). So a missing snapshot
is a silent gap, not a hard failure.

### Every field currently written to `extraction_runs.processing_config`

Defined by `ResolvedConfig` in `backend/src/pipeline/config.rs:200-248`:

1. `profile_name` — `String` (the resolved profile name)
2. `model` — pass-1 model id
3. `pass2_model` — `Option<String>` (pass-2 model id; `None` ⇒ falls back to `model`)
4. `template_file` — pass-1 template filename
5. `template_hash` — `Option<String>` (filled at runtime;
   pass 1 = SHA-256 of pass-1 template, pass 2 = SHA-256 of pass-2 template)
6. `pass2_template_file` — `Option<String>`
7. `system_prompt_file` — `Option<String>`
8. `system_prompt_hash` — `Option<String>`
9. `global_rules_file` — `Option<String>` ✱ (filename only — no hash)
10. `schema_file` — `String` (filename only — no hash, despite
    `schema_hash` column existing)
11. `chunking_mode` — `String` (legacy field)
12. `chunk_size` — `Option<i32>`
13. `chunk_overlap` — `Option<i32>`
14. `chunking_config` — `HashMap<String, serde_json::Value>` (the new map)
15. `context_config` — `HashMap<String, serde_json::Value>`
16. `max_tokens` — `i32`
17. `temperature` — `f64`
18. `auto_approve_grounded` — `bool`
19. `run_pass2` — `bool`
20. `overrides_applied` — `Vec<String>` (which fields came from the
    per-document override layer)

### Pass-1 fields currently present

All 20 fields above are written when Pass 1 completes
(`llm_extract.rs:537-545`).

### Pass-2 fields **absent that should be present** (or merely incomplete)

All 20 fields *are* written for Pass 2 because Pass 2 reuses
`write_processing_config_snapshot`. However, the snapshot does **not**
distinguish pass-1 metadata from pass-2 metadata in its shape:

- No `pass2_template_hash` field exists on `ResolvedConfig`. Pass 2's
  helper writes the **pass-2 template hash into the `template_hash`
  field** (`llm_extract_pass2.rs:438-445` calls
  `write_processing_config_snapshot(..., &template_hash, ...)` where
  `template_hash` is the pass-2 hash). So on a pass-2 run row,
  `processing_config.template_hash` = pass-2 hash; but
  `processing_config.template_file` = the **pass-1** template
  filename (because `ResolvedConfig.template_file` is still the pass-1
  field and is never replaced for pass-2). This is an internal
  inconsistency: the JSONB carries pass-1 `template_file` paired with
  pass-2 `template_hash`. Distinguishing requires reading the sibling
  scalar columns (`template_name`, `template_hash`) on the same row.
- `pass2_template_hash` is therefore **not stored as a distinct field
  in JSONB**. It only lives in `template_hash` of the pass-2 row.
- `cross_doc_entities` (count or list) — not in `ResolvedConfig`, not
  serialized. The pass-2 run cannot be reproduced from JSONB alone if
  more publications happen between original run and replay.
- `entities_json` content (or its hash) is not captured.
- `prior_context_doc_ids` — exists as a `pipeline_config` column
  (`mod.rs:161`) but is not surfaced into `ResolvedConfig` or JSONB.
- No model **provider**, no model **endpoint**, no model **costs at
  time of run** — `ResolvedConfig.model` is just the id. If the
  `llm_models` row's `cost_per_input_token` changes, the historical
  cost calc is non-reconstructable from JSONB; it is preserved on the
  `extraction_runs.cost_usd` scalar column, which is fine for cost,
  but the model identity (provider+endpoint) is not snapshotted.

### Pass-2 metrics captured, where

| Pass-2 metric | Stored where? |
|---|---|
| `input_tokens` | `extraction_runs.input_tokens` (pass-2 row) — set by `complete_extraction_run` (`extraction.rs:246-273`), called from `llm_extract_pass2.rs:423-435`. |
| `output_tokens` | `extraction_runs.output_tokens` (pass-2 row), same path. |
| `cost_usd` | `extraction_runs.cost_usd` (pass-2 row), same path. |
| `duration_secs` | **NOT captured anywhere for pass 2.** Only `started_at` and `completed_at` timestamps exist on the run row — duration must be derived. There is no `duration_secs` or `duration_ms` column on `extraction_runs`. (Per-chunk has `duration_ms` on `extraction_chunks`, but pass 2 writes no chunk rows.) |
| Pass-2 model name | `extraction_runs.model_name` (pass-2 row) — explicit (`llm_extract_pass2.rs:339`). |
| Pass-2 template content hash | `extraction_runs.template_hash` (pass-2 row) — and also `processing_config.template_hash` (overlapping). |
| Pass-2 template filename | `extraction_runs.template_name` (pass-2 row) — `pass2_template_file` value. **Not** `processing_config.pass2_template_file`-as-distinct-field; that field always carries the profile's pass-2 template filename, even on a pass-1 row. |

---

## 4. Profile Resolution and Versioning

### How `document_type` → profile filename mapping works

Single source of truth: `profile_name_for_document_type` in
`backend/src/api/pipeline/upload.rs:45-55` —

```
match document_type {
    "complaint" => "complaint",
    "discovery_response" => "discovery_response",
    "motion" | "motion_brief" => "motion",
    "brief" => "brief",
    "affidavit" => "affidavit",
    "court_ruling" => "court_ruling",
    _ => "default",
}
```

The mapping is hardcoded. The chosen name is appended with `.yaml` and
loaded from `state.config.processing_profile_dir` via
`ProcessingProfile::load` (`config.rs:167-187`); a missing file falls
back to `default.yaml`.

At extraction time, the pass-1 step resolves the profile name with this
priority (`llm_extract.rs:222-227`):

1. `pipeline_config.profile_name` (per-document override)
2. `default_profile_name_from_schema(pipe_config.schema_file)`
   (`llm_extract.rs:1445-1454`) — strips `_v\d+.yaml` and uses the bare
   stem, e.g. `complaint_v2.yaml` → `complaint`.

### Is the profile filename captured in the audit log?

Yes — only the **name**:

- `extraction_runs.processing_config.profile_name` — written by
  `write_processing_config_snapshot` (`llm_extract.rs:1497`).
- The profile YAML's `name:` field, not the filename, but in this codebase
  they are 1:1 (`brief.yaml` declares `name: brief` etc.).

### Could two runs against different *versions* of the same profile name
### be distinguished from the database alone?

**NO.** Concrete evidence:

1. **No profile content hash.** No SHA-256 of the YAML body is computed
   anywhere — grep finds `sha2_hex` only on the template file and the
   system prompt file (`llm_extract.rs:288-291`, `llm_extract.rs:311-312`,
   `llm_extract_pass2.rs:255,269`). The profile YAML itself is never
   hashed.

2. **No `profile_hash` column** on `extraction_runs` (verified by reading
   migrations 20260327, 20260410, 20260412, 20260420, 20260421,
   20260422113610, 20260422214842, 20260428213218 — all under
   `backend/pipeline_migrations/`). And no field on `ResolvedConfig` to
   serialize one to JSONB.

3. **Partial fingerprinting via descendant files.** `template_hash` and
   `system_prompt_hash` *are* recorded on each run. Therefore:
   - If a profile change *only* changes `extraction_model`,
     `pass2_extraction_model`, `chunking_mode`, `chunk_size`,
     `chunk_overlap`, `chunking_config`, `context_config`, `max_tokens`,
     `temperature`, `run_pass2`, `auto_approve_grounded`, or
     `synthesis_model`: the JSONB snapshot's *individual fields* (where
     present — synthesis_model and auto_approve_grounded are not)
     reflect the new values, so two runs would differ.
   - If a profile change *swaps* `template_file`, `pass2_template_file`,
     `system_prompt_file`, `global_rules_file`, or `schema_file` to
     point at a different file — the JSONB filename strings change too,
     so the runs differ.
   - If a profile change *edits* the same `template_file` content — the
     `template_hash` differs.
   - If a profile change *edits* the same `system_prompt_file` content
     — `system_prompt_hash` differs.
   - **If a profile change *edits* the same `global_rules_file` content
     — there is NO hash, so the runs are indistinguishable.** Gap 5.
   - **If a profile change *edits* the same `schema_file` content — the
     `schema_content` JSONB column on `extraction_runs` (F3, migration
     `20260410_f3_reproducibility_columns.sql`) was filled from
     `schema` via `serde_json::to_value(&schema)` at
     `llm_extract.rs:359` and `llm_extract_pass2.rs:347`, so the full
     schema body is captured per run.** That column is fine.
   - **If a profile change rewrites `chunking_config` keys** — the
     full map is in JSONB, so runs differ.

4. **The profile *YAML body* itself is not snapshotted anywhere.** A
   profile-level edit that changes only fields *not* on `ResolvedConfig`
   (`description`, `display_name`, `is_default`, `synthesis_model`)
   would be invisible. Of those, `synthesis_model` is the only one that
   could affect runtime behavior — and currently it has no consumer, so
   no observable run difference, but the audit gap stands.

5. **The profile **filename** as it lived on disk (`brief.yaml` vs.
   `brief_v2.yaml`) is not stored.** Only `processing_config.profile_name`
   is kept; the path to the YAML file is `processing_profile_dir +
   profile_name + ".yaml"` and that base filename is the only
   identifier. A profile rotated by overwriting the same path keeps the
   same `profile_name` value.

**Conclusion:** Two runs against `brief.yaml` versions A and B can be
distinguished by reading individual `processing_config` fields **only
if the change touched a field that is materialized on `ResolvedConfig`
or whose referenced file changed and is hashed (template, system
prompt) or content-snapshotted (schema)**. A change to the
`global_rules_file` content, or to a YAML-only field like
`description`/`synthesis_model`/`is_default`, is not detectable from the
DB state alone.

---

## 5. Config Panel Field Coverage

### Fields shown in the panel (`ConfigurationPanel.tsx`)

(Each is editable unless noted.)

1. **Profile** — dropdown, `profile_name` (`ConfigurationPanel.tsx:609-619`).
2. **Model** — dropdown, `extraction_model` (`ConfigurationPanel.tsx:625-635`).
3. **Template** — dropdown, `template_file` (`ConfigurationPanel.tsx:641-651`).
4. **Schema** — dropdown, `schema_file` — **silently inert on save**
   (`ConfigurationPanel.tsx:653-667` shows it; `ConfigurationPanel.tsx:507-510`
   drops it from the PATCH payload).
5. **Chunking** — dropdown, `chunking_mode` (`ConfigurationPanel.tsx:673-683`)
   — three modes (`full` / `structured` / `chunked`). This drives the
   *legacy* dispatch only when `chunking_config["mode"]` is absent (see
   Gap 6).
6. **Max Tokens** — number input, `max_tokens` (`ConfigurationPanel.tsx:689-696`).
7. **Temperature** — number input, `temperature` (`ConfigurationPanel.tsx:702-710`).
8. **Pass 2** — checkbox, `run_pass2` (`ConfigurationPanel.tsx:716-725`).
9. **Pass 2 Model** — dropdown, `pass2_extraction_model`, only visible
   when Pass 2 is checked (`ConfigurationPanel.tsx:740-752`).

Plus a content-classification info line (read-only) and Preview / Process
buttons.

### Fields that exist in `ProcessingProfile` but are NOT in the panel

| Profile field | UI status |
|---|---|
| `description` | Not shown — fine, display-only |
| `system_prompt_file` | **NOT EXPOSED** — interface field omitted from `Overrides` (`ConfigurationPanel.tsx:182-195`); explicit comment "no UI surface yet" (`ConfigurationPanel.tsx:215-218`) |
| `global_rules_file` | **NOT EXPOSED** — neither in TS profile interface (`configApi.ts:65-84`), nor in panel state, nor in PATCH input |
| `pass2_template_file` | **NOT EXPOSED** — TS profile interface (`configApi.ts:65-84`) does not declare it |
| `synthesis_model` | declared in TS profile interface (`configApi.ts:75`) but **NOT exposed** in panel, **NOT** in PATCH input |
| `chunking_config` (whole map and every sub-key) | **NOT EXPOSED** — TS interface does not declare it; backend override path hardcodes `None` (`mod.rs:518`, `upload.rs:447`, `config_handler.rs:80`); end-to-end inert |
| `context_config` | **NOT EXPOSED** — same shape as chunking_config |
| `chunk_size` | declared in `Overrides` interface (`ConfigurationPanel.tsx:190`) but **no JSX widget** |
| `chunk_overlap` | declared in `Overrides` interface (`ConfigurationPanel.tsx:191`) but **no JSX widget** |
| `auto_approve_grounded` | **NOT EXPOSED** anywhere on the path |
| `is_default` | Not shown — profile-list authoring concern, fine |

Also: `admin_instructions` (a `pipeline_config` column, not on the
profile) is captured at upload time (`upload.rs:344-345`) but the
Configuration Panel has no widget to edit it after upload.

### Pass-2 specific findings

- `pass2_template_file` (which prompt template runs Pass 2) — invisible
  in the UI. Operators cannot see which file is being used and cannot
  switch it. Backend treats it strictly as profile-level
  (`config.rs:431-433`).
- `synthesis_model` — declared on the TS profile interface but never
  surfaced. Not used by Pass 2 — Pass 2 uses `pass2_extraction_model`.
  The `synthesis_model` field is dead-data on the profile shape.
- `run_pass2` — exposed (checkbox) ✓.

---

## 6. Database Schemas (current state)

Composed from the migrations under `backend/pipeline_migrations/`:
`20260327_create_pipeline_tables.sql` (base); `20260410_f3_reproducibility_columns.sql`;
`20260412_fp7_chunk_extraction.sql`; `20260416_add_step_config_to_pipeline_config.sql`;
`20260420_config_system.sql`; `20260421_config_system_addl.sql`;
`20260422113610_add_unique_constraint_on_extraction_runs_document_and_pass.sql`;
`20260422214842_add_pass2_extraction_model_column.sql`;
`20260428213218_add_chunk_metadata_column.sql`.

### `extraction_runs`

| Column | Type | Source migration |
|---|---|---|
| `id` | SERIAL PRIMARY KEY | 20260327 |
| `document_id` | TEXT NOT NULL REFERENCES documents(id) | 20260327 |
| `pass_number` | INTEGER NOT NULL | 20260327 |
| `model_name` | TEXT NOT NULL | 20260327 |
| `input_tokens` | INTEGER | 20260327 |
| `output_tokens` | INTEGER | 20260327 |
| `cost_usd` | NUMERIC(10,4) | 20260327 |
| `raw_output` | JSONB NOT NULL | 20260327 |
| `schema_version` | TEXT NOT NULL | 20260327 |
| `started_at` | TIMESTAMPTZ NOT NULL | 20260327 |
| `completed_at` | TIMESTAMPTZ | 20260327 |
| `status` | TEXT NOT NULL DEFAULT 'RUNNING' | 20260327 |
| `assembled_prompt` | TEXT | 20260410 |
| `template_name` | TEXT | 20260410 |
| `template_hash` | TEXT | 20260410 |
| `rules_name` | TEXT | 20260410 — **never populated** by either pass |
| `rules_hash` | TEXT | 20260410 — **never populated** by either pass |
| `schema_hash` | TEXT | 20260410 — **never populated** (NULL passed at every insert; `schema_content` is used instead) |
| `schema_content` | JSONB | 20260410 — populated by both passes |
| `temperature` | DOUBLE PRECISION | 20260410 |
| `max_tokens_requested` | INTEGER | 20260410 |
| `admin_instructions` | TEXT | 20260410 |
| `prior_context` | TEXT | 20260410 — **never populated** (NULL passed at every insert; pass 2 inlines cross-doc context into the prompt but does not record what was used) |
| `chunk_count` | INTEGER | 20260412 |
| `chunks_succeeded` | INTEGER | 20260412 |
| `chunks_failed` | INTEGER | 20260412 |
| `chunks_pruned_nodes` | INTEGER | 20260412 |
| `chunks_pruned_relationships` | INTEGER | 20260412 |
| `processing_config` | JSONB | 20260420 |
| **constraint** `extraction_runs_doc_pass_unique UNIQUE (document_id, pass_number)` | — | 20260422113610 |

### `pipeline_config`

| Column | Type | Source migration |
|---|---|---|
| `document_id` | TEXT PRIMARY KEY REFERENCES documents(id) | 20260327 |
| `pass1_model` | TEXT NOT NULL DEFAULT 'claude-sonnet-4-6' | 20260327 |
| `pass2_model` | TEXT | 20260327 |
| `pass1_max_tokens` | INTEGER NOT NULL DEFAULT 32000 | 20260327 |
| `pass2_max_tokens` | INTEGER | 20260327 |
| `schema_file` | TEXT NOT NULL | 20260327 |
| `admin_instructions` | TEXT | 20260327 |
| `prior_context_doc_ids` | TEXT[] | 20260327 |
| `created_by` | TEXT NOT NULL | 20260327 |
| `created_at` | TIMESTAMPTZ NOT NULL DEFAULT NOW() | 20260327 |
| `step_config` | JSONB NOT NULL DEFAULT '{}' | 20260416 |
| `profile_name` | TEXT | 20260420 |
| `template_file` | TEXT | 20260420 |
| `system_prompt_file` | TEXT | 20260420 |
| `chunking_mode` | TEXT | 20260420 |
| `chunk_size` | INTEGER | 20260420 |
| `chunk_overlap` | INTEGER | 20260420 |
| `temperature` | NUMERIC(3,2) | 20260420 |
| `run_pass2` | BOOLEAN | 20260420 |
| `extraction_model` | TEXT | 20260421 |
| `max_tokens` | INTEGER | 20260421 |
| `pass2_extraction_model` | TEXT | 20260422214842 |

**No** `chunking_config` JSONB column. **No** `context_config` JSONB
column. **No** `pass2_template_file` column. **No** `global_rules_file`
column. **No** `synthesis_model` column. **No** `auto_approve_grounded`
column. **No** `profile_hash` column.

Note also the legacy/new dual columns: `pass1_model` (legacy, NOT NULL) +
`extraction_model` (override, nullable). `pass2_model` (legacy) +
`pass2_extraction_model` (override). `pass1_max_tokens` (legacy) +
`max_tokens` (override). The legacy NOT-NULL columns are still
authoritative for the base; overrides sit alongside.

### `extraction_chunks`

| Column | Type | Source migration |
|---|---|---|
| `id` | UUID PRIMARY KEY DEFAULT gen_random_uuid() | 20260412 |
| `extraction_run_id` | INTEGER NOT NULL REFERENCES extraction_runs(id) ON DELETE CASCADE | 20260412 |
| `chunk_index` | INTEGER NOT NULL | 20260412 |
| `chunk_text` | TEXT NOT NULL | 20260412 |
| `status` | TEXT NOT NULL DEFAULT 'pending' | 20260412 |
| `node_count` | INTEGER NOT NULL DEFAULT 0 | 20260412 |
| `relationship_count` | INTEGER NOT NULL DEFAULT 0 | 20260412 |
| `error_message` | TEXT | 20260412 |
| `input_tokens` | INTEGER | 20260412 |
| `output_tokens` | INTEGER | 20260412 |
| `duration_ms` | INTEGER | 20260412 |
| `created_at` | TIMESTAMPTZ NOT NULL DEFAULT NOW() | 20260412 |
| `chunk_metadata` | JSONB (nullable) | 20260428213218 |

---

## 7. Automated Verification

`GET /api/admin/pipeline/documents/:id/report` (`api/pipeline/report.rs:18-120`)
exists and produces a self-contained HTML page for one document, with:

- Total entity count.
- **Entity breakdown by type** — items grouped by `entity_type` and a
  separate `<h3>{type} ({count})</h3>` section per type
  (`report.rs:101-113, 212-244`).
- **Verification rates** — exact / normalized / not_found / pending
  counts (`report.rs:56-65, 188-191`).
- **Party names** — appear in the per-entity-type table when
  `entity_type == "Party"` (the `item_label` helper extracts
  `properties.party_name` etc., `report.rs:122-133`). Not as a dedicated
  party-only section, but party rows are visible.
- **Relationships** — total count + a flat table of all relationships
  (type, from-label, to-label, tier) (`report.rs:115-117, 246-272`).
  Aggregated breakdown by relationship_type is **not** present —
  every relationship gets one row, no per-type subtotal.
- **Run metadata** — model name, input/output tokens, cost (from the
  first run; not pass-1-vs-pass-2-specific) (`report.rs:67-78, 184`).

So a **partial** quality report exists. What's missing relative to the
audit's question:

- It does not aggregate relationship-by-type counts (only individual rows).
- It does not show which profile / template / model **versions** ran
  (no profile_name, no template_hash, no resolved chunking_config).
- It does not show pass-1 vs pass-2 results separately.
- It is HTML-only — no JSON/CSV form for programmatic checks.
- It is not invokable as a single command (only a logged-in admin GET).

No other "single quality report" endpoint or script exists — verified
by grepping `backend/src/api/pipeline/*.rs` for `report_handler`,
`build_report`, `extraction_summary`, `quality_report`, `metrics_report`
(only `report_handler` matched).

---

## 8. Summary of Gaps

Numbered for cross-reference. Each entry: **what is missing** ·
*why it matters for reprocessing trust* · **file(s) where the fix would land**.

1. **`chunking_config` / `context_config` per-document override path is
   completely missing.** The `pipeline_config` table has no JSONB column
   for either; `get_pipeline_config_overrides` hardcodes `None`
   (`mod.rs:518-519`); `PatchConfigInput` has no field for either
   (`config_handler.rs:36-61`); `frontend/src/services/configApi.ts:137-160`
   has no field for either; the upload handler hardcodes `None`
   (`upload.rs:447-448`).
   *Why it matters:* the only way to change chunking strategy
   per-document is to edit the profile YAML; that change applies to
   every future document of that type. There is no operator escape
   hatch and no audit when the YAML is rotated. This is the field
   that was discovered to be silently dropped yesterday — the fix in
   commit 5cb0b7d added the YAML keys but did not expose the
   override path.
   *Fix lands in:* new migration adding `pipeline_config.chunking_config
   JSONB`, `pipeline_config.context_config JSONB`; `mod.rs:486-525`
   (read both); `mod.rs:540-604` (write both via PATCH);
   `config_handler.rs:35-84` (DTO + From impl); `upload.rs:432-449`
   (seed at upload); `frontend/src/services/configApi.ts:65-160`
   (TS types); `frontend/src/components/pipeline/ConfigurationPanel.tsx`
   (widgets — likely a JSON editor or expanded sub-fields).

2. **Pass-2 template selection is invisible in the Configuration Panel.**
   `pass2_template_file` is loaded from YAML, used by the runtime, and
   recorded in the JSONB snapshot, but the TS `ProcessingProfile` type
   omits it (`configApi.ts:65-84`); `PatchConfigInput` omits it; the
   panel shows nothing about it.
   *Why it matters:* an operator running a `brief` document does not
   see which Pass-2 template will run. They cannot swap to a different
   pass-2 template per-document. After a run, the only way to know
   which pass-2 template ran is to read the pass-2 row's
   `extraction_runs.template_name` directly.
   *Fix lands in:* `configApi.ts` (add field to TS interface);
   `ConfigurationPanel.tsx` (add a Pass-2 Template dropdown nested
   under the existing run_pass2 checkbox); decision required on
   whether per-document override is wanted (then add column +
   override field — `config.rs:255-291`, `mod.rs`, `config_handler.rs`,
   `upload.rs`).

3. **Pass-2 cross-document context is not recorded.** The `prior_context`
   column on `extraction_runs` exists (`20260410_f3_reproducibility_columns.sql`)
   but is passed `None` at insert (`llm_extract_pass2.rs:351`).
   `prior_context_doc_ids` on `pipeline_config` (`20260327`) is read
   nowhere by the pass-2 code path. The cross-doc entities loaded at
   `llm_extract_pass2.rs:278-279` are silently inlined into the LLM
   prompt and not recorded.
   *Why it matters:* if more documents get PUBLISHED between original
   pass-2 and a replay, the replay will silently use a different
   cross-doc context. This breaks reproducibility for any pass-2 run
   that depends on already-published evidence.
   *Fix lands in:* `llm_extract_pass2.rs:332-356` (pass the
   serialized cross-doc context to `prior_context`); decide on a
   shape (list of `(doc_id, prefixed_id, item_id)` triples) and
   capture it in `ResolvedConfig` too if it should be in JSONB; or
   add a new column for the actual snapshot.

4. **No profile-content fingerprint.** `processing_config.profile_name`
   is just a string; there is no `profile_hash` field on
   `ResolvedConfig`, no `profile_hash` column on `extraction_runs`,
   and no SHA computed against the profile YAML body.
   *Why it matters:* if anyone edits `brief.yaml` between two runs,
   the audit log says "ran with profile_name=brief" both times. Two
   runs against different YAML versions cannot be told apart by the
   profile name alone — they can only be partially distinguished by
   the descendant fields' hashes (template_hash, system_prompt_hash)
   and only when the relevant file content actually changes.
   *Fix lands in:* `config.rs:148-160` (compute hash in `from_file` /
   `load`); `config.rs:200-248` (add `profile_hash: Option<String>`
   to `ResolvedConfig`); `llm_extract.rs:1497-1532` (carry through);
   migration to add `extraction_runs.profile_hash TEXT` (optional —
   JSONB carries it already once the field is on the struct).

5. **No `global_rules_hash` / `global_rules_name` captured.** Pass 1
   loads the rules fragment and substitutes it into the prompt
   (`llm_extract.rs:330-341`) but does not hash it. Pass 2 does not
   load the rules fragment at all (`llm_extract_pass2.rs:316-320`
   doesn't substitute `{{global_rules}}`). The F3 columns
   `rules_name` / `rules_hash` (`20260410`) exist but are passed
   `None` everywhere (`llm_extract.rs:357`, `llm_extract_pass2.rs:344`).
   *Why it matters:* `global_rules_v4.md` is shared across every
   profile and influences every extraction. Editing it changes every
   future run silently — no DB evidence shows which version of the
   rules was active.
   *Fix lands in:* `llm_extract.rs:330-341` (compute `sha2_hex` of
   the loaded rules); pass to `insert_extraction_run` as
   `rules_name=global_rules_file`, `rules_hash=hash`;
   `config.rs:200-248` (add `global_rules_hash: Option<String>` to
   `ResolvedConfig`); `llm_extract.rs:1497-1532` (carry through);
   `llm_extract_pass2.rs:316-320` (also load + substitute the rules
   in pass 2 — see Gap 6).

6. **`{{global_rules}}` and `{{admin_instructions}}` are NOT
   substituted in Pass-2 prompts.** `llm_extract_pass2.rs:316-320`
   replaces only `{{schema_json}}`, `{{entities_json}}`,
   `{{context}}` (with empty), and `{{document_text}}`. The comment
   at `llm_extract_pass2.rs:312-315` acknowledges this: "still
   intentionally unfilled — mirroring pass-1's current behavior;
   that gap is tracked separately." Pass 1 *does* substitute both as
   of commit 09d187c, so Pass 2 is now stale relative to Pass 1.
   *Why it matters:* operators authoring pass-2 templates that
   reference these placeholders will see literal `{{global_rules}}`
   text leak into the LLM. Pass-2 quality also lacks the
   per-document admin instructions that Pass 1 honors.
   *Fix lands in:* `llm_extract_pass2.rs:300-320` (load rules,
   substitute both placeholders, mirror Pass 1's
   `assemble_chunk_prompt` shape).

7. **`chunking_mode` legacy field can be silently overridden by
   `chunking_config["mode"]`.** `resolve_effective_mode`
   (`llm_extract.rs:1571-1580`) prefers `chunking_config["mode"]`
   when present, otherwise falls back to `chunking_mode`. The
   Configuration Panel only exposes the legacy `chunking_mode`
   dropdown. Profiles like `brief.yaml` set both
   `chunking_mode: structured` AND `chunking_config.mode: structured`
   today, but a profile (or future override) that sets
   `chunking_config.mode: structured` but `chunking_mode: full`
   would run *structured* while the UI shows *Full document*.
   *Why it matters:* the UI lies about which path the dispatcher
   will take when both fields disagree. A user looking at the
   "modified" badge in the panel will not see the real driver.
   *Fix lands in:* either (a) remove the legacy `chunking_mode`
   field from the panel and surface `chunking_config["mode"]`
   instead, or (b) drive both fields from the same widget. Likely
   (a) once chunking_config has a UI (Gap 1).

8. **`schema_file` widget is silently inert.** `ConfigurationPanel.tsx:653-667`
   shows a Schema dropdown; `ConfigurationPanel.tsx:507-510` drops it
   from the PATCH payload. The user can choose a schema, see the
   "modified" badge appear, click Save, and have the choice silently
   discarded.
   *Why it matters:* operators believe they overrode the schema and
   start a run; the run uses the original schema, producing items
   under the wrong type set; the panel's badge falsely signaled
   acceptance.
   *Fix lands in:* either disable the dropdown and add a tooltip
   ("schema is profile-level — change the profile to switch
   schema"), or wire a true override (column + DTO field + repo
   update + override resolver).

9. **`system_prompt_file`, `chunk_size`, `chunk_overlap`, `pass2_extraction_model`,
   `synthesis_model`, `auto_approve_grounded`, and `admin_instructions`
   have no UI widget.** `system_prompt_file` and `chunk_size`/`chunk_overlap`
   *do* have working override columns, so the gap is purely UI-side.
   `synthesis_model` and `auto_approve_grounded` are loaded from the
   profile and have no override path at all (Gap 9-A). `admin_instructions`
   is captured at upload but cannot be edited later in the panel
   (Gap 9-B).
   *Why it matters:* operators who want to tweak any of these for one
   document have to either edit the profile (affects all future docs)
   or run direct SQL. There is no in-app surface.
   *Fix lands in:* `frontend/src/components/pipeline/ConfigurationPanel.tsx`
   (widgets); for `synthesis_model`/`auto_approve_grounded` also need
   override columns + DTO fields (mirror `pass2_extraction_model`).

10. **`synthesis_model` is loaded and discarded.** `ProcessingProfile`
    parses it (`config.rs:81`); `ResolvedConfig` does not have a field
    for it; `resolve_config` does not copy it; no caller uses it; no
    column captures it.
    *Why it matters:* a profile author setting `synthesis_model:
    claude-opus-4-7` in YAML expects something to use that. Today
    nothing does and the value vanishes. Either it should be wired
    through (as the Pass-2/curation model) or deleted from the
    struct.
    *Fix lands in:* `config.rs:81` (decide: delete or surface);
    if surface, add field to `ResolvedConfig`, copy in
    `resolve_config`, decide which step consumes it (currently
    Pass 2 uses `pass2_model`).

11. **`pass2_extraction_model` does not appear in `processing_config`
    JSONB as a distinct named field — the snapshot conflates pass-1
    and pass-2 metadata.** On a pass-1 run row, `processing_config.model`
    = pass-1 model and `processing_config.pass2_model` = pass-2 model.
    On a pass-2 run row, the *same* `ResolvedConfig` is serialized,
    so `processing_config.model` is still the **pass-1 model id** and
    `processing_config.pass2_model` is the resolved pass-2 model.
    The actual pass-2 model is recorded on the sibling
    `extraction_runs.model_name` column, not in JSONB. Similarly,
    `processing_config.template_file` on a pass-2 row is the **pass-1**
    template filename — the pass-2 template filename is only on
    `extraction_runs.template_name` (and `processing_config.pass2_template_file`).
    *Why it matters:* a JSONB-only audit query against pass-2 rows
    will report the wrong model and template. Reconciliation requires
    cross-checking with the scalar columns. An auditor who does
    `SELECT processing_config->>'model' FROM extraction_runs WHERE
    pass_number = 2` will see the pass-1 model.
    *Fix lands in:* `config.rs:200-248` (decide whether to add an
    `effective_pass` discriminator, or to swap fields when serializing
    a pass-2 snapshot); `llm_extract.rs:1497-1532` (apply the
    discriminator).

12. **`schema_hash` and `schema_version` are only partially populated.**
    `schema_hash` column (`20260410`) is passed `None` at every
    insert (`llm_extract.rs:357-358`, `llm_extract_pass2.rs:345-346`).
    `schema_version` is bound from `schema.version`
    (`llm_extract.rs:352`, `llm_extract_pass2.rs:340`) and
    `schema_content` carries the full JSON, so reproducibility *is*
    achieved — but the dedicated hash column intended for fast
    diffing/grouping is unused.
    *Why it matters:* not a correctness gap (content snapshot
    suffices) but a performance / discoverability gap. Querying "all
    runs that used the same schema content" requires hashing the
    JSON in SQL or in app code.
    *Fix lands in:* `llm_extract.rs:347-368` and
    `llm_extract_pass2.rs:332-356` — compute hash once and bind.

13. **No `duration_secs` on `extraction_runs`.** Pass-1 chunked path
    captures `duration_ms` per chunk on `extraction_chunks`. Pass-2
    has no chunks and therefore no duration record beyond
    `started_at` / `completed_at`.
    *Why it matters:* aggregating cost-per-second / latency reports
    requires deriving from timestamps every time. Not blocking
    reprocessing trust, but useful for reproducibility comparisons.
    *Fix lands in:* either accept the timestamp delta as the duration
    (no DB change, just a query helper) or add a generated column.

14. **No automated single-command quality report aggregating
    relationship-by-type, profile/template versions, and pass-1
    vs pass-2 split.** The HTML report at `report.rs:18-120` covers
    entity-by-type and verification rates, but lacks
    relationship-by-type aggregation, profile/template version
    fingerprints, and pass-1/pass-2 separation.
    *Why it matters:* a reviewer looking at the report cannot see
    which version of the prompts produced these entities and
    relationships. They must cross-reference `extraction_runs`
    columns separately.
    *Fix lands in:* `backend/src/api/pipeline/report.rs` (add
    relationship-by-type subtotals; add a "Configuration" section
    sourced from `processing_config` JSONB; segregate pass-1 vs
    pass-2). Optionally a JSON-shaped sibling endpoint for
    programmatic checks.

---

## 9. Open Questions

1. **Is `chunking_config` intended to be a per-document override at
   all, or a profile-only knob?** The `PipelineConfigOverrides` struct
   (`config.rs:255-291`) reserves the field as `Option<HashMap<...>>`
   and the `resolve_config` merge logic (`config.rs:401-409`) is
   already written to support it. But the read site at
   `mod.rs:518-519` hardcodes `None` and the comment claims "Group 3's
   migration" will add the column. **Is Group 3 still planned, or
   should this become a profile-only field?** This determines whether
   Gap 1's fix is "add column + plumbing" or "delete the
   `Option<HashMap<…>>` from the override DTO and document that the
   knob is profile-only."

2. **Is `synthesis_model` supposed to be the pass-2 model, the
   curation model (a hypothetical pass 3), or dead code?** The
   profile loads it (`config.rs:81`) but no caller reads it. Pass 2
   uses `pass2_extraction_model`. Either delete the field or wire
   it.

3. **Does Roman want `{{global_rules}}` substituted into Pass-2
   prompts (Gap 6)?** The comment at `llm_extract_pass2.rs:312-315`
   suggests it's deferred work, not a deliberate exclusion, but the
   answer determines whether the audit recommendation is "wire it"
   or "remove the placeholder from pass-2 templates and confirm by
   convention".

4. **Is the `schema_file` panel widget intended to be a
   per-document override?** Today it is silently inert
   (Gap 8). Either intent is reasonable: schemas usually drive entity
   types and changing them mid-pipeline can produce items that don't
   match the verification path's expectations. The fix shape depends
   on the intent.

5. **For pass-2 audit: do we want the cross-doc context (Gap 3)
   recorded as a list of `(doc_id, prefixed_id, item_id)` triples in
   `prior_context` TEXT, or as a JSONB shape on the snapshot, or
   both?** The TEXT column is large enough to hold the inlined JSON
   if it's compact; JSONB on the snapshot is more queryable.

6. **Should `auto_approve_grounded` get a per-document override?** It
   is currently profile-only. A few documents might warrant
   no-auto-approve; if so, add the override column.

7. **Is the Configuration Panel meant to be the audit surface as
   well, or is `processing_config` JSONB the authoritative audit
   source and the panel is just a pre-run editor?** Today it is a
   pre-run editor only — completed runs do not surface the resolved
   `processing_config`. If the panel should also serve as the
   "what ran" view, there is additional UI work in scope.

8. **For pass-2 metric capture: is `duration_secs` a useful field to
   add, or is `completed_at - started_at` sufficient?** No code today
   queries duration directly.

9. **Profile-content fingerprint (Gap 4) — is the desired audit
   guarantee "two runs against differently-edited brief.yaml are
   distinguishable from the DB" *only* via descendant file hashes
   (template, system_prompt) plus any field on `ResolvedConfig`, or
   should the YAML body itself be hashed?** The descendant-file
   approach catches the changes that affect runtime behavior; a
   body hash also catches `description` / `display_name` /
   `is_default` edits, which are operationally meaningless. Decision
   affects whether the fix is "compute and store profile_hash" or
   "document that descendant fingerprints are sufficient."
