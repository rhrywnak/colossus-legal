# CLAUDE.md — Colossus-Legal

> **Read this FIRST, every session.** It is the standing context for all coding work in this repo. Then read the specific instruction Roman provides for the current task.
>
> Last updated: 2026-05-03

---

## 1. PROJECT

**Colossus-Legal** — Litigation-support system for *Awad v. CFS / Phillips*. Document ingest → AI extraction → human review → knowledge graph + vector store → trial-prep query interfaces.

| Component | Tech | Location |
|---|---|---|
| Backend | Rust + Axum | `backend/` (port 3403) |
| Frontend | React + Vite + TypeScript | `frontend/` (port 5473) |
| Knowledge Graph | Neo4j 5.x Community | DEV `bolt://10.10.100.200:7687`, PROD `bolt://10.10.100.110:7687` |
| Vector Store | Qdrant | DEV `http://10.10.100.200:6333` (REST), `:6334` (gRPC); same hosts on PROD |
| Relational Store | PostgreSQL — `colossus_legal` database | DEV `10.10.100.200`, PROD `10.10.100.110` |
| Auth | Authentik SSO via Traefik ForwardAuth | `X-authentik-*` headers reach backend |
| RAG Pipeline | `colossus-rag` crate (Rig framework + Claude API) | shared workspace |

**Repos in this project:**
- `colossus-legal` — application (this repo)
- `colossus-rs` — shared Rust workspace (`colossus-extract`, `colossus-pipeline`, `colossus-rag`, `colossus-pdf`, `colossus-auth`)
- `colossus-ansible` — deployment automation
- `colossus-homelab` — infrastructure docs and Butane configs

**Current working branch (as of 2026-05-03):** `feature/intelligent-chunking` at `v2.0.0-beta.214`. Always verify with `git branch --show-current` before starting work.

---

## 2. HUMAN CONTEXT

**Roman** — 45 years IT, CS degree, retired, learning Rust. Treat code as a teaching artifact:

- Explain *why* a pattern is used, not just what it does
- Annotate Rust-specific concepts with `## Rust Learning:` doc-comment headers
- Working code over perfect code; clear explanations over terse code
- Reference `docs/RUST-PATTERNS.md` when applicable

---

## 3. THE THREE STANDING RULES

These three rules apply to **every line of code** written in this repo. Roman should not have to repeat them in individual instructions.

### Rule 1 — No Silent Failures

Every operationally distinct state must produce a different observable. Errors propagate with context; they are never swallowed.

**Concrete requirements:**

- No `.unwrap()` or `.expect()` in production paths. (Tests are fine; startup-once panics with a documented invariant are fine when the invariant is asserted at construction.)
- No `let _ = ...` to discard a `Result`.
- No `if let Ok(_)` or `if let Some(_)` patterns that drop the error case silently.
- Every `?` propagation must terminate at a handler that logs the error with context (`tracing::error!` with the operation, the inputs, the underlying cause).
- Frontend: every `fetch` / `authFetch` has explicit `.catch()` and explicit error UI. No swallowed promise rejections.
- Configuration: missing required config is a startup error with a message identifying which key is missing. Not a runtime surprise.
- An empty file, a missing file, an empty JSON object, and a missing JSON object must be **distinguishable** in logs and behavior. Not collapsed.
- Boundaries between audiences (template authoring comments vs. LLM prompt content) are code-enforced, not convention-enforced.

**The test:** if it can fail, the failure is observable. If a reader of the logs cannot tell *what* failed and *why*, the error handling is incomplete.

### Rule 2 — No Hardcoded Values

Anything that varies across environments, cases, or configurations comes from configuration. Code is case-agnostic; case-specific data lives in YAML, env vars, or database rows.

**Concrete requirements:**

- No URLs, ports, credentials, or absolute file paths in code.
- No magic numbers. Thresholds, timeouts, limits, sizes — all in env vars or YAML config, with `#[serde(default)]` for forward compatibility.
- No domain-specific names, terms, or aliases in shared library code. Bias-tag vocabulary, document-type names, person aliases — all in config, never compiled in.
- Configuration default values: prefer `Default` impls or YAML, not `const` / `static`.
- The reusability checkpoint applies: *"Could another Colossus project (e.g., colossus-ai) use this with zero code changes?"* If no, the offending value is hardcoded — extract to config.
- `.env`, `.env.local`, and `.fastembed_cache/` must be in `.gitignore` and never committed.
- Container images: pin to specific version tags, never `:latest`.
- Frontend: no hardcoded backend URLs. Use the relative `/api/*` path or a single configurable base URL.
- Tests are allowed to use literal expected values — that's the test asserting an invariant. Production code is not.

**The test:** to change a default, can Roman edit YAML/env vars and restart, with no code change and no rebuild? If a code change is required, it was hardcoded.

### Rule 3 — Tutorial-Quality Comments

Code in this repo is read by Roman as he learns Rust. Comments teach. They explain *why*, not just *what*.

**Concrete requirements:**

- Every non-trivial function has a doc comment (`///`) explaining its purpose, parameters, return value, and any side effects.
- Rust-specific concepts get teaching headers in doc comments:
  - `## Rust Learning: Arc<dyn Trait>` — explain trait objects, why Arc, when to use
  - `## Rust Learning: lifetime 'a` — explain why a lifetime is needed here
  - `## Rust Learning: ? operator` — when first introduced in a module
  - `## Rust Learning: async / .await` — explain blocking implications
  - `## Rust Learning: From / Into` — explain the trait pair when used
- Architectural decisions get a `// Why:` comment explaining the choice and what alternatives were considered.
- Domain-specific reasoning gets a `// Domain note:` comment connecting the code to legal / case context (e.g., *"Domain note: STATED_BY is the speaker who made the statement under oath; ABOUT is who the statement concerns. Different relationships, different queries."*).
- Inline comments explain non-obvious mechanics: why an unwrap is safe, why a clone is required, why a particular iterator chain is used over a loop.
- Comments are written for someone with 45 years of IT experience but learning Rust — neither too basic ("this is a variable") nor assuming Rust expertise ("obviously we use a `Cow` here").
- Pattern in `colossus-rag/src/router.rs` and `colossus-rag/src/retriever.rs` is the established style — match it.

**The test:** can Roman read the code, follow the *intent*, and learn one new Rust thing per non-trivial file? If the code is correct but Roman can't explain it back, the comments are incomplete.

---

## 4. THE SUPPORTING RULES

These come from accumulated lessons across the project. They are not as universal as the Three Standing Rules, but they apply broadly.

### Workflow

1. **Pre-Coding Analysis required before any code.** STOP gate. See Section 5 for the template.
2. **Wait for "Proceed" or equivalent approval before writing code.**
3. **One CC instance per repo per instruction.** Never commingle changes across `colossus-legal`, `colossus-rs`, `colossus-ansible`, `colossus-homelab` in a single instruction.
4. **No version bumps by CC.** Roman bumps versions and tags. CC writes code, edits files, builds, tests, and commits — nothing further.
5. **CC does not perform file reads, greps, finds, or API verification.** Those operations belong to the Opus session. CC writes/edits/builds/tests/commits only.
6. **Tests must verify behavioral correctness with specific inputs and expected outputs.** A clean compile is not verification.
7. **Research before designing.** When an instruction prescribes a derive, trait bound, API shape, or pattern, the prescription is grounded in canonical sources (Rust API Guidelines, std docs, proven production code) — never invented from training alone.
8. **No tech debt accumulation.** Post-implementation review issues (duplication, questionable patterns, gaps in test coverage) get fixed before push. Amend the commit if pre-push; new task if post-push, but prioritized — not backlogged indefinitely.

### Architecture

9. **Steps accept `&AppContext`, never `&AppState`.** Pipeline steps are domain-agnostic.
10. **Call provider traits, never concrete implementations.** `context.llm_provider.invoke()` not `AnthropicChunkExtractor`. `context.embedding_provider.embed()` not `rig_fastembed::...`.
11. **Reusability checkpoint mandatory.** Pipeline crate, providers, generic helpers: `colossus-ai` must be able to use them with zero code changes. Domain-specific logic stays in `colossus-legal`.
12. **No business logic in the frontend.** All state transitions and action availability live in the backend.

### Errors and Resilience

13. **Every HTTP call has a timeout.**
    - Backend: `reqwest::Client::builder().timeout(...).connect_timeout(...).build()`. Share one client via `AppState`; do not create per request.
    - Frontend: every `fetch` uses `AbortController` with a timeout signal. Normal: 30s. RAG/synthesis: 90s.
    - `qdrant-client` must have timeout configured.
14. **No silent retries.** Retries are explicit, logged, and bounded.
15. **Startup-time validation > runtime surprises.** Required config, required files, required external services all verified at startup. Fail loudly and early.

### Code Hygiene

16. **No magic values.** Use named constants from `constants.rs` — no string literals or magic numbers in business logic.
17. **No module over 300 lines** (excluding doc comments). Split if longer.
18. **No function over 50 lines.** If approaching the limit, extract a helper.
19. **`cargo check` after every change.** Don't accumulate more than 10 errors.
20. **Run formatters and linters before commit:** `cargo fmt --check`, `cargo clippy --workspace`, `npm run typecheck`, `npm run lint`.
21. **Disk/code consistency tests for invariants.** When an invariant must hold across many files (e.g., "no profile YAML carries `synthesis_model:`"), write a test that scans the filesystem and asserts it. Catches regression that review alone misses.

### Deployment

22. **No plaintext secrets** anywhere — code, config, Butane files, scripts. Secrets in Ansible Vault or gitignored `.env`.
23. **Pin container versions.** Never `:latest`.
24. **Never deploy via `ansible-playbook` directly.** Always Semaphore web UI.
25. **Never create migration files manually.** Use `./scripts/new-migration.sh` — the HHMMSS suffix prevents collisions.
26. **`.env`, `.env.local`, `.fastembed_cache/` are gitignored.** Verify before first commit on a new feature branch.
27. **Audit before deploy.** Verify the full path: browser → Traefik → Authentik → backend → external services → response. Component-level testing passing is necessary but not sufficient.

### Testing

28. **`cargo test --workspace` is the standard target.** As of 2026-05-02 Instruction G, the workspace test target is clean. Do not regress to `cargo test --lib` workarounds.
29. **`npm run typecheck` is a clean baseline.** Do not commit code that breaks it.
30. **Frontend test pattern: pure-helper tests + service tests.** Component testing infrastructure (RTL, jsdom) is not currently set up.

---

## 5. PRE-CODING ANALYSIS TEMPLATE

For every task, CC produces this analysis BEFORE writing any code. Roman approves before implementation proceeds.

```markdown
## Pre-Coding Analysis for [Task ID or short description]

### 1. Task Understanding
[2-3 sentences: what will be implemented, what problem it solves]

### 2. Branch Verification
- Current branch: `feature/...`
- Working tree clean: YES / NO (if no, list uncommitted files)
- Last commit: `<hash> <message>`

### 3. Files to Modify
| File | Changes (summary) | Est. lines changed |
|------|-------------------|--------------------|

### 4. Files to Create
| File | Purpose | Est. total lines |
|------|---------|------------------|

### 5. Dependencies / External Surface
- New crates: [list with version, or "None"]
- Removed crates: [or "None"]
- New env vars: [list with default, or "None"]
- New config files or schemas: [or "None"]
- API endpoints added: [list with method + path, or "None"]
- API endpoints changed: [list, or "None"]

### 6. Rust Patterns to Implement
| Pattern | Where | Why |
|---------|-------|-----|

### 7. Tests to Write
| Test name | What it asserts | Module |
|-----------|-----------------|--------|

### 8. Standing Rule Compliance
- No silent failures: [confirm with one-sentence explanation of error paths]
- No hardcoded values: [confirm or list any new config additions]
- Tutorial comments: [confirm planned for new non-trivial functions]

### 9. Deployment Impact
- New env vars: [list, or "None"]
- Ansible template changes: [or "None"]
- Migrations: [or "None"]
- Container rebuild: YES / NO (frontend? backend? both?)
- Traefik / auth changes: [or "None"]

### 10. Verification Plan
- How to test locally (commands):
- How to test on DEV after deploy:
- Full path test: [browser → ... → response]

### 11. Rollback Plan
[What to revert if this breaks DEV]

### 12. Open Questions for Roman
[List any ambiguities, conflicts with existing code, or design choices needing input. Better to ask now than guess.]
```

**STOP. Wait for "Proceed" before writing code.**

---

## 6. POST-CODING REQUIREMENTS

Before reporting completion:

```bash
# In the repo root or backend/
cargo build --workspace
cargo test --workspace
cargo clippy --workspace -- -D warnings
cargo fmt --check

# In frontend/
npm run typecheck
npm run lint
npm run build
npm test

# Git hygiene
git diff --name-only       # only files in approved list?
git status                 # clean tree apart from approved changes?
```

Provide completion report including:

- Build results (`cargo build` and `npm run build`)
- Test results (count passed/failed/ignored for each suite)
- Clippy and lint output (must be clean)
- Files changed (vs. approved list)
- Anything observed during implementation that wasn't in the original analysis (deferred items, new tech debt, surprises)

If new env vars were added, confirm Ansible template was updated. If new endpoints were added, confirm frontend calls have timeouts. If new HTTP clients were created, confirm timeouts configured.

---

## 7. RUST QUICK REFERENCE

```rust
// ✅ Required derives on DTOs
#[derive(Debug, Clone, Serialize, Deserialize)]

// ✅ Snake-case enum tags for JSON
#[serde(rename_all = "snake_case")]

// ✅ Typed error enums
#[derive(Debug, thiserror::Error)]
pub enum MyError {
    #[error("operation X failed for {input}: {source}")]
    Variant { input: String, #[source] source: anyhow::Error },
}

// ✅ Optional fields skip serialization when None
#[serde(skip_serializing_if = "Option::is_none")]
pub field: Option<String>,

// ✅ Forward-compatible config struct
#[derive(Deserialize)]
struct Config {
    #[serde(default)]
    pub timeout_secs: Option<u64>,
}

// ✅ HTTP client with mandatory timeouts
let client = reqwest::Client::builder()
    .timeout(Duration::from_secs(30))
    .connect_timeout(Duration::from_secs(5))
    .build()?;

// ✅ Version from Cargo.toml — not a string literal
version: env!("CARGO_PKG_VERSION"),

// ❌ NEVER in production code
option.unwrap();              // use ? or pattern-match
"some error".into();          // use a typed error
reqwest::Client::new();       // use builder with timeout
let _ = result;               // explicit handling required
```

---

## 8. COMMANDS

```bash
# Quick checks
cd backend && cargo check
cd backend && cargo test --workspace
cd backend && cargo clippy --workspace -- -D warnings
cd backend && cargo fmt --check

cd frontend && npm run typecheck
cd frontend && npm run lint
cd frontend && npm test

# Git
git branch --show-current
git status
git diff --name-only

# Module size check (run before committing)
find backend/src -name "*.rs" -exec sh -c \
  'lines=$(grep -v "^\s*$" "$1" | grep -v "^\s*//" | wc -l); \
   if [ "$lines" -gt 300 ]; then echo "OVER ($lines): $1"; fi' _ {} \;
```

---

## 9. ARCHITECTURE QUICK REFERENCE

```
Browser → Traefik (TLS termination)
        → Authentik ForwardAuth (frontend routes only)
        → Backend (port 3403; reads X-authentik-* headers; backend enforces auth on its API routes)
        → Neo4j (10.10.100.200:7687 DEV, 10.10.100.110:7687 PROD)
        → Qdrant (10.10.100.200:6334 gRPC DEV / PROD same host as PROD DB server)
        → PostgreSQL (colossus_legal db on the DEV/PROD DB hosts)

RAG pipeline (Chat / Ask):
  Question → Router → QdrantRetriever → Neo4jExpander → LegalAssembler → RigSynthesizer → Answer

Repos:
  colossus-legal     — app (this repo)
  colossus-rs        — shared workspace: colossus-extract, colossus-pipeline,
                       colossus-rag, colossus-pdf, colossus-auth
  colossus-ansible   — deployment (Semaphore-driven)
  colossus-homelab   — infrastructure (Proxmox, CoreOS, Butane)
```

---

## 10. KEY DOCUMENTS

| Document | When to Read |
|----------|--------------|
| `OPUS_SESSION_PROTOCOL.md` (project knowledge) | Start of every Opus session |
| Most recent `COLOSSUS_LEGAL_SESSION_TRANSITION_*.md` | Continuity between sessions |
| `docs/DATA_MODEL_v3.md` (or successor) | Working on Neo4j models or queries |
| `docs/RUST-PATTERNS.md` | Writing Rust code |
| `BIAS_ANALYSIS_FEATURE_DESIGN_v1.md` | Building Bias Explorer or related features |
| `INTELLIGENT_CHUNKING_DESIGN_v2.md` | Pipeline chunking work |
| `EXTRACTION_TEMPLATE_CONSTRUCTION_GUIDE_v2.md` | Authoring extraction templates |
| `COLOSSUS_LEGAL_MASTER_TRACKER_v6.md` | Open issue tracker |

---

## 11. WHAT NOT TO DO

❌ Write code before Pre-Coding Analysis is approved
❌ Modify files outside the approved list
❌ Add features not in the task spec
❌ Use `unwrap()` or `expect()` in production handlers
❌ Use `let _ = ...` to discard `Result`
❌ Create modules over 300 lines or functions over 50 lines
❌ Create HTTP clients without timeouts
❌ Create fetch calls without `AbortController`
❌ Hardcode secrets, API keys, URLs, magic numbers, or domain-specific values
❌ Use `:latest` container tags
❌ Suppress build errors with `2>/dev/null || true`
❌ Commit `.env*` files or `.fastembed_cache/`
❌ Add a backend env var without updating the Ansible template
❌ Bump versions or tag releases (Roman does that)
❌ Commingle changes across multiple repos in one instruction
❌ Skip the Three Standing Rules — silent failures, hardcoded values, missing tutorial comments are rejected on review
❌ Bypass the UI with curl or direct DB writes to work around missing functionality

---

## 12. IF SOMETHING GOES WRONG

**STOP all edits.** Report the issue. Read-only operations only until Roman responds.

Specifically:
- If you discover an existing bug while implementing a feature, document it but do not fix it in the same instruction unless Roman approves scope expansion.
- If a test fails for reasons unrelated to your change, document it and ask before debugging.
- If `cargo build` or `npm run build` breaks unexpectedly after your change, do not "fix it forward" — revert and report.

---

## 13. THE LAYER SYSTEM

For multi-step features, work in layers. Never skip a layer.

| Layer | Description |
|-------|-------------|
| L0 | Skeleton — compiles, structure in place, no real behavior |
| L1 | Real Data — happy path works end-to-end |
| L2 | Validation — error handling complete, edge cases covered |
| L3 | Integration — advanced features, polish |

Each layer is a checkpoint. Don't move to L1 until L0 is committed and tested.

---

# End of CLAUDE.md
