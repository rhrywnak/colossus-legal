//! The tools the question chat's model may call mid-reply.
//!
//! All three answer from the package already gathered for this turn — the same
//! rows, read once — so a tool cannot see a different record from the documents
//! the model is quoting (and a tool call costs no second trip to the store).
//!
//! ## Domain note: why tools, when every document is already in the package
//!
//! The document blocks carry text without page breaks (so citations stay
//! sentence-sized). `get_document` gives the same text PAGE BY PAGE, which is how
//! a witness is examined ("turn to page 3"). `get_answer_history` gives her
//! attempts on their own, when the model wants them without the rest.

use std::sync::Arc;

use colossus_chat::{ChatTool, ToolSpec};
use serde_json::{json, Value};

use crate::services::chat_question_text::{document_date_label, translate_keys, PackagedDocument};

/// Build this turn's tools over the gathered documents and the rendered answers.
pub fn tools(
    documents: Arc<Vec<PackagedDocument>>,
    answer_history: String,
) -> Vec<Arc<dyn ChatTool>> {
    vec![
        Arc::new(ListDocuments(Arc::clone(&documents))),
        Arc::new(GetDocument(documents)),
        Arc::new(GetAnswerHistory(translate_keys(&answer_history))),
    ]
}

struct ListDocuments(Arc<Vec<PackagedDocument>>);
struct GetDocument(Arc<Vec<PackagedDocument>>);
struct GetAnswerHistory(String);

#[async_trait::async_trait]
impl ChatTool for ListDocuments {
    fn name(&self) -> &str {
        "list_documents"
    }
    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: self.name().into(),
            description: "List every document in the case record: its id, name, date and page \
                          count. Call this when you need a document's id for get_document."
                .into(),
            input_schema: json!({"type": "object", "properties": {}, "additionalProperties": false}),
        }
    }
    async fn run(&self, _input: Value) -> Result<String, String> {
        Ok(self
            .0
            .iter()
            .map(|d| {
                let date =
                    document_date_label(d.date).unwrap_or_else(|| "date not recorded".into());
                format!(
                    "{} — {} — {} — {} pages",
                    d.id,
                    d.title,
                    date,
                    d.page_starts.len()
                )
            })
            .collect::<Vec<_>>()
            .join("\n"))
    }
}

#[async_trait::async_trait]
impl ChatTool for GetDocument {
    fn name(&self) -> &str {
        "get_document"
    }
    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: self.name().into(),
            description: "Read one document page by page, optionally only pages first..last. \
                          Call this when you need to say which page something is on. Quote \
                          from the document blocks you were given, so the quotation is checked."
                .into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "id": {"type": "string", "description": "The document id, from list_documents"},
                    "first_page": {"type": "integer", "minimum": 1},
                    "last_page": {"type": "integer", "minimum": 1}
                },
                "required": ["id"],
                "additionalProperties": false
            }),
        }
    }
    async fn run(&self, input: Value) -> Result<String, String> {
        let id = input
            .get("id")
            .and_then(Value::as_str)
            .ok_or("get_document needs an id")?;
        let doc = self
            .0
            .iter()
            .find(|d| d.id == id)
            .ok_or_else(|| format!("no document has the id `{id}` — call list_documents"))?;
        let first = page_arg(&input, "first_page").unwrap_or(i32::MIN);
        let last = page_arg(&input, "last_page").unwrap_or(i32::MAX);
        let pages = pages_of(doc);
        let chosen: Vec<String> = pages
            .into_iter()
            .filter(|(n, _)| (first..=last).contains(n))
            .map(|(n, text)| format!("[Page {n}]\n{text}"))
            .collect();
        if chosen.is_empty() {
            return Err(format!("`{id}` has no pages in that range"));
        }
        Ok(format!("{}\n\n{}", doc.title, chosen.join("\n\n")))
    }
}

#[async_trait::async_trait]
impl ChatTool for GetAnswerHistory {
    fn name(&self) -> &str {
        "get_answer_history"
    }
    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: self.name().into(),
            description: "Every answer the witness has given to THIS question, oldest first, with \
                          the read each one received."
                .into(),
            input_schema: json!({"type": "object", "properties": {}, "additionalProperties": false}),
        }
    }
    async fn run(&self, _input: Value) -> Result<String, String> {
        Ok(self.0.clone())
    }
}

/// A page bound from the model's input. `None` = not given (or not a page number
/// an `i32` can hold) and the range stays open on that side — the tool then
/// returns every page, or names an empty range as an error.
fn page_arg(input: &Value, key: &str) -> Option<i32> {
    input
        .get(key)
        .and_then(Value::as_i64)
        // best-effort: a bound past i32 is no real page; treated as unbounded.
        .and_then(|n| i32::try_from(n).ok())
}

/// Split a packaged document back into its pages, using its page map.
fn pages_of(doc: &PackagedDocument) -> Vec<(i32, String)> {
    let chars: Vec<char> = doc.block.text.chars().collect();
    let mut out = Vec::with_capacity(doc.page_starts.len());
    for (i, (start, page)) in doc.page_starts.iter().enumerate() {
        let end = doc
            .page_starts
            .get(i + 1)
            .map_or(chars.len(), |(next, _)| *next);
        let text: String = chars[*start..end].iter().collect();
        out.push((*page, text.trim_end().to_string()));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repositories::pipeline_repository::chat_discussions::CorpusDocument;
    use crate::services::chat_question_text::package_documents;

    fn docs() -> Arc<Vec<PackagedDocument>> {
        Arc::new(package_documents(
            &[CorpusDocument {
                id: "doc-1".into(),
                title: "THE LETTER".into(),
                document_date: chrono::NaiveDate::from_ymd_opt(2009, 11, 5),
                pages: vec![(1, "First page.".into()), (2, "Second page.".into())],
            }],
            None,
        ))
    }

    #[tokio::test]
    async fn get_document_returns_pages_and_names_a_bad_id() {
        let t = tools(docs(), String::new());
        let get = &t[1];
        let out = get
            .run(json!({"id": "doc-1", "first_page": 2}))
            .await
            .unwrap();
        assert!(out.contains("[Page 2]\nSecond page.") && !out.contains("First page."));
        assert!(get
            .run(json!({"id": "nope"}))
            .await
            .unwrap_err()
            .contains("list_documents"));
        assert!(get.run(json!({})).await.is_err());
    }

    #[tokio::test]
    async fn list_documents_names_id_title_date_and_pages() {
        let out = tools(docs(), String::new())[0]
            .run(json!({}))
            .await
            .unwrap();
        assert_eq!(out, "doc-1 — THE LETTER — November 5, 2009 — 2 pages");
    }

    #[tokio::test]
    async fn answer_history_is_key_free() {
        let out = tools(docs(), "Read: P2 holds".into())[2]
            .run(json!({}))
            .await
            .unwrap();
        assert_eq!(out, "Read: talking point 2 holds");
    }
}
