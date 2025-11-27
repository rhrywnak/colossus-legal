# Colossus-Legal – System Architecture

This document describes the high-level architecture of **Colossus-Legal**:
a small-case legal knowledge-graph and analysis system.

---

## 1. High-Level Overview

Colossus-Legal is a **three-layer** system:

1. **Frontend UI (React + Vite + TS)**  
   - Provides a browser-based interface for viewing and editing case data.  
   - Communicates with the backend over HTTP/JSON.

2. **Backend API (Rust + Axum + Neo4j)**  
   - Exposes REST endpoints for claims, documents, evidence, people, hearings, and decisions.  
   - Implements validation, domain logic, and Neo4j persistence.  

3. **Data Layer (Neo4j Graph)**  
   - Stores the legal graph: who said what, when, where, and how it relates.  
   - Provides flexible queries for analysis and reporting.

External AI/LLM components (Claude / OpenAI / local LLMs) are planned for later phases.

---

## 2. Backend Architecture (Rust / Axum)

### 2.1 Components

- `backend/src/main.rs`
  - Application entry point.
  - Loads configuration from env (`NEO4J_URI`, etc.).
  - Builds `AppState` (holds a Neo4j `Graph` handle plus any shared resources).
  - Constructs the root Axum `Router`:
    - Attaches `/health`, `/api/status`.
    - Merges API subrouters from `backend/src/api/`.

- `backend/src/state.rs`
  - Defines `AppState`.
  - Owns shared dependencies (Neo4j graph, configuration structs, etc.).
  - Passed into handlers via `State<AppState>`.

- `backend/src/api/`
  - Router assembly and HTTP handlers.
  - Each domain (Claims, Documents, Evidence, People, Hearings, Decisions) should live in its own module:
    - `api/claims.rs`
    - `api/documents.rs`
    - etc.
  - Handlers:
    - Parse path/query/body.
    - Call repositories/services.
    - Return DTOs.

- `backend/src/models/`
  - Domain types:
    - `Claim`, `Document`, `Evidence`, `Person`, `Hearing`, `Decision`.
  - Represents the *business* view of entities (not HTTP and not raw Neo4j).

- `backend/src/dto/`
  - HTTP-facing types (request/response payloads).
  - Versions can evolve without changing internal models.

- `backend/src/repositories/`
  - Neo4j access layer.
  - One repository per aggregate:
    - `ClaimRepository`, `DocumentRepository`, etc.
  - Responsibilities:
    - Construct Cypher queries.
    - Map rows to `models::*`.

- `backend/src/neo4j.rs`
  - Creates the Neo4j `Graph` handle from env vars.
  - May contain helpers for running typed queries.

---

### 2.2 Request Flow

1. **HTTP Request** from frontend → `Axum Router`.
2. Router dispatches to matching handler in `backend/src/api/*`.
3. Handler:
   - Extracts input (path/body/query).
   - Uses `State<AppState>` to obtain repos/services.
   - Translates DTO → domain model.
4. Repository issues queries to Neo4j:
   - `Graph::execute(query).await`.
   - Maps rows to domain model.
5. Handler translates domain model → DTO.
6. Axum serializes DTO to JSON → HTTP response.

This separation ensures that:
- Handlers are thin.
- All Neo4j logic is centralized in repositories.
- Domain types are independent from HTTP and persistence details.

---

### 📘 Tutorial: Implementing `GET /claims` End-to-End (Backend)

This is a worked example that matches the architecture described above.

1. **Define DTO (if not already present)**  
   File: `backend/src/dto/claim.rs`

   ```rust
   #[derive(serde::Serialize)]
   pub struct ClaimDto {
       pub id: String,
       pub title: String,
       pub description: Option<String>,
       pub status: String,
   }

   impl From<crate::models::claim::Claim> for ClaimDto {
       fn from(c: crate::models::claim::Claim) -> Self {
           Self {
               id: c.id,
               title: c.title,
               description: c.description,
               status: c.status.to_string(),
           }
       }
   }
   ```

2. **Add repository method**  
   File: `backend/src/repositories/claim_repository.rs`

   ```rust
   impl ClaimRepository {
       pub async fn list_claims(&self) -> Result<Vec<Claim>, RepoError> {
           let mut result = self
               .graph
               .execute(neo4rs::query("MATCH (c:Claim) RETURN c"))
               .await?;

           let mut claims = Vec::new();
           while let Ok(Some(row)) = result.next().await {
               let node: neo4rs::Node = row.get("c")?;
               claims.push(Claim::try_from(node)?);
           }
           Ok(claims)
       }
   }
   ```

3. **Add API handler**  
   File: `backend/src/api/claims.rs`

   ```rust
   pub async fn list_claims(
       State(state): State<AppState>,
   ) -> Result<Json<Vec<ClaimDto>>, StatusCode> {
       let repo = ClaimRepository::new(state.graph.clone());
       let claims = repo.list_claims().await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
       Ok(Json(claims.into_iter().map(ClaimDto::from).collect()))
   }
   ```

4. **Wire into router**  
   File: `backend/src/api/mod.rs`

   ```rust
   pub fn router() -> Router<AppState> {
       Router::new()
           .route("/health", get(health_check))
           .route("/claims", get(claims::list_claims))
   }
   ```

5. **Compile and test**  

   ```bash
   cargo check --manifest-path backend/Cargo.toml
   cargo run --manifest-path backend/Cargo.toml
   curl http://localhost:3403/claims
   ```

This pattern (DTO → repository → handler → router) is the template for all future endpoints.

---

## 3. Frontend Architecture (React / Vite / TypeScript)

### 3.1 Main Pieces

- `frontend/src/main.tsx`
  - Bootstraps React, router, and top-level providers.

- `frontend/src/App.tsx`
  - Global layout (navigation, shell, route outlet).
  - Defines or uses the router for:
    - Dashboard
    - Claims
    - Documents
    - Evidence
    - People
    - Hearings
    - Decisions

- `frontend/src/pages/`
  - Screen-level components:
    - `ClaimsPage`, `ClaimDetailPage`, etc.
  - Each page:
    - Consumes services from `src/services/`.
    - Manages page-local state (filters, sort, etc.).

- `frontend/src/services/`
  - API clients for each resource:
    - `claims.ts` → calls `/claims` endpoints.
    - `status.ts` → calls `/api/status`.
  - Encapsulates fetch logic and error handling.

- `frontend/src/styles/`
  - Global CSS / design system.

---

### 📘 Tutorial: Displaying Claims in the Frontend

1. **Add service function**  
   File: `frontend/src/services/claims.ts`

   ```ts
   const API_BASE = import.meta.env.VITE_API_URL ?? "http://localhost:3403";

   export type ClaimDto = {
     id: string;
     title: string;
     description?: string;
     status: string;
   };

   export async function getClaims(): Promise<ClaimDto[]> {
     const res = await fetch(`${API_BASE}/claims`);
     if (!res.ok) {
       throw new Error(`Failed to fetch claims: ${res.status}`);
     }
     return res.json();
   }
   ```

2. **Create Claims page**  
   File: `frontend/src/pages/ClaimsPage.tsx`

   ```tsx
   import { useEffect, useState } from "react";
   import { getClaims, ClaimDto } from "../services/claims";

   export function ClaimsPage() {
     const [claims, setClaims] = useState<ClaimDto[]>([]);
     const [loading, setLoading] = useState(true);
     const [error, setError] = useState<string | null>(null);

     useEffect(() => {
       getClaims()
         .then((data) => setClaims(data))
         .catch((err) => setError(err.message))
         .finally(() => setLoading(false));
     }, []);

     if (loading) return <div>Loading claims…</div>;
     if (error) return <div>Error: {error}</div>;
     if (claims.length === 0) return <div>No claims yet.</div>;

     return (
       <div>
         <h1>Claims</h1>
         <ul>
           {claims.map((c) => (
             <li key={c.id}>
               <strong>{c.title}</strong> ({c.status})
             </li>
           ))}
         </ul>
       </div>
     );
   }
   ```

3. **Add route**  
   In `App.tsx` (or wherever routes are defined):

   ```tsx
   import { ClaimsPage } from "./pages/ClaimsPage";

   // inside your Router
   <Route path="/claims" element={<ClaimsPage />} />
   ```

4. **Run and verify**

   ```bash
   cd frontend
   npm run dev
   ```

   Open `http://localhost:5173/claims` and confirm the claims list loads.

---

## 4. Data Architecture (Neo4j Graph)

The graph is defined more fully in `DATA_MODEL.md`. At a high level:

- Nodes:
  - `Claim`, `Document`, `Evidence`, `Person`, `Hearing`, `Decision`.
- Relationships:
  - `APPEARS_IN` (Claim → Document)
  - `RELIES_ON` (Claim → Evidence)
  - `PRESENTED_AT` (Evidence → Hearing)
  - `MADE_BY` (Claim → Person)
  - `DECIDES` (Decision → Claim)
  - `REFUTES` / `IGNORES` (Decision/Evidence → Claim)

Neo4j is accessed exclusively through repository modules in `backend/src/repositories/`.

---

## 5. Environments and Config

- Configuration is passed via environment variables:
  - `NEO4J_URI`
  - `NEO4J_USER`
  - `NEO4J_PASSWORD`
  - `BACKEND_PORT`
  - `VITE_API_URL` (frontend)

- Local dev:
  - Backend: `cargo run --manifest-path backend/Cargo.toml`
  - Frontend: `npm run dev` in `frontend/`

- Docker:
  - `docker-compose.yml` will orchestrate:
    - Backend container
    - Frontend container (optional)
    - Neo4j container (or use external Neo4j instance)

---

## 6. Phased Build-Out

This architecture will be realized gradually:

1. **Baseline** – health/status endpoints, minimal API router, simple status UI.
2. **Claims v1** – add Claims endpoints + basic UI.
3. **Graph Core** – expand models/relationships, repository logic.
4. **Document ingestion** – upload + extraction pipeline.
5. **AI assistance** – LLM-based analysis and suggestions.
6. **Reporting & visualization** – dashboards, timelines, graph views.

See `PHASE_PLAN.md` and `TASK_TRACKER.md` for implementation details.

---

# End of ARCHITECTURE.md
