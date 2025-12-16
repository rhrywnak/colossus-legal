//! Prompt construction logic for the document processor.
//!
//! For now, this is a very simple templating helper that:
//! - takes a prompt template
//! - injects {DOCUMENT_NAME} and {DOCUMENT_TEXT}


/// Build the final prompt string given a template, document name, and text.
///
/// This simply replaces the placeholders:
/// - {DOCUMENT_NAME}
/// - {DOCUMENT_TEXT}
pub fn build_prompt(prompt_template: &str, document_name: &str, text: &str) -> String {
    prompt_template
        .replace("{DOCUMENT_TEXT}", text)
        .replace("{DOCUMENT_NAME}", document_name)
}
