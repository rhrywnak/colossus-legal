# CLAUDE.md — colossus-legal

> **Read this file first. Then read `COLOSSUS-CC-STANDARDS.md` immediately after.**
> COLOSSUS-CC-STANDARDS.md contains the pre-coding analysis template,
> STOP gate rules, forensic mode, completion report format, and all
> generic coding standards. Both files must be read before any task begins.

---

## What this repo is

**Colossus-Legal** — Legal document analysis and case management system.

- **Backend:** Rust + Axum → `backend/`
- **Frontend:** React + Vite + TypeScript → `frontend/`
- **Pipeline DB:** PostgreSQL `colossus_legal_v2`
- **Graph DB:** Neo4j 5.x
- **Vector DB:** Qdrant (REST :6333, gRPC :6334)
- **Auth:** Authentik SSO → Traefik ForwardAuth → X-authentik-* headers
- **Shared libraries:** colossus-rs workspace (colossus-auth, colossus-rag,
  colossus-extract, colossus-pipeline)

Current phase, active branch, and task status are in the session transition
document — not here. Read the transition doc for that context.

---

## Colossus-legal specific rules

These rules apply permanently to this repo regardless of phase.
They are IN ADDITION to everything in COLOSSUS-CC-STANDARDS.md.

### AppContext not AppState
Steps accept `&AppContext` — never `&AppState`.
AppState is the old struct. AppContext holds Arc<dyn LlmProvider> etc.
If you see AppState in step code, that is a bug. Stop and flag it.

### LlmProvider — never bypass
Call `context.llm_provider.invoke()` for all LLM calls.
Never call AnthropicChunkExtractor or rig::providers::anthropic directly.

### EmbeddingProvider — never bypass
Call `context.embedding_provider.embed()` for all embedding calls.
Never call rig_fastembed directly.

### Constants — always use them
All document status values come from `backend/src/pipeline/constants.rs`.
Never write "COMPLETED", "FAILED", "NEW" etc. as string literals in code.
If constants.rs does not exist yet for a task, create it first.

### Step idempotency — mandatory
Every step must check whether its work is already done before doing it.
Re-running a step on an already-processed document must produce the
same result without duplicating data. No exceptions.

### No tokio::spawn in steps
Steps are called by the Worker. The Worker handles concurrency.
Steps must never spawn their own tasks.

### Transactions for multi-table writes
Any step that writes to more than one table must use a PostgreSQL
transaction. Partial writes are not acceptable.

### Cleanup hooks — mandatory
on_cancel() and on_delete() must reverse all external side effects:
- Neo4j nodes written by this step
- Qdrant vectors written by this step
- PostgreSQL rows written by this step

A no-op on_cancel() or on_delete() is only correct when the step
has zero external side effects. Always verify before leaving as no-op.

### Environment variables — always update Ansible
When adding any new env var to the backend:
1. Add to `backend/src/config.rs`
2. Add to Ansible template `colossus-legal-backend.env.j2`
3. Add to group_vars
4. Add to Ansible vault if it is a secret

Never add an env var to config.rs without updating the Ansible template.
They must stay in sync or DEV/PROD will have different behavior.

### Timeouts — mandatory on all HTTP calls
Frontend: every `authFetch` must use AbortController
- Normal endpoints: 30 seconds
- `/ask` (RAG synthesis): 90 seconds

Backend: every `reqwest::Client` built with `.timeout()` and
`.connect_timeout()`. Never create a per-request reqwest::Client —
share via AppState.

### Migration files — always use the script
NEVER create migration files manually. Use the provided script:

```bash
./scripts/new-migration.sh pipeline "description of change"
./scripts/new-migration.sh main "description of change"
```

The script generates timestamp-based filenames (`YYYYMMDDHHMMSS_<slug>.sql`)
that are unique to the second. Date-based prefixes (`YYYYMMDD`) caused a
production crash on 2026-04-20 when two migrations landed the same day and
sqlx panicked with VersionMismatch at startup.

Existing migrations with `YYYYMMDD` prefixes are locked — do not rename
them, because sqlx tracks applied versions by number and a rename looks
like a brand-new migration to the tracker.

`./scripts/check-migrations.sh` validates uniqueness across both migration
directories. It runs as part of `build-release.sh` (in colossus-ansible),
so duplicates fail the build before any container is shipped. Run it
manually before committing a new migration.

---

## Infrastructure

| Role | Hostname | IP |
|------|----------|----|
| DEV app | colossus-dev-app1 | 10.10.100.220 |
| DEV DB | colossus-dev-db1 | 10.10.100.200 |
| PROD app | colossus-prod-app1 | 10.10.100.120 |
| PROD DB | colossus-prod-db1 | 10.10.100.110 |

All servers run CoreOS. All podman commands require `sudo`. SSH user is `core`.
Backend API port: 3403. All routes under `/api/` prefix. Exception: `/health` at root.

### DEV PostgreSQL access
```bash
ssh core@10.10.100.200 "sudo podman exec colossus-postgres psql \
  -U postgres -d colossus_legal_v2 -c \"YOUR QUERY\""
```

### Logging rule
NEVER issue `journalctl` without `-n 5` or less AND `grep` with a
specific search term. No exceptions.

### Deploy sequence — never deviate
1. `git add -A && git commit`
2. `./scripts/bump-version.sh {ver}` → commit → tag → push with tags
3. `cd ~/Projects/colossus-ansible && ./scripts/build-release.sh v{ver}`
4. Deploy via Semaphore web UI only — never `ansible-playbook` directly

---

## Commands for this repo

```bash
# Branch verification
git branch --show-current
git status

# Backend
cd backend && cargo check
cd backend && cargo test
cd backend && cargo build
cd backend && cargo clippy -- -D warnings

# Frontend
cd frontend && npm run build
cd frontend && npm run lint

# Module size check (code lines, excluding comments)
find backend/src -name "*.rs" -exec sh -c \
  'lines=$(grep -v "^[[:space:]]*//\|^[[:space:]]*\*\|^[[:space:]]*$" "$1" | wc -l); \
   if [ $lines -gt 300 ]; then echo "OVER 300: $lines $1"; fi' _ {} \;
```

---

## What NOT to do in this repo

❌ Use AppState in step implementations — use AppContext
❌ Call AnthropicChunkExtractor — use context.llm_provider.invoke()
❌ Call rig_fastembed directly — use context.embedding_provider.embed()
❌ Write document status as string literals — use constants from constants.rs
❌ Use tokio::spawn inside step implementations
❌ Write steps without idempotency checks
❌ Write multi-table steps without transactions
❌ Leave on_cancel/on_delete as no-ops when side effects exist
❌ Add env vars to config.rs without updating the Ansible template
❌ Create authFetch calls without AbortController timeout
❌ Create reqwest::Client without timeout
❌ Use :latest on container images
❌ Deploy via ansible-playbook directly — Semaphore UI only
❌ Bump version numbers — Roman does that
❌ Reference or modify files in colossus-rs, colossus-ansible, or colossus-homelab

---

*This file contains permanent rules only.*
*Current phase, branch, and task state: read the session transition document.*
*Full coding standards, STOP gate, pre-coding template: read COLOSSUS-CC-STANDARDS.md.*
