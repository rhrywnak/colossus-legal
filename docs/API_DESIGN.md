# Colossus-Legal – API Design

This document specifies the REST API for Colossus-Legal.

Base URL (dev):

- Backend: `http://localhost:3403`
- Frontend: configured via `VITE_API_URL`

All responses are JSON and use standard HTTP status codes.

---

## 1. Health & Status

### 1.1 `GET /health`

**Purpose:** Simple health probe (used by load balancers and monitoring).

- **Request:** no body.
- **Response:**
  - `200 OK` on success
  - Body: `"OK"` or small status text.

### 1.2 `GET /api/status`

**Purpose:** Frontend status panel.

**Response:**

```json
{
  "app": "colossus-legal-backend",
  "version": "0.1.0",
  "status": "ok"
}
```

Possible `status` values:
- `"ok"` – backend healthy and ready.
- `"degraded"` – backend up but external dependencies partially failing.
- `"error"` – backend is running but cannot serve traffic reliably.

---

## 2. Claims API (v1 – planned)

The Claims API is the first full domain surface we will build.

### 2.1 Data Shapes

**Claim DTO (response)**

```json
{
  "id": "uuid-or-string",
  "title": "Claim title",
  "description": "Optional detailed text",
  "status": "open | closed | refuted | pending"
}
```

**Create Claim DTO (request)**

```json
{
  "title": "Required title",
  "description": "Optional description"
}
```

Additional fields (linking to Document/Person) will be added in later phases.

---

### 📘 Tutorial: Designing and Implementing `GET /claims`

1. **Define the DTO**  
   - Add a `ClaimDto` to `backend/src/dto/claim.rs` that matches the response shape above.
   - Ensure it is `Serialize`.

2. **Define the handler signature**  
   In `backend/src/api/claims.rs`:

   ```rust
   pub async fn list_claims(
       State(state): State<AppState>,
   ) -> Result<Json<Vec<ClaimDto>>, StatusCode> { /* ... */ }
   ```

3. **Define repository method contract**  
   In `backend/src/repositories/claim_repository.rs`:

   ```rust
   pub async fn list_claims(&self) -> Result<Vec<Claim>, RepoError>;
   ```

4. **Implement repository using Neo4j**  
   Use a basic Cypher query, see `DATA_MODEL.md` for node labels.

   ```rust
   let mut result = self
       .graph
       .execute(neo4rs::query("MATCH (c:Claim) RETURN c"))
       .await?;
   ```

5. **Map domain → DTO**  
   Back in the handler, map `Vec<Claim>` into `Vec<ClaimDto>` and return `Json`.

6. **Update router**  
   In `api::router()` (see `ARCHITECTURE.md`):

   ```rust
   .route("/claims", get(claims::list_claims))
   ```

7. **Test the endpoint**  

   ```bash
   cargo check --manifest-path backend/Cargo.toml
   cargo run --manifest-path backend/Cargo.toml
   curl http://localhost:3403/claims
   ```

These steps are the pattern for all list endpoints going forward.

---

### 2.2 Endpoints

#### `GET /claims`

- **Description:** List all claims (later: pageable / filterable).
- **Response:**
  - `200 OK`
  - Body: `Claim[]`

#### `GET /claims/{id}`

- **Description:** Get a single claim by ID.
- **Response:**
  - `200 OK` – claim found
  - `404 Not Found` – if claim does not exist

#### `POST /claims`

- **Description:** Create a new claim.
- **Request Body:** Create Claim DTO.
- **Response:**
  - `201 Created`
  - Body: `Claim` (including generated id and timestamps)
  - `400 Bad Request` – invalid input

#### `PUT /claims/{id}`

- **Description:** Update an existing claim.
- **Request Body:** same shape as Create Claim DTO (or a partial update DTO).
- **Response:**
  - `200 OK` – updated claim
  - `400 Bad Request`
  - `404 Not Found`

#### `DELETE /claims/{id}`

- **Description:** Soft delete or hard delete (to be decided).
- **Response:**
  - `204 No Content` if delete succeeded
  - `404 Not Found` if claim not found

---

## 3. Future Resource APIs

These will mirror the Claims pattern:

### 3.1 Documents

- `GET /documents`
- `GET /documents/{id}`
- `POST /documents`
- `PUT /documents/{id}`
- `DELETE /documents/{id}`

### 3.2 Evidence

- `GET /evidence`
- `GET /evidence/{id}`
- `POST /evidence`
- …

### 3.3 People

- `GET /people`
- `GET /people/{id}`
- `POST /people`
- …

### 3.4 Hearings

- `GET /hearings`
- `GET /hearings/{id}`
- …

### 3.5 Decisions

- `GET /decisions`
- `GET /decisions/{id}`
- …

---

## 4. Relationship & Analysis APIs (Later Phases)

Once core CRUD APIs exist, we add:

### 4.1 Relationship APIs

- `POST /claims/{claim_id}/appears-in/{document_id}`
- `POST /claims/{claim_id}/relies-on/{evidence_id}`
- `POST /evidence/{evidence_id}/presented-at/{hearing_id}`
- `POST /decisions/{decision_id}/decides/{claim_id}`
- `POST /decisions/{decision_id}/refutes/{claim_id}`
- `POST /decisions/{decision_id}/ignores/{claim_id}`

These endpoints translate directly into Neo4j relationships.

### 4.2 Analysis Endpoints

Examples:

- `GET /analysis/paths/{claim_id}`
  - Returns paths from a claim to evidence, documents, decisions.

- `GET /analysis/refuted-claims`
  - List claims with REFUTES relationships.

- `GET /analysis/timeline`
  - Returns time-ordered events (claims, evidence, hearings, decisions).

---

## 5. Error Handling

- Use structured JSON errors:
  - `400 Bad Request` – validation/input errors.
  - `404 Not Found` – missing resources.
  - `500 Internal Server Error` – unexpected failure.

Example error response:

```json
{
  "error": "validation_error",
  "message": "title must not be empty",
  "details": {
    "field": "title"
  }
}
```

---

## 6. Auth (Future)

For now, Colossus-Legal is a trusted local app (2–3 users, homelab).  
Auth will be added later:

- API keys or simple token-based auth.
- Possibly reusing patterns from colossus-ai.

---

# End of API_DESIGN.md
