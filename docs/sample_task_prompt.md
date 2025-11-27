/task new
title: T1.3 – Implement basic CRUD HTTP API (in-memory stubs)
instructions:
  This task corresponds to **T1.3** in docs/TASK_TRACKER.md and must follow the conventions in CODEX.md.

  Goal:
    Expose basic CRUD HTTP endpoints for the six core entities (Claim, Document, Evidence, Person, Hearing, Decision)
    using the domain models and DTOs created in T1.2. For Phase 1, persistence can be in-memory or stubbed; Neo4j
    integration will come later in Phase 2.

  Context:
    - Repo: colossus-legal
    - Backend root: ./backend
    - Models and DTOs live under:
        - backend/src/models/
        - backend/src/dto/
    - Existing server:
        - Axum 0.7 + tokio
        - main.rs currently sets up /health and /api/status
    - API shapes are described in docs/API_DESIGN.md (use as guidance, but stubbing is allowed at this phase).

  Requirements:

    1) Create an API module:
       - Add backend/src/api/mod.rs
       - Optionally split by entity (e.g. claims.rs, documents.rs), but not required.
       - The API module should define the routes and handlers for:
         - Claims
         - Documents
         - Evidence
         - People
         - Hearings
         - Decisions

    2) Define routes:
       For each entity, provide at least:
         - GET /<entity>           -> list all (e.g. /claims)
         - GET /<entity>/{id}      -> get by id
         - POST /<entity>          -> create
         - PUT /<entity>/{id}      -> update
         - DELETE /<entity>/{id}   -> delete

       Entities and base paths:
         - Claim:      /claims
         - Document:   /documents
         - Evidence:   /evidence
         - Person:     /people
         - Hearing:    /hearings
         - Decision:   /decisions

       For now, we do NOT need full query params or filtering; just basic routes.

    3) Handlers (Phase 1 behavior):
       - Use the DTOs from T1.2 as request bodies for POST/PUT.
       - Return the model types (or lists of them) as JSON.
       - For Phase 1, it is acceptable to implement one of:
         a) In-memory storage using static Mutex<HashMap<String, T>> per entity, OR
         b) Pure stub responses:
            - GET /...: return an empty Vec<T> or a small hard-coded example
            - GET /.../{id}: return a dummy entity with the provided id
            - POST /...: echo back a created entity with a generated id
            - PUT /.../{id}: echo back an updated entity
            - DELETE /.../{id}: return 204 No Content or a small JSON {"status":"deleted"}

       - The key requirement is that:
         - The routes exist
         - They compile
         - They return structurally correct JSON responses
         - They match the DTO/model shapes defined in T1.2.

    4) Wire routes into main.rs:
       - In backend/src/main.rs, import the API module and extend the Router to include all CRUD routes.
       - Keep /health and /api/status still working and unchanged in behavior.
       - Ensure CORS setup (added earlier) still applies to these new routes.

    5) Status codes:
       - GET list: 200 OK
       - GET by id: 200 OK
       - POST: 201 Created
       - PUT: 200 OK
       - DELETE: 204 No Content (or 200 with a small status object)

       These can be implemented using Axum’s `Json` + `StatusCode`.

  Task Tracker Update:
    - Edit docs/TASK_TRACKER.md
    - In the Phase 1 section, mark **T1.3 – Basic CRUD Endpoints (Stubbed)** as DONE, including date (2025-11-22).
    - Optionally add a brief note that these are in-memory or stubbed for Phase 1 and will be backed by Neo4j in Phase 2.
    - Do not modify other tasks.

files:
  - backend/src/lib.rs
  - backend/src/api/mod.rs
  - backend/src/api/*.rs
  - backend/src/main.rs
  - docs/TASK_TRACKER.md

