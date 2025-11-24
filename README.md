# Colossus-Legal

Colossus-Legal is a case-focused legal analysis system designed to ingest documents, extract claims and evidence, 
build a Neo4j knowledge graph, and produce analysis and reports. The project follows the same architecture style 
as Colossus-AI but is implemented as a clean standalone vertical.

This repository contains:

- **Backend**: Rust + Axum 0.7 backend API server  
- **Frontend**: React + Vite + TypeScript UI  
- **Neo4j Integration**: Uses the shared Neo4j instance on your homelab  
- **Documentation**: Architecture, API specs, task tracker, and prompts in `docs/` and `prompts/`

---

# ⚠️ Environment Variable Warning: Escaping `$` Characters

Some shells and environments on this system treat the `$` character inside `.env` files as a variable expansion 
indicator — even though `.env` files normally treat `$` literally.

This can cause environment variables like:

```
NEO4J_PASSWORD=Drwho2010$
```

to incorrectly load as:

```
NEO4J_PASSWORD=Drwho2010
```

because `$` is interpreted as the beginning of a shell variable.

### ✅ Correct way to include `$` in `.env` on this system

Escape it:

```
NEO4J_PASSWORD=Drwho2010\$
```

### Symptoms of missing `$`

If `$` is not escaped, you may see:

- Backend startup panic:
  ```
  Neo4j connectivity check failed: The client is unauthorized due to authentication failure.
  ```
- `dbg!(&config)` output missing the `$`
- Login failures when connecting via Neo4rs

### Applies to:

- Neo4j passwords  
- API keys containing `$`  
- Any secret values in `.env` with `$`

### Recommended practice:

- Always escape `$` as `\$` in `.env`
- Update `.env.example` to show correct escaping
- Verify credentials with:
  ```
  cypher-shell -a bolt://10.10.100.50:7687 -u neo4j -p 'Drwho2010$'
  ```

---

# Project Structure

```
colossus-legal/
  backend/        # Rust Axum server
  frontend/       # React + Vite UI
  docs/           # Architecture docs, API specs, task tracker
  prompts/        # LLM prompt templates
  scripts/        # Dev helper scripts (bootstrap, etc.)
```

---

# Backend (Rust + Axum)

The backend:

- Loads configuration via `dotenvy::dotenv()`
- Connects to Neo4j at startup
- Performs a health ping (`RETURN 1`)
- Hosts API endpoints for:
  - `/health`
  - `/api/status`
  - CRUD endpoints for claims, documents, evidence, people, hearings, decisions (Phase 1 stubs)

Backend server runs on `BACKEND_PORT` (default `3403`).

Run backend:

```
cd backend
cargo run
```

---

# Frontend (React + Vite)

Frontend exposes a minimal shell UI:

- Navigation bar
- Status panel that checks backend `/api/status`
- Placeholder pages for all core entities

Run frontend:

```
cd frontend
npm install
npm run dev
```

Default dev server port: `5473`

---

# Development Workflow (Architects + Codex)

You and ChatGPT act as **Architect + PM**.  
Codex CLI acts as the **Engineer**.

Documents used:

- `docs/ARCHITECTURE.md`
- `docs/API_DESIGN.md`
- `docs/DATA_MODEL.md`
- `docs/TASK_TRACKER.md`
- `CODEX.md`

Codex tasks follow the format in `CODEX.md` and update the task tracker after completing each task.

---

# Running With Docker Compose

```
docker compose up --build
```

Backend and frontend will run inside containers using shared Neo4j credentials.

---

# License

Private personal project. No public license.
