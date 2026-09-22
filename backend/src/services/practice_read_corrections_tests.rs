// Tests for `services::practice_read_corrections` — the prompt file, cut in two.
//
// The disk tests read the SHIPPED files, for the Rule 21 reason: the marker and
// the section names are a contract between this code and a markdown file, and
// only reading the file proves both sides still say the same thing.

use super::*;

fn template(name: &str) -> String {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    std::fs::read_to_string(root.join("extraction_templates").join(name))
        .unwrap_or_else(|e| panic!("{name} is on disk: {e}"))
}

/// The v5 file carries every correction, and the system half carries none.
///
/// The second half is the audience boundary: a correction sent as a STANDING
/// instruction would tell the model about a fault it has not made.
#[test]
fn the_v5_prompt_file_carries_every_correction() {
    let file = template("practice_read_prompt_v5.md");
    let prompt = split_prompt(&file).expect("v5 is complete");

    let corrections = prompt.corrections.expect("v5 carries the marker");
    assert!(corrections.key_in_prose.contains("{token}"));
    assert!(corrections.key_in_prose.contains("{part}"));
    assert!(corrections.unknown_key.contains("{key}"));
    assert!(corrections.resend.contains("{reply}"));
    assert!(corrections.resend.contains("{correction}"));
    assert!(!corrections.unparseable.is_empty());
    assert!(!corrections.nothing_said.is_empty());

    assert!(!prompt.system.contains(CORRECTIONS_MARKER));
    for section in CORRECTION_SECTIONS {
        assert!(
            !prompt
                .system
                .contains(&format!("{SECTION_PREFIX}{section}")),
            "{section} leaked into the system prompt"
        );
    }
    assert!(
        prompt
            .system
            .ends_with("Reply now with the JSON object, and nothing else."),
        "the system prompt still ends on its instruction"
    );
}

/// v5's system half never names a key in an EXAMPLE of prose.
///
/// The GO's correction: the examples use neutral placeholders, never a real
/// case document. The keys may appear only where the prompt describes what it
/// is sending (`P1` `P2` `P3`) and in the `keys` example — both of which this
/// test allows by line.
#[test]
fn v5_examples_name_no_key_and_no_real_document() {
    let system = split_prompt(&template("practice_read_prompt_v5.md"))
        .expect("v5 is complete")
        .system;
    for line in system.lines() {
        let describes_the_payload = line.contains("`P1` `P2` `P3`")
            || line.contains("`R1` `R2` `R3`")
            || line.contains("(`S1`)")
            || line.contains("\"keys\":");
        if !describes_the_payload {
            assert_eq!(
                crate::services::practice_read_parse::find_key_token(
                    line,
                    crate::services::practice_read_parse::KEY_FAMILIES,
                ),
                None,
                "a key in a v5 prose line: {line}"
            );
        }
    }
    assert!(
        !system.contains("SSA"),
        "no real case document in an example"
    );
}

/// A file with no marker is v4: all of it is the system prompt, no corrections.
///
/// This is the one-UPDATE rollback the migration promises, and it must keep
/// working exactly as before v2.2.1.
#[test]
fn a_file_without_the_marker_is_all_system_prompt() {
    let file = template("practice_read_prompt_v4.md");
    let prompt = split_prompt(&file).expect("v4 has no marker, which is legitimate");
    assert_eq!(prompt.system, file);
    assert_eq!(prompt.corrections, None);
}

/// The marker with a section missing is a broken deploy, refused BY NAME.
#[test]
fn a_marker_with_a_missing_section_is_refused_by_name() {
    let file = template("practice_read_prompt_v5.md");
    let broken = file.replace("### unknown_key", "### unknown_keyz");
    let error = split_prompt(&broken).expect_err("a section is missing");
    assert!(error.contains("### unknown_key"), "{error}");
    assert!(error.contains("practice_read_prompt_file"), "{error}");
}

/// A blank section is as missing as an absent one.
#[test]
fn a_blank_section_is_refused() {
    let file = format!(
        "system\n{CORRECTIONS_MARKER}\n### key_in_prose\n\n### unknown_key\nx\n\
         ### unparseable\nx\n### nothing_said\nx\n### resend\nx\n"
    );
    let error = split_prompt(&file).expect_err("key_in_prose is blank");
    assert!(error.contains("key_in_prose"), "{error}");
}

/// Only a line that IS the marker cuts; a sentence quoting it does not.
#[test]
fn a_sentence_mentioning_the_marker_does_not_cut_the_prompt() {
    let file = format!("Never write {CORRECTIONS_MARKER} in a reply.\nReply now.");
    let prompt = split_prompt(&file).expect("no marker line");
    assert_eq!(prompt.system, file);
    assert!(prompt.corrections.is_none());
}

fn v5() -> Corrections {
    split_prompt(&template("practice_read_prompt_v5.md"))
        .expect("v5 is complete")
        .corrections
        .expect("v5 carries corrections")
}

/// Each of the four ruled rejections has its own sentence, filled in.
#[test]
fn every_ruled_rejection_gets_its_own_filled_correction() {
    let c = v5();
    let key_in_prose = c
        .correction_for(&ReplyRejection::KeyInProse {
            part: "why".to_string(),
            token: "R2".to_string(),
        })
        .expect("a correction");
    assert!(
        key_in_prose.contains("R2") && key_in_prose.contains("`why`"),
        "{key_in_prose}"
    );
    assert!(
        !key_in_prose.contains('{'),
        "every token filled: {key_in_prose}"
    );

    let unknown = c
        .correction_for(&ReplyRejection::UnknownKey {
            key: "R9".to_string(),
            sent: "R1".to_string(),
        })
        .expect("a correction");
    assert!(
        unknown.contains("R9") && !unknown.contains("{key}"),
        "{unknown}"
    );

    let unparseable = c
        .correction_for(&ReplyRejection::Unparseable {
            detail: "trailing comma".to_string(),
        })
        .expect("a correction");
    let nothing = c
        .correction_for(&ReplyRejection::NothingSaid)
        .expect("a correction");

    let all = [key_in_prose, unknown, unparseable, nothing];
    for (i, a) in all.iter().enumerate() {
        for b in &all[i + 1..] {
            assert_ne!(a, b, "two rejections share one correction");
        }
    }
}

/// An empty reply has nothing to send back: the plain re-request.
#[test]
fn an_empty_reply_has_no_correction() {
    assert_eq!(v5().correction_for(&ReplyRejection::Empty), None);
}

/// Attempt 2 keeps the whole original message, then the reply, then the fix.
#[test]
fn the_resend_keeps_the_payload_and_carries_the_reply_and_the_correction() {
    let message = v5().resend("THE PAYLOAD", "{\"why\": \"R2 says\"}", "Use words.");
    assert!(message.starts_with("THE PAYLOAD\n\n"), "{message}");
    let reply_at = message
        .find("{\"why\": \"R2 says\"}")
        .expect("the reply is sent back");
    let fix_at = message.find("Use words.").expect("the correction is sent");
    assert!(
        reply_at < fix_at,
        "reply first, then what was wrong with it"
    );
    assert!(!message.contains("{reply}") && !message.contains("{correction}"));
}

/// A reply that happens to contain `{correction}` is not spliced into.
#[test]
fn a_reply_containing_a_token_is_sent_back_verbatim() {
    let message = v5().resend("P", "odd {correction} text", "FIX");
    assert!(message.contains("odd {correction} text"), "{message}");
    assert_eq!(message.matches("FIX").count(), 1, "{message}");
}
