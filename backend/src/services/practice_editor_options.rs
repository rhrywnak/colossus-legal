//! What the add form may attach a new question to.
//!
//! Task B1: "attach to one of THIS scenario's ruled instances or points via a
//! picker — the receipt/pair/source line inherit from it — or `no receipt`".
//!
//! ## Why the labels are composed on the SERVER
//!
//! Two reasons, and the second decides it. The templates are stored rows, and
//! the browser holds none. But also: an instance's label is the best sentence
//! this scenario has for "the thing at hearing p. 33", and where that sentence
//! comes from is a judgement — see [`instance_label`]. A client composing it
//! would be making that judgement in a place nobody reviews.
//!
//! ## Nothing here reads the graph
//!
//! The instances are counted from the questions already bound to them, and
//! labelled from the deck's own authored strings. Design §5, unchanged: the tool
//! reads the scenario record, the deck and the log.

use crate::domain::settings::Settings;
use crate::domain::wording_templates::render;
use crate::dto::practice_review::PracticeAttachOptionDto;
use crate::repositories::pipeline_repository::practice::{
    PracticePointRecord, PracticeQuestionRecord,
};

/// How many ruled instances this scenario's deck knows about.
///
/// ## Domain note: counted from the DECK, not read from the record
///
/// The seed refuses a deck naming an instance the scenario does not have, so the
/// highest `source_index` any cross question binds to is a lower bound this
/// service can trust without a second read. It is a LOWER bound and stated as
/// one: a scenario with six ruled instances whose deck only uses four offers
/// four here, and the honest consequence is that the add form cannot attach to
/// the two nobody has written a question about yet. Widening that means reading
/// `scenario_human_facts`, which is the seed's job and not this page's.
fn instances_in_deck(deck: &[PracticeQuestionRecord]) -> usize {
    deck.iter().filter(|q| q.source_kind == "instance").count()
}

/// The sentence that names one instance in the picker.
///
/// The `source_line` of the question bound to it — the exhibit as Marie would
/// name it aloud, which is exactly the register a picker needs. A question with
/// none contributes its position and nothing else, and the template's `{text}`
/// renders empty rather than the machine cutting a phrase out of the receipt
/// paragraph (the same choice, and the same reason, as the "I'd point to…"
/// list).
fn instance_label(deck: &[PracticeQuestionRecord], index: i32) -> String {
    deck.iter()
        .filter(|q| q.source_kind == "instance")
        .nth(usize::try_from(index - 1).unwrap_or(usize::MAX))
        .and_then(|q| q.source_line.clone())
        .unwrap_or_default()
}

/// Every instance and point a new question may attach to, already labelled.
///
/// `no receipt` is NOT in this list: it is the picker's own stored default and
/// the absence of a choice, not a thing to attach to. Putting it here would make
/// it a source with an index, which is exactly what it is not.
pub fn attach_options(
    settings: &Settings,
    deck: &[PracticeQuestionRecord],
    points: &[PracticePointRecord],
) -> Vec<PracticeAttachOptionDto> {
    let w = &settings.practice_wording.editor;
    let mut out = Vec::new();

    for index in 1..=instances_in_deck(deck) {
        let index = i32::try_from(index).unwrap_or(i32::MAX);
        out.push(PracticeAttachOptionDto {
            source_kind: "instance".to_string(),
            source_index: index,
            label: render(
                &w.editor_attach_instance_template,
                &[
                    ("n", &index.to_string()),
                    ("text", &instance_label(deck, index)),
                ],
            ),
        });
    }

    for point in points {
        out.push(PracticeAttachOptionDto {
            source_kind: "point".to_string(),
            source_index: point.position,
            label: render(
                &w.editor_attach_point_template,
                &[("n", &point.position.to_string()), ("text", &point.text)],
            ),
        });
    }
    out
}

#[cfg(test)]
#[path = "practice_editor_options_tests.rs"]
mod tests;
