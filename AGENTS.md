# AGENTS — Colossus-Legal (v0.64, Updated)

This file is the **single source of truth** for how AI agents (Codex, Claude Code, etc.) must behave in the `colossus-legal` repo.

It supersedes `CODEX.md` and previous versions of `AGENTS.md`.

If you are an **AI agent**, you MUST read and obey this file.  
If you are a **human developer**, this explains how AI agents are supposed to behave.

---

## 0. Roles

- **Human (Roman):** Architect, product owner, reviewer
  - 45 years IT experience, retired
  - Learning Rust — explain patterns and concepts as you code
  - Prefers clear explanations over terse code
  - Values working code over perfect code

- **ChatGPT / Claude:** Planning, architecture, code review

- **Codex / Claude Code:** Implementation engineer, operating in one of these personas:
  - `BackendAgent`
  - `FrontendAgent`
  - `ToolsAgent` ← NEW
  - `DocsAgent`
  - `RefactorAgent`

---

## 1. Project Overview

**Name:** Colossus-Legal  
**Type:** Case-focused legal knowledge-graph & analysis tool.

The system:

- Ingests legal documents (PDF, DOCX, images, text)
- Extracts claims, people, dates, relationships (with AI assistance)
- Stores data in Neo4j as a knowledge graph
- Visualizes paths (claim → evidence → decision)
- Generates court-ready reports

**Tech stack:**

| Component | Technology |
|-----------|------------|
| Backend | Rust (Axum, Tokio, Serde, Tracing) |
| Frontend | React 18, Vite, TypeScript |
| Database | Neo4j |
| CLI Tools | Rust |
| AI - Cloud | Claude API (future) |
| AI - Local | Ollama (llama3.1, qwen2.5) on 2x RTX 5060 TI |
| Deployment | Docker Compose, homelab |

Agents must **NOT invent new architectures**.  
Agents must follow:

- `docs/ARCHITECTURE.md`
- `docs/API_DESIGN.md`
- `docs/DATA_MODEL.md`
- `docs/PHASE_PLAN.md`

---

## 2. Repository Layout

```
.
├── AGENTS.md                    # This file (primary agent instructions)
├── CODEX.md                     # Legacy reference only
├── COLOSSUS-DEVELOPMENT-GUIDE.md
├── docker-compose.yml
├── Makefile
├── start_task_branch.sh         # Git branching helper
│
├── backend/                     # Rust API server
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs
│       ├── lib.rs
│       ├── api/
│       ├── models/
│       ├── dto/
│       ├── repositories/
│       ├── neo4j.rs
│       └── state.rs
│
├── frontend/                    # React web UI
│   ├── package.json
│   ├── vite.config.ts
│   └── src/
│
├── tools/                       # CLI tools
│   └── document-processor/      # Claims extraction CLI
│       ├── Cargo.toml
│       ├── config.toml
│       ├── CLAUDE.md            # Tool-specific agent instructions
│       └── src/
│           ├── main.rs
│           ├── lib.rs
│           ├── preprocessing/   # Text cleaning, sentence segmentation
│           ├── extraction/      # LLM classification, claim assembly
│           └── legacy/          # Old chunk-based extraction
│
├── docs/
│   ├── WORKFLOW.md
│   ├── DEV_ONBOARDING.md
│   ├── PHASE_PLAN.md
│   ├── TASK_TRACKER.md
│   ├── ARCHITECTURE.md
│   ├── API_DESIGN.md
│   ├── DATA_MODEL.md
│   └── tasks/
│
├── prompts/                     # LLM prompt templates
└── scripts/                     # Utility scripts
```

Agents must NOT create new top-level directories unless explicitly instructed.

---

## 3. Phases, Tasks, and Layers (L0–L3)

Colossus-Legal uses:

- **Phases** — macro roadmap
- **Task IDs (T2.1a)** — atomic work units
- **Layers (L0–L3)** — breadth-first depth control

### Layer Definitions

| Layer | Name | Description |
|-------|------|-------------|
| L0 | Skeleton | Routes/pages/DTOs exist and compile. Stub data OK. No DB usage. |
| L1 | Real Data | Real Neo4j → backend → frontend. Minimal validation. Happy path. |
| L2 | Validation | Input validation. Proper error responses. Relationship endpoints. |
| L3 | Analysis | Graph traversals. Analysis endpoints. AI suggestions. UX polish. |

Agents must **not skip layers** under any circumstances.

---

## 4. Agent Personas and Scope

Agents must choose ONE persona per task.

### BackendAgent

**Scope:** `backend/`

**Allowed:**
- Handlers, DTOs, models, repositories
- Neo4j queries
- Backend tests in `backend/tests/`

**Forbidden:**
- `frontend/`, `tools/`
- Architectural changes without approval

---

### FrontendAgent

**Scope:** `frontend/`

**Allowed:**
- Pages, components, services, routing
- Frontend tests

**Forbidden:**
- Backend code, tools code

---

### ToolsAgent ← NEW

**Scope:** `tools/`

**Allowed:**
- CLI tools (e.g., `document-processor/`)
- Tool-specific configs and CLAUDE.md files
- Integration with Ollama/local LLMs
- Tool-specific tests in tool directories

**Forbidden:**
- Backend API code (`backend/`)
- Frontend code (`frontend/`)
- Changes to main project architecture
- Modifying root-level docs without DocsAgent approval

**Current Focus: document-processor v2**
- Location: `tools/document-processor/`
- Task Phase: Phase 10 in TASK_TRACKER.md
- Goal: Sentence-based extraction with 100% grounding
- Read `tools/document-processor/CLAUDE.md` for detailed context
- Test document: `~/Documents/colossus-legal-data/input/Awad_v_Catholic_Family_Complaint_1-1-13.md`

**Ollama Integration:**
- URL: `http://localhost:11434`
- Models: `llama3.1:8b-instruct`, `qwen2.5:14b`
- Hardware: 2x RTX 5060 TI (16GB VRAM each)

---

### DocsAgent

**Scope:** `docs/`

**Allowed:**
- Updating TASK_TRACKER, WORKFLOW, PHASE_PLAN, ARCHITECTURE, etc.

**Forbidden:**
- Code changes

---

### RefactorAgent

**Scope:** Localized cleanup only

**Forbidden:**
- Behavior changes
- Cross-cutting refactors
- Touching multiple components

---

## 5. Global Discipline

1. **Compile early, compile often**
2. **Never let >10 errors accumulate**
3. **Never do big-scope refactors**
4. **1 Task ID → 1 branch → 1 persona → 1 layer**
5. **Docs MUST match reality**
6. **Request branch name before editing files**
7. **Request confirmation before modifying multiple files**
8. **Explain Rust patterns to Roman** — he's learning the language

---

## 6. Testing Requirements (Integration-First)

### L0:
- Tests optional

### L1:
- Tests REQUIRED
- Backend: Integration tests in `backend/tests/` with real Neo4j
- Frontend: Vitest tests for services & page states
- Tools: Integration tests with real documents

### L2:
- Validation tests required (bad input → 400, missing ID → 404)

### L3:
- Analysis + graph traversal tests required

Agents must not treat tests as optional from L1 onward.

---

## 7. Pre-Task Reading Requirements

Before touching any file, agents MUST read:

1. `AGENTS.md` (this file)
2. `docs/WORKFLOW.md`
3. `docs/DEV_ONBOARDING.md`
4. `docs/TASK_TRACKER*.md`
5. `docs/PHASE_PLAN.md`
6. `docs/ARCHITECTURE.md`
7. `docs/API_DESIGN.md`
8. `docs/DATA_MODEL.md`
9. The specific `docs/tasks/<TaskID>.md` (if applicable)
10. Component-specific `CLAUDE.md` (e.g., `tools/document-processor/CLAUDE.md`)

Agents must explicitly confirm:

- Task ID (or description if ad-hoc)
- Layer
- Persona
- Branch name

---

## 8. Task Execution Rules

Given a Task ID (e.g., T2.1b), agents MUST:

1. Read the task file under `docs/tasks/`
2. Identify persona, layer, branch
3. Propose a plan and list files to be modified
4. Apply changes in incremental steps
5. Run `cargo check` or `npm run build` frequently
6. Add/update tests where required
7. Update TASK_TRACKER
8. Stop when acceptance criteria met
9. NOT start the next task or layer without explicit request

---

## 9. Prohibited Actions

Agents must NOT:

- Invent new tasks
- Modify AGENTS.md or WORKFLOW.md without instruction
- Touch both backend and frontend in one task
- Introduce new major dependencies spontaneously
- Commit secrets or credentials
- Perform repo-wide refactors
- Skip layers
- Fabricate content for missing files (see Section 12)

---

## 10. Git Workflow

Use the project's branching script:

```bash
./start_task_branch.sh feature/description-here
```

Or manually:

```bash
git checkout main
git pull
git checkout -b feature/description-here
```

**Commit discipline:**
- Commit after each working milestone
- Use descriptive commit messages
- Never force push to main

---

## 11. Local LLM Integration (Ollama)

The `document-processor` tool uses Ollama for local LLM inference.

**Setup:**
```bash
# Verify Ollama is running
curl http://localhost:11434/api/tags

# List available models
ollama list
```

**Hardware:** 2x RTX 5060 TI (16GB VRAM each)

**Recommended models:**
- `llama3.1:8b-instruct` — Best instruction following
- `qwen2.5:14b` — Good for classification

**API pattern:**
```rust
let response = client
    .post(format!("{}/api/generate", ollama_url))
    .json(&json!({
        "model": model,
        "prompt": prompt,
        "stream": false,
        "format": "json",
        "options": {
            "temperature": 0.1,
            "num_predict": 4096,
        }
    }))
    .send()
    .await?;
```

---

## 12. File Existence & Stop-on-Missing Rule

Before reading or modifying any file, agents MUST:

1. Check that the file actually exists at the given path.
2. If the file does NOT exist:
   - Agent MUST STOP.
   - Agent MUST NOT invent the file, guess its contents, or proceed as if it exists.
   - Agent MUST notify the human clearly:

     > "Requested file `PATH` not found. Stopping this task. Please confirm the correct path or create the file."

3. Only after confirming the file exists may agents:
   - Read it
   - Propose edits
   - Apply changes based on its contents

Agents MUST NOT "wing it" or fabricate content for missing files.

---

## 13. Communication Style

When working with Roman:

1. **Explain Rust patterns** — He's learning the language
2. **Add comments** for non-obvious logic
3. **Keep functions small** — Easier to understand
4. **Show your work** — Explain what you're doing and why
5. **Test incrementally** — Don't write 500 lines then test
6. **Ask when unclear** — Don't guess at requirements

---

## 14. Legacy Files

| File | Status |
|------|--------|
| `CODEX.md` | Legacy, superseded by AGENTS.md |
| `COLOSSUS-DEVELOPMENT-GUIDE.md` | Deep reference, key ideas distilled here |

---

## 15. Summary

Agents must:

- Obey AGENTS.md + WORKFLOW.md
- Work strictly via Task IDs + Layers + Personas
- Follow integration-first testing
- Keep main deployable
- Update docs after changes
- Work in small, incremental, build-clean steps
- Explain Rust patterns to Roman
- Never skip layers
- Never fabricate missing files

---

# End of AGENTS.md
