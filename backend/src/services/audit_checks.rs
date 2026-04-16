//! Individual audit health check functions.
//! Each check queries one data source; the handler orchestrates them concurrently.

use neo4rs::{query, Graph};
use serde::Serialize;
use std::path::PathBuf;

fn error_check(name: &str, msg: String) -> AuditCheck {
    AuditCheck {
        name: name.into(),
        status: "fail".into(),
        message: msg,
        details: vec![],
    }
}

#[derive(Debug, Serialize)]
pub struct AuditCheck {
    pub name: String,
    pub status: String, // "pass", "warn", "fail"
    pub message: String,
    pub details: Vec<AuditIssue>,
}

#[derive(Debug, Serialize)]
pub struct AuditIssue {
    pub severity: String, // "critical", "high", "low"
    pub resource_type: String,
    pub resource_id: String,
    pub description: String,
}

/// Check 1: PDF ↔ Document Node Match
pub async fn check_pdf_match(graph: &Graph, storage_path: &str) -> AuditCheck {
    let mut issues = Vec::new();
    let mut total = 0usize;

    let result = graph
        .execute(query(
            "MATCH (d:Document) RETURN d.id AS id, d.title AS title, d.file_path AS file_path",
        ))
        .await;

    let mut rows = match result {
        Ok(r) => r,
        Err(e) => return error_check("pdf_match", format!("Neo4j query failed: {e}")),
    };

    while let Ok(Some(row)) = rows.next().await {
        total += 1;
        let id: String = row.get("id").unwrap_or_default();
        let file_path: Option<String> = row.get("file_path").ok();

        match file_path {
            None => {
                issues.push(AuditIssue {
                    severity: "low".into(),
                    resource_type: "document".into(),
                    resource_id: id,
                    description: "No file_path set".into(),
                });
            }
            Some(fp) if !fp.is_empty() => {
                let full: PathBuf = [storage_path, &fp].iter().collect();
                if !full.exists() {
                    issues.push(AuditIssue {
                        severity: "critical".into(),
                        resource_type: "document".into(),
                        resource_id: id,
                        description: format!("PDF not found: {fp}"),
                    });
                }
            }
            _ => {}
        }
    }

    let status = if issues.iter().any(|i| i.severity == "critical") {
        "fail"
    } else if issues.is_empty() {
        "pass"
    } else {
        "warn"
    };

    AuditCheck {
        name: "pdf_match".into(),
        status: status.into(),
        message: format!("{total} documents checked, {} issues", issues.len()),
        details: issues,
    }
}

/// Check 2: Evidence Completeness
pub async fn check_evidence_completeness(graph: &Graph) -> (AuditCheck, usize, usize) {
    let mut issues = Vec::new();
    let mut total = 0usize;
    let mut complete = 0usize;

    let result = graph
        .execute(query(
            "MATCH (e:Evidence)
             OPTIONAL MATCH (e)-[:STATED_BY]->(p)
             OPTIONAL MATCH (e)-[:CONTAINED_IN]->(d)
             RETURN e.id AS id,
                    e.verbatim_quote IS NOT NULL AS has_quote,
                    e.page_number IS NOT NULL AS has_page,
                    p IS NOT NULL AS has_speaker,
                    d IS NOT NULL AS has_document",
        ))
        .await;

    let mut rows = match result {
        Ok(r) => r,
        Err(e) => {
            return (
                error_check("evidence_completeness", format!("Neo4j query failed: {e}")),
                0,
                0,
            )
        }
    };

    while let Ok(Some(row)) = rows.next().await {
        total += 1;
        let id: String = row.get("id").unwrap_or_default();
        let has_quote: bool = row.get("has_quote").unwrap_or(false);
        let has_page: bool = row.get("has_page").unwrap_or(false);
        let has_speaker: bool = row.get("has_speaker").unwrap_or(false);
        let has_document: bool = row.get("has_document").unwrap_or(false);

        let mut item_issues = Vec::new();
        if !has_quote {
            item_issues.push("missing verbatim_quote");
        }
        if !has_page {
            item_issues.push("missing page_number");
        }
        if !has_speaker {
            item_issues.push("no STATED_BY relationship");
        }
        if !has_document {
            item_issues.push("no CONTAINED_IN relationship");
        }

        if item_issues.is_empty() {
            complete += 1;
        } else {
            let severity = if !has_document || !has_speaker {
                "critical"
            } else {
                "high"
            };
            issues.push(AuditIssue {
                severity: severity.into(),
                resource_type: "evidence".into(),
                resource_id: id,
                description: item_issues.join(", "),
            });
        }
    }

    let status = if issues.iter().any(|i| i.severity == "critical") {
        "fail"
    } else if issues.is_empty() {
        "pass"
    } else {
        "warn"
    };

    let check = AuditCheck {
        name: "evidence_completeness".into(),
        status: status.into(),
        message: format!(
            "{complete}/{total} evidence items complete, {} issues",
            issues.len()
        ),
        details: issues,
    };
    (check, total, complete)
}

/// Check 3: Neo4j ↔ Qdrant Reconciliation
pub async fn check_qdrant_reconciliation(
    graph: &Graph,
    http_client: &reqwest::Client,
    qdrant_url: &str,
) -> (AuditCheck, usize) {
    // Get all Evidence IDs from Neo4j
    let neo4j_ids: Vec<String> = match graph
        .execute(query("MATCH (e:Evidence) RETURN e.id AS id"))
        .await
    {
        Ok(mut rows) => {
            let mut ids = Vec::new();
            while let Ok(Some(row)) = rows.next().await {
                if let Ok(id) = row.get::<String>("id") {
                    ids.push(id);
                }
            }
            ids
        }
        Err(e) => {
            return (
                error_check("qdrant_reconciliation", format!("Neo4j query failed: {e}")),
                0,
            )
        }
    };

    // Get all point node_ids from Qdrant via scroll API
    let scroll_url = format!(
        "{}/collections/colossus_evidence/points/scroll",
        qdrant_url.trim_end_matches('/')
    );
    let qdrant_ids = fetch_qdrant_node_ids(http_client, &scroll_url).await;

    let qdrant_point_count = qdrant_ids.len();
    let neo4j_set: std::collections::HashSet<&str> = neo4j_ids.iter().map(|s| s.as_str()).collect();
    let qdrant_set: std::collections::HashSet<&str> =
        qdrant_ids.iter().map(|s| s.as_str()).collect();

    let mut issues = Vec::new();

    // Evidence in Neo4j but not Qdrant
    for id in neo4j_set.difference(&qdrant_set) {
        issues.push(AuditIssue {
            severity: "high".into(),
            resource_type: "evidence".into(),
            resource_id: id.to_string(),
            description: "In Neo4j but not indexed in Qdrant".into(),
        });
    }

    // Points in Qdrant but not Neo4j
    for id in qdrant_set.difference(&neo4j_set) {
        issues.push(AuditIssue {
            severity: "high".into(),
            resource_type: "qdrant_point".into(),
            resource_id: id.to_string(),
            description: "In Qdrant but no matching Neo4j node".into(),
        });
    }

    let status = if issues.is_empty() { "pass" } else { "warn" };

    let check = AuditCheck {
        name: "qdrant_reconciliation".into(),
        status: status.into(),
        message: format!(
            "{} Neo4j evidence, {} Qdrant points, {} mismatches",
            neo4j_ids.len(),
            qdrant_point_count,
            issues.len()
        ),
        details: issues,
    };
    (check, qdrant_point_count)
}

/// Check 4: Orphaned Nodes
pub async fn check_orphaned_nodes(graph: &Graph) -> AuditCheck {
    let mut issues = Vec::new();

    let result = graph
        .execute(query(
            "MATCH (n) WHERE NOT (n)--()
             RETURN labels(n) AS labels, n.id AS id, n.title AS title",
        ))
        .await;

    let mut rows = match result {
        Ok(r) => r,
        Err(e) => return error_check("orphaned_nodes", format!("Neo4j query failed: {e}")),
    };

    while let Ok(Some(row)) = rows.next().await {
        let labels: Vec<String> = row.get("labels").unwrap_or_default();
        let id: String = row.get("id").unwrap_or_else(|_| "unknown".into());
        let title: Option<String> = row.get("title").ok();
        let label_str = labels.join(", ");

        issues.push(AuditIssue {
            severity: "low".into(),
            resource_type: label_str,
            resource_id: id,
            description: format!(
                "Orphaned node{}",
                title.map(|t| format!(": {t}")).unwrap_or_default()
            ),
        });
    }

    let status = if issues.is_empty() { "pass" } else { "warn" };

    AuditCheck {
        name: "orphaned_nodes".into(),
        status: status.into(),
        message: format!("{} orphaned nodes found", issues.len()),
        details: issues,
    }
}

/// Fetch all node_id values from Qdrant's scroll API.
async fn fetch_qdrant_node_ids(client: &reqwest::Client, url: &str) -> Vec<String> {
    let resp = client
        .post(url)
        .json(&serde_json::json!({ "limit": 10000, "with_payload": ["node_id"] }))
        .timeout(std::time::Duration::from_secs(5))
        .send()
        .await;
    let body = match resp {
        Ok(r) if r.status().is_success() => r.json::<serde_json::Value>().await.ok(),
        _ => None,
    };
    body.and_then(|b| {
        b["result"]["points"].as_array().map(|pts| {
            pts.iter()
                .filter_map(|p| p["payload"]["node_id"].as_str().map(|s| s.to_string()))
                .collect()
        })
    })
    .unwrap_or_default()
}
