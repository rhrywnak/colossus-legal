//! The prep page's two composed count lines (task R3).
//!
//! Split out of `rehearsal_render` when that module crossed the 300-line limit,
//! and the seam is a real one rather than an arithmetic convenience: everything
//! here turns NUMBERS INTO SENTENCES, and everything left there turns stored
//! records into blocks. These three functions share one property that nothing
//! else in the render does — they are the page's only prose that has to be
//! grammatical at ONE.
//!
//! ## Why the plural forms are rows and not a template system
//!
//! The general singular/plural renderer was deferred out of .391, and this page
//! could not wait for it: it OPENS with the count line, and "They said it 1
//! times" on the surface a witness preps from is the kind of slip a reader stops
//! trusting the whole page over. So each count-bearing clause carries both forms
//! as its own settings row and [`pick_form`] chooses — the narrow version of the
//! same idea, and the rows a general system would eventually read.

use crate::domain::settings::Settings;
use crate::domain::wording_templates::render;
use crate::dto::rehearsal::RehearsalInstance;

/// The prep page's count line, in plain words and grammatically correct at ONE.
///
/// "They said it 5 times, in 3 documents, from December 2009 through October
/// 2015 — every one is below, with your answer under it."
///
/// ## Domain note: why this is composed here rather than by a template system
///
/// The general singular/plural template system was deferred out of .391, and this
/// page cannot wait for it: it opens with this sentence, and "They said it 1
/// times" on the surface a witness preps from is the kind of thing a reader stops
/// trusting the whole page over. So the two count-bearing clauses carry BOTH forms
/// as their own settings rows and this function picks, which is the narrow version
/// of the same idea — and the rows a general system would eventually read.
///
/// ## Why the date clause can be absent
///
/// 57% of this case's evidence carries no date at all (measured on DEV). A range
/// needs two endpoints; with none, the clause is OMITTED rather than rendered
/// with an invented span or an em dash. A sentence that quietly drops a clause it
/// has no facts for is honest; one that says "from — through —" is not.
///
/// When every dated instance shares one date the clause says "on <date>" rather
/// than "from X through X", which is a sentence nobody would write by hand.
pub(crate) fn plain_count_line(
    instances: &[RehearsalInstance],
    documents: usize,
    settings: &Settings,
) -> Option<String> {
    if instances.is_empty() {
        return None;
    }
    let w = &settings.rehearsal_wording;

    let times = pick_form(instances.len(), &w.count_times_one, &w.count_times_many);
    let docs = pick_form(documents, &w.count_documents_one, &w.count_documents_many);

    // The endpoints come from the list, which `walk_instances` has already sorted
    // oldest-first with the undated at the end — so the first dated row is the
    // earliest and the last dated row is the latest. Re-deriving them here would
    // be a second ordering free to disagree with the one on screen.
    let dated: Vec<&str> = instances
        .iter()
        .filter_map(|i| i.when.as_deref())
        .filter(|d| !d.trim().is_empty())
        .collect();

    let span = match (dated.first(), dated.last()) {
        (Some(first), Some(last)) if first == last => {
            Some(render(&w.count_span_one_date, &[("date", first)]))
        }
        (Some(first), Some(last)) => Some(render(
            &w.count_span_range,
            &[("from", first), ("through", last)],
        )),
        _ => None,
    };

    Some(render(
        &w.count_line_template,
        &[
            ("times", times.as_str()),
            ("documents", docs.as_str()),
            // An absent span leaves the slot empty rather than the sentence
            // half-built; the stored template puts the clause between commas so
            // an empty one closes up cleanly.
            ("span", span.unwrap_or_default().as_str()),
        ],
    ))
}

/// "5 of 5 answered" — or "3 of 5 answered — 2 to prepare" when work remains.
///
/// ## Domain note: the second clause is the point of the line
///
/// A witness reading "3 of 5 answered" has to do the subtraction to learn what is
/// left, and the number they are subtracting toward is the one they came for. The
/// preparing clause says it outright, and it is ABSENT when nothing is
/// outstanding — "5 of 5 answered — 0 to prepare" reads as a to-do list with an
/// empty item on it.
pub(crate) fn answered_line(
    instances: &[RehearsalInstance],
    settings: &Settings,
) -> Option<String> {
    if instances.is_empty() {
        return None;
    }
    let w = &settings.rehearsal_wording;
    let answered = instances.iter().filter(|i| i.answer.is_some()).count();
    let total = instances.len();
    let remaining = total - answered;

    let template = if remaining == 0 {
        &w.answered_line_all
    } else {
        &w.answered_line_some
    };
    Some(render(
        template,
        &[
            ("answered", answered.to_string().as_str()),
            ("total", total.to_string().as_str()),
            ("remaining", remaining.to_string().as_str()),
        ],
    ))
}

/// One or many, with the count already in the stored form.
///
/// Both forms are rows, so "1 time" / "5 times" are Roman's to word — including
/// in a language where the split is not at one. This only chooses.
fn pick_form(n: usize, one: &str, many: &str) -> String {
    let template = if n == 1 { one } else { many };
    render(template, &[("n", n.to_string().as_str())])
}

#[cfg(test)]
#[path = "rehearsal_count_tests.rs"]
mod tests;
