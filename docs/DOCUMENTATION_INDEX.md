# Colossus-Legal — Documentation Index

This file is the **master index** for the Colossus-Legal documentation system.
It explains each document, how it fits into the workflow, and when to use it.

---

## 1. Foundation Layer (Read First)

These define *how we work*.

- **AGENTS.md**  
  Defines Codex behavior, agent personas, scopes, and safety rules.

- **docs/WORKFLOW.md**  
  End-to-end engineering workflow: phases → tasks → layers (L0–L3) → branching → versioning.

- **docs/DEV_ONBOARDING.md**  
  Developer guide: coding standards, module size rules, testing practices, Rust/React patterns.

- **docs/PHASE_PLAN.md**  
  High-level multi-phase roadmap (Foundations → Claims → Graph → AI pipeline → Reporting).

- **docs/TASK_TRACKER.md** or `TASK_TRACKER_OPTION_B.md`  
  Canonical list of Task IDs (T2.1a, T2.1b, …), statuses, layers, and acceptance criteria.

---

## 2. Architecture & Design Layer

These define how the system is structured and how components talk to each other.

- **docs/ARCHITECTURE.md**  
  System architecture for backend + frontend + Neo4j, with example flows (e.g., GET /claims tutorial).

- **docs/API_DESIGN.md**  
  REST API specifications: endpoints, DTOs, status codes, and step-by-step design notes.

- **docs/DATA_MODEL.md**  
  Neo4j schema (nodes, relationships, properties) with example Cypher queries and subgraphs.

**Phase 2 / Claims references**
- Claims API/flow docs: `docs/API_DESIGN.md` (Claims endpoints, errors), `docs/ARCHITECTURE.md` (Claims v1 end-to-end).
- Roadmap: `docs/PHASE_PLAN.md` (Phase 2 status and remaining tasks).
- Tasks: `docs/tasks/T2.1a_Claims_API_L0.md`, `docs/tasks/T2.1b_Claims_API_L1.md`, `docs/tasks/T2.1c_Claims_API_L2_Validation.md`, `docs/tasks/T2.2a_Claims_UI_L0.md`, `docs/tasks/T2.2b_Claims_UI_L1.md`, `docs/tasks/T2.3_Claims_Integration.md` (if present).
- Document references: `docs/ARCHITECTURE.md` (Document v1 end-to-end, L1 list), `docs/API_DESIGN.md` (Document endpoints: GET /documents implemented, CRUD/analysis planned), `docs/DATA_MODEL.md` (Document node + relationships defined, marked FUTURE where not implemented), `docs/PHASE_PLAN.md` (Phase 3 Document slice status).

---

## 3. Implementation & Tasks Layer

These files drive actual development tasks and Codex sessions.

- **docs/FIRST_3_TASKS_FOR_CODEX.md**  
  Safe initial tasks to bootstrap Claims API and UI (good starting point for new Codex sessions).

- **docs/tasks/T2.1a_Claims_API_L0.md**  
  Implement Claims API Layer 0 (skeleton routes + stubbed data).

- **docs/tasks/T2.1b_Claims_API_L1.md**  
  Upgrade Claims API to Layer 1 (real Neo4j-backed list, happy path).

- **docs/tasks/T2.2a_Claims_UI_L0.md**  
  Implement Claims UI Layer 0 (skeleton page with stubbed service).

- **(Future)** `docs/tasks/T2.1c_*`, `T2.1d_*`, `T2.2b_*`  
  Higher-layer tasks for validation, relationships, analysis, and polish.

- **docs/tasks/T3.3_Document_Integration.md**  
  Document slice integration (L1) docs update and E2E verification; see `docs/TASK_TRACKER.md` Phase 3 entries for status.

You can add more `docs/tasks/*.md` files for each new Task ID and Layer.

---

## 4. Meta & Index Layer

These documents describe the documentation system itself and release history.

- **docs/DOCUMENTATION_INDEX.md** (this file)  
  High-level map of all docs and how to use them.

- **docs/RELEASE_NOTES.md**  
  Version history and tags, especially layer-aware tags like `v0.2.0-claims-L0`.

- **(Optional)** `docs/STYLE_GUIDE.md`, `docs/GLOSSARY.md`  
  Coding style, naming conventions, and domain terminology.

---

## 5. How to Use This System (Humans)

1. When starting work:
   - Read `WORKFLOW.md` to remind yourself of the lifecycle.
   - Open `TASK_TRACKER.md` to choose the next Task ID + Layer.

2. For design and implementation:
   - Use `ARCHITECTURE.md`, `API_DESIGN.md`, `DATA_MODEL.md`.

3. For specific implementation steps:
   - Open the task file in `docs/tasks/` for your Task ID (e.g., `T2.1a_Claims_API_L0.md`).

4. When done:
   - Update `TASK_TRACKER.md` and, if needed, `PHASE_PLAN.md`.
   - Consider updating `RELEASE_NOTES.md` and tagging the repo.

---

## 6. How to Use This System (Codex)

Before Codex makes any changes, it must:

1. Read:
   - `AGENTS.md`
   - `docs/WORKFLOW.md`
   - `docs/DEV_ONBOARDING.md`
   - `docs/TASK_TRACKER.md`
   - `docs/ARCHITECTURE.md`
   - `docs/API_DESIGN.md`
   - `docs/DATA_MODEL.md`
   - The specific `docs/tasks/<TaskID>.md` file

2. Ask for:
   - Confirmation of Task ID
   - Layer (L0–L3)
   - Branch name to use

3. Work only within:
   - Assigned agent persona scope
   - Assigned Task ID + Layer
   - Files listed in the task file

---

# End of DOCUMENTATION_INDEX.md
