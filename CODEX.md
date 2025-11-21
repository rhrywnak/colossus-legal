# CODEX.md – Colossus-Legal

This file tells Codex **how to behave** when working in the `colossus-legal` repo.

- You (human) + ChatGPT = **Architects / PMs**
- Codex = **Implementation Engineer**
- Colossus-Legal is a **separate vertical** from `colossus-ai`, but follows the same architectural style.

Codex should **not invent architecture**.  
Codex should **follow the docs in `docs/` and instructions in this file.**

---

## 1. Project Overview (for Codex)

**Name:** Colossus-Legal  
**Type:** Case-focused legal knowledge graph + analysis + reporting tool  
**Main technologies:**

- Backend: Rust, Axum, Tokio
- Data: Neo4j (shared instance, separate logical space for Colossus-Legal)
- Frontend: React 18, Vite, TypeScript
- AI: External (Claude API) and/or local (Ollama / vLLM) – wired later
- Deployment: Docker Compose (homelab-friendly)

The system:

- Ingests legal documents (PDF, DOCX, images, text)
- Extracts claims, people, dates, relationships (with AI assistance)
- Stores them in Neo4j as a knowledge graph
- Visualizes paths (claim → evidence → decision)
- Generates court-ready reports

**Important:** This repo is focused on *one case / small number of cases* and a *small number of users* (2–3). Performance and UX matter, but complexity should be kept under control.

---

## 2. File Layout (what Codex should expect)

At minimum, the repo contains:

```text
.
├── README.md
├── CODEX.md               # this file
├── Makefile
├── docker-compose.yml
├── backend/
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs
│       └── lib.rs
├── frontend/
│   ├── package.json
│   ├── tsconfig.json
│   ├── vite.config.ts
│   ├── index.html
│   └── src/
│       ├── main.tsx
│       ├── App.tsx
│       └── styles/
│           └── index.css
├── docs/
│   └── README.md
├── prompts/
│   └── README.md
└── scripts/
    ├── dev-backend.sh
    └── dev-frontend.sh
```

The *detailed* architecture, APIs, data model, and implementation phases live in `docs/` and **must be treated as the source of truth**.

---

## 3. How Codex Should Work in This Repo

### 3.1 General Behavior

When working in this repo, Codex should:

1. **Read the relevant docs first**, especially:
   - `docs/ARCHITECTURE.md`
   - `docs/API_DESIGN.md`
   - `docs/DATA_MODEL.md`
   - `docs/PHASE_PLAN.md`
   - `docs/TASK_TRACKER.md`
2. **Follow the existing architecture.**
3. **Keep changes scoped to the task.**
4. **Prefer clarity and maintainability over cleverness.**
5. **Keep environment-specific values (like secrets or passwords) out of source code.**

### 3.2 Task Workflow

For each task:

1. Identify the task scope.
2. Read related architecture docs.
3. Plan which files to modify.
4. Implement minimal, correct code.
5. Write clean, readable diffs.
6. Add tests when appropriate.

---

## 4. Backend (Rust) Guidelines

- Use Axum, Tokio, Serde, Tracing.
- Use modules:
  - `config/`, `api/`, `services/`, `repositories/`, `models/`, `dto/`, `error/`, `llm/`, `extraction/`.
- Neo4j credentials come from env vars:
  - `NEO4J_URI`
  - `NEO4J_USER`
  - `NEO4J_PASSWORD`
- Use structured logs.
- Keep handlers thin; put logic in services.
- Use serde DTOs and strong types.

---

## 5. Frontend (React + Vite + TS) Guidelines

- Use React 18, Vite, TS.
- Use `src/pages/`, `src/components/`, `src/services/`, `src/hooks/`, `src/store/`.
- Use environment variable:
  - `VITE_API_URL` to reach backend.
- Early phases:
  - Stub pages, navigation, health check.
- Later phases:
  - Full document upload, AI review UI, graph views, exports.

---

## 6. Phased Implementation (Codex must follow)

Follow phases in: `docs/PHASE_PLAN.md`

1. Phase 1: Foundations  
2. Phase 2: Graph & Analysis  
3. Phase 3: Document Ingestion  
4. Phase 4: AI Suggestion Pipeline  
5. Phase 5: Batch Analysis  
6. Phase 6: Reporting & polish  

Codex must **not jump ahead** without explicit instruction.

---

## 7. Tests & Quality

- Add tests for services and API handlers.
- Use `cargo fmt`, `cargo clippy`.
- For frontend, use React Testing Library if needed (later phases).

---

## 8. Prohibited Actions

Codex must **not**:
- Change project scope.
- Introduce new frameworks.
- Hardcode secrets.
- Modify architecture docs unless instructed.
- Mix code from colossus-ai unless asked.

---

## 9. Example Task Format

When the human assigns tasks, they will look like:

**Title:** Implement Neo4j connection helper  
**Context:** See `docs/ARCHITECTURE.md` → Backend Architecture  
**Instructions:**  
- Use env vars for credentials  
- Create connection module  
- Fail fast on connection error  
- No queries yet  

Codex must follow exactly.

---

**End of CODEX.md**
