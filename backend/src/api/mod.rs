use axum::{
    extract::{FromRef, Path, State},
    http::StatusCode,
    routing::get,
    Json, Router,
};

use crate::state::AppState;
use crate::models::{Claim, Decision, Document, Evidence, Hearing, Person};
use hyper::http::StatusCode;
use chrono::NaiveDate;
use std::{
    collections::HashMap,
    sync::{Arc, OnceLock},
};
use tokio::sync::Mutex;

use crate::{
    dto::{
        ClaimCreateRequest, ClaimUpdateRequest, DecisionCreateRequest, DecisionUpdateRequest,
        DocumentCreateRequest, DocumentUpdateRequest, EvidenceCreateRequest, EvidenceUpdateRequest,
        HearingCreateRequest, HearingUpdateRequest, PersonCreateRequest, PersonUpdateRequest,
    },
    models::{Claim, Decision, Document, Evidence, Hearing, Person},
    state::AppState as GlobalAppState,
};



static MEMORY_STATE: OnceLock<AppState> = OnceLock::new();

impl FromRef<GlobalAppState> for AppState {
    fn from_ref(_state: &GlobalAppState) -> AppState {
        MEMORY_STATE.get_or_init(AppState::default).clone()
    }
}

pub fn router() -> Router<GlobalAppState> {
    Router::<GlobalAppState>::new()
        .route("/claims", get(list_claims).post(create_claim))
        .route(
            "/claims/:id",
            get(get_claim).put(update_claim).delete(delete_claim),
        )
        .route("/documents", get(list_documents).post(create_document))
        .route(
            "/documents/:id",
            get(get_document)
                .put(update_document)
                .delete(delete_document),
        )
        .route("/evidence", get(list_evidence).post(create_evidence))
        .route(
            "/evidence/:id",
            get(get_evidence)
                .put(update_evidence)
                .delete(delete_evidence),
        )
        .route("/people", get(list_people).post(create_person))
        .route(
            "/people/:id",
            get(get_person).put(update_person).delete(delete_person),
        )
        .route("/hearings", get(list_hearings).post(create_hearing))
        .route(
            "/hearings/:id",
            get(get_hearing)
                .put(update_hearing)
                .delete(delete_hearing),
        )
        .route("/decisions", get(list_decisions).post(create_decision))
        .route(
            "/decisions/:id",
            get(get_decision)
                .put(update_decision)
                .delete(delete_decision),
        )
}

// Claims
pub async fn list_claims(
    State(state): State<AppState>,
) -> Result<Json<Vec<Claim>>, (StatusCode, String)> {
    let repo = ClaimRepository::new(state.graph.clone());

    match repo.list_claims().await {
        Ok(claims) => Ok(Json(claims)),
        Err(err) => {
            // Optionally log: tracing::error!("Failed to list claims: {:?}", err);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to list claims".to_string(),
            ))
        }
    }
}


async fn get_claim(State(state): State<AppState>, Path(id): Path<String>) -> Json<Claim> {
    let data = state.claims.lock().await;
    if let Some(existing) = data.get(&id) {
        Json(existing.clone())
    } else {
        Json(Claim {
            id,
            text: "Sample Claim".to_string(),
            made_by: None,
            first_made: None,
            category: None,
            verified: None,
        })
    }
}

async fn create_claim(
    State(state): State<AppState>,
    Json(payload): Json<ClaimCreateRequest>,
) -> (StatusCode, Json<Claim>) {
    let first_made = parse_naive_date(&payload.first_made);
    let mut store = state.claims.lock().await;
    let id = format!("claim-{}", store.len() + 1);
    let claim = Claim {
        id: id.clone(),
        text: payload.text,
        made_by: payload.made_by,
        first_made,
        category: payload.category,
        verified: payload.verified,
    };
    store.insert(id.clone(), claim.clone());
    (StatusCode::CREATED, Json(claim))
}

async fn update_claim(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(payload): Json<ClaimUpdateRequest>,
) -> Json<Claim> {
    let mut data = state.claims.lock().await;
    let updated = data.entry(id.clone()).or_insert(Claim {
        id: id.clone(),
        text: "Updated Claim".to_string(),
        made_by: None,
        first_made: None,
        category: None,
        verified: None,
    });

    if let Some(text) = payload.text {
        updated.text = text;
    }
    if let Some(made_by) = payload.made_by {
        updated.made_by = Some(made_by);
    }
    if let Some(first_made) = parse_naive_date(&payload.first_made) {
        updated.first_made = Some(first_made);
    }
    if let Some(category) = payload.category {
        updated.category = Some(category);
    }
    if let Some(verified) = payload.verified {
        updated.verified = Some(verified);
    }

    Json(updated.clone())
}

async fn delete_claim(State(state): State<AppState>, Path(id): Path<String>) -> StatusCode {
    state.claims.lock().await.remove(&id);
    StatusCode::NO_CONTENT
}

fn parse_naive_date(input: &Option<String>) -> Option<NaiveDate> {
    input
        .as_deref()
        .and_then(|value| NaiveDate::parse_from_str(value, "%Y-%m-%d").ok())
}

// Documents
async fn list_documents(State(state): State<AppState>) -> Json<Vec<Document>> {
    let data = state.documents.lock().await;
    Json(data.values().cloned().collect())
}

async fn get_document(State(state): State<AppState>, Path(id): Path<String>) -> Json<Document> {
    let data = state.documents.lock().await;
    if let Some(existing) = data.get(&id) {
        Json(existing.clone())
    } else {
        Json(Document {
            id,
            title: "Sample Document".to_string(),
            doc_type: None,
            description: None,
            file_path: None,
            uploaded_at: None,
            related_claim_id: None,
            source_url: None,
        })
    }
}

async fn create_document(
    State(state): State<AppState>,
    Json(payload): Json<DocumentCreateRequest>,
) -> (StatusCode, Json<Document>) {
    let id = payload.id.unwrap_or_else(|| "generated-document-id".to_string());
    let document = Document {
        id: id.clone(),
        title: payload.title,
        doc_type: payload.doc_type,
        description: payload.description,
        file_path: payload.file_path,
        uploaded_at: payload.uploaded_at,
        related_claim_id: payload.related_claim_id,
        source_url: payload.source_url,
    };
    state
        .documents
        .lock()
        .await
        .insert(id.clone(), document.clone());
    (StatusCode::CREATED, Json(document))
}

async fn update_document(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(payload): Json<DocumentUpdateRequest>,
) -> Json<Document> {
    let mut data = state.documents.lock().await;
    let updated = data.entry(id.clone()).or_insert(Document {
        id: id.clone(),
        title: "Updated Document".to_string(),
        doc_type: None,
        description: None,
        file_path: None,
        uploaded_at: None,
        related_claim_id: None,
        source_url: None,
    });

    if let Some(title) = payload.title {
        updated.title = title;
    }
    if let Some(doc_type) = payload.doc_type {
        updated.doc_type = Some(doc_type);
    }
    if let Some(description) = payload.description {
        updated.description = Some(description);
    }
    if let Some(file_path) = payload.file_path {
        updated.file_path = Some(file_path);
    }
    if let Some(uploaded_at) = payload.uploaded_at {
        updated.uploaded_at = Some(uploaded_at);
    }
    if let Some(related_claim_id) = payload.related_claim_id {
        updated.related_claim_id = Some(related_claim_id);
    }
    if let Some(source_url) = payload.source_url {
        updated.source_url = Some(source_url);
    }

    Json(updated.clone())
}

async fn delete_document(State(state): State<AppState>, Path(id): Path<String>) -> StatusCode {
    state.documents.lock().await.remove(&id);
    StatusCode::NO_CONTENT
}

// Evidence
async fn list_evidence(State(state): State<AppState>) -> Json<Vec<Evidence>> {
    let data = state.evidence.lock().await;
    Json(data.values().cloned().collect())
}

async fn get_evidence(State(state): State<AppState>, Path(id): Path<String>) -> Json<Evidence> {
    let data = state.evidence.lock().await;
    if let Some(existing) = data.get(&id) {
        Json(existing.clone())
    } else {
        Json(Evidence {
            id,
            claim_id: None,
            document_id: None,
            description: None,
            evidence_type: None,
            is_supporting: None,
            collected_on: None,
        })
    }
}

async fn create_evidence(
    State(state): State<AppState>,
    Json(payload): Json<EvidenceCreateRequest>,
) -> (StatusCode, Json<Evidence>) {
    let id = payload.id.unwrap_or_else(|| "generated-evidence-id".to_string());
    let evidence = Evidence {
        id: id.clone(),
        claim_id: payload.claim_id,
        document_id: payload.document_id,
        description: payload.description,
        evidence_type: payload.evidence_type,
        is_supporting: payload.is_supporting,
        collected_on: payload.collected_on,
    };
    state
        .evidence
        .lock()
        .await
        .insert(id.clone(), evidence.clone());
    (StatusCode::CREATED, Json(evidence))
}

async fn update_evidence(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(payload): Json<EvidenceUpdateRequest>,
) -> Json<Evidence> {
    let mut data = state.evidence.lock().await;
    let updated = data.entry(id.clone()).or_insert(Evidence {
        id: id.clone(),
        claim_id: None,
        document_id: None,
        description: None,
        evidence_type: None,
        is_supporting: None,
        collected_on: None,
    });

    if let Some(claim_id) = payload.claim_id {
        updated.claim_id = Some(claim_id);
    }
    if let Some(document_id) = payload.document_id {
        updated.document_id = Some(document_id);
    }
    if let Some(description) = payload.description {
        updated.description = Some(description);
    }
    if let Some(evidence_type) = payload.evidence_type {
        updated.evidence_type = Some(evidence_type);
    }
    if let Some(is_supporting) = payload.is_supporting {
        updated.is_supporting = Some(is_supporting);
    }
    if let Some(collected_on) = payload.collected_on {
        updated.collected_on = Some(collected_on);
    }

    Json(updated.clone())
}

async fn delete_evidence(State(state): State<AppState>, Path(id): Path<String>) -> StatusCode {
    state.evidence.lock().await.remove(&id);
    StatusCode::NO_CONTENT
}

// People
async fn list_people(State(state): State<AppState>) -> Json<Vec<Person>> {
    let data = state.people.lock().await;
    Json(data.values().cloned().collect())
}

async fn get_person(State(state): State<AppState>, Path(id): Path<String>) -> Json<Person> {
    let data = state.people.lock().await;
    if let Some(existing) = data.get(&id) {
        Json(existing.clone())
    } else {
        Json(Person {
            id,
            name: "Sample Person".to_string(),
            role: None,
            email: None,
            phone: None,
            affiliation: None,
            notes: None,
            date_of_birth: None,
        })
    }
}

async fn create_person(
    State(state): State<AppState>,
    Json(payload): Json<PersonCreateRequest>,
) -> (StatusCode, Json<Person>) {
    let id = payload.id.unwrap_or_else(|| "generated-person-id".to_string());
    let person = Person {
        id: id.clone(),
        name: payload.name,
        role: payload.role,
        email: payload.email,
        phone: payload.phone,
        affiliation: payload.affiliation,
        notes: payload.notes,
        date_of_birth: payload.date_of_birth,
    };
    state.people.lock().await.insert(id.clone(), person.clone());
    (StatusCode::CREATED, Json(person))
}

async fn update_person(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(payload): Json<PersonUpdateRequest>,
) -> Json<Person> {
    let mut data = state.people.lock().await;
    let updated = data.entry(id.clone()).or_insert(Person {
        id: id.clone(),
        name: "Updated Person".to_string(),
        role: None,
        email: None,
        phone: None,
        affiliation: None,
        notes: None,
        date_of_birth: None,
    });

    if let Some(name) = payload.name {
        updated.name = name;
    }
    if let Some(role) = payload.role {
        updated.role = Some(role);
    }
    if let Some(email) = payload.email {
        updated.email = Some(email);
    }
    if let Some(phone) = payload.phone {
        updated.phone = Some(phone);
    }
    if let Some(affiliation) = payload.affiliation {
        updated.affiliation = Some(affiliation);
    }
    if let Some(notes) = payload.notes {
        updated.notes = Some(notes);
    }
    if let Some(dob) = payload.date_of_birth {
        updated.date_of_birth = Some(dob);
    }

    Json(updated.clone())
}

async fn delete_person(State(state): State<AppState>, Path(id): Path<String>) -> StatusCode {
    state.people.lock().await.remove(&id);
    StatusCode::NO_CONTENT
}

// Hearings
async fn list_hearings(State(state): State<AppState>) -> Json<Vec<Hearing>> {
    let data = state.hearings.lock().await;
    Json(data.values().cloned().collect())
}

async fn get_hearing(State(state): State<AppState>, Path(id): Path<String>) -> Json<Hearing> {
    let data = state.hearings.lock().await;
    if let Some(existing) = data.get(&id) {
        Json(existing.clone())
    } else {
        Json(Hearing {
            id,
            claim_id: None,
            scheduled_date: None,
            location: None,
            judge: None,
            notes: None,
            outcome_summary: None,
        })
    }
}

async fn create_hearing(
    State(state): State<AppState>,
    Json(payload): Json<HearingCreateRequest>,
) -> (StatusCode, Json<Hearing>) {
    let id = payload.id.unwrap_or_else(|| "generated-hearing-id".to_string());
    let hearing = Hearing {
        id: id.clone(),
        claim_id: payload.claim_id,
        scheduled_date: payload.scheduled_date,
        location: payload.location,
        judge: payload.judge,
        notes: payload.notes,
        outcome_summary: payload.outcome_summary,
    };
    state
        .hearings
        .lock()
        .await
        .insert(id.clone(), hearing.clone());
    (StatusCode::CREATED, Json(hearing))
}

async fn update_hearing(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(payload): Json<HearingUpdateRequest>,
) -> Json<Hearing> {
    let mut data = state.hearings.lock().await;
    let updated = data.entry(id.clone()).or_insert(Hearing {
        id: id.clone(),
        claim_id: None,
        scheduled_date: None,
        location: None,
        judge: None,
        notes: None,
        outcome_summary: None,
    });

    if let Some(claim_id) = payload.claim_id {
        updated.claim_id = Some(claim_id);
    }
    if let Some(scheduled_date) = payload.scheduled_date {
        updated.scheduled_date = Some(scheduled_date);
    }
    if let Some(location) = payload.location {
        updated.location = Some(location);
    }
    if let Some(judge) = payload.judge {
        updated.judge = Some(judge);
    }
    if let Some(notes) = payload.notes {
        updated.notes = Some(notes);
    }
    if let Some(outcome_summary) = payload.outcome_summary {
        updated.outcome_summary = Some(outcome_summary);
    }

    Json(updated.clone())
}

async fn delete_hearing(State(state): State<AppState>, Path(id): Path<String>) -> StatusCode {
    state.hearings.lock().await.remove(&id);
    StatusCode::NO_CONTENT
}

// Decisions
async fn list_decisions(State(state): State<AppState>) -> Json<Vec<Decision>> {
    let data = state.decisions.lock().await;
    Json(data.values().cloned().collect())
}

async fn get_decision(State(state): State<AppState>, Path(id): Path<String>) -> Json<Decision> {
    let data = state.decisions.lock().await;
    if let Some(existing) = data.get(&id) {
        Json(existing.clone())
    } else {
        Json(Decision {
            id,
            claim_id: None,
            decided_on: None,
            outcome: None,
            judge: None,
            summary: None,
            notes: None,
        })
    }
}

async fn create_decision(
    State(state): State<AppState>,
    Json(payload): Json<DecisionCreateRequest>,
) -> (StatusCode, Json<Decision>) {
    let id = payload.id.unwrap_or_else(|| "generated-decision-id".to_string());
    let decision = Decision {
        id: id.clone(),
        claim_id: payload.claim_id,
        decided_on: payload.decided_on,
        outcome: payload.outcome,
        judge: payload.judge,
        summary: payload.summary,
        notes: payload.notes,
    };
    state
        .decisions
        .lock()
        .await
        .insert(id.clone(), decision.clone());
    (StatusCode::CREATED, Json(decision))
}

async fn update_decision(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(payload): Json<DecisionUpdateRequest>,
) -> Json<Decision> {
    let mut data = state.decisions.lock().await;
    let updated = data.entry(id.clone()).or_insert(Decision {
        id: id.clone(),
        claim_id: None,
        decided_on: None,
        outcome: None,
        judge: None,
        summary: None,
        notes: None,
    });

    if let Some(claim_id) = payload.claim_id {
        updated.claim_id = Some(claim_id);
    }
    if let Some(decided_on) = payload.decided_on {
        updated.decided_on = Some(decided_on);
    }
    if let Some(outcome) = payload.outcome {
        updated.outcome = Some(outcome);
    }
    if let Some(judge) = payload.judge {
        updated.judge = Some(judge);
    }
    if let Some(summary) = payload.summary {
        updated.summary = Some(summary);
    }
    if let Some(notes) = payload.notes {
        updated.notes = Some(notes);
    }

    Json(updated.clone())
}

async fn delete_decision(State(state): State<AppState>, Path(id): Path<String>) -> StatusCode {
    state.decisions.lock().await.remove(&id);
    StatusCode::NO_CONTENT
}
