// Tests for `services::ai_jobs_compose` — the panel as PROD's store would draw
// it, and each of board 4's worst states.

use super::*;
use crate::services::ai_jobs_options::tests::{model, prod_models};

pub(crate) fn row(key: &str, value: &str, job: &str, role: &str) -> AiJobSettingRow {
    AiJobSettingRow {
        key: key.into(),
        value: value.into(),
        ai_job: job.into(),
        ai_role: role.into(),
        updated_by: "migration".into(),
        updated_at: Utc::now(),
    }
}

/// PROD's nine job rows on 2026-09-24.
pub(crate) fn prod_rows() -> Vec<AiJobSettingRow> {
    vec![
        row(
            "question_chat_model",
            "claude-opus-5-5",
            "discuss_chat",
            "model",
        ),
        row(
            "question_chat_prompt_file",
            "question_chat_prompt_v1.md",
            "discuss_chat",
            "instructions",
        ),
        row("question_chat_effort", "medium", "discuss_effort", "effort"),
        row(
            "chat_case_narrative_file",
            "case_narrative_v1.md",
            "case_story",
            "instructions",
        ),
        row(
            "practice_read_model",
            "claude-opus-5",
            "answer_analysis",
            "model",
        ),
        row(
            "practice_read_prompt_file",
            "practice_read_prompt_v5.md",
            "answer_analysis",
            "instructions",
        ),
        row("chat_default_model", "claude-opus-5", "chat_page", "model"),
        row(
            "theme_scan_default_model",
            "claude-opus-5",
            "theme_scan",
            "model",
        ),
        row(
            "theme_scan_prompt_file",
            "theme_scan_prompt_v3.md",
            "theme_scan",
            "instructions",
        ),
    ]
}

pub(crate) fn change(key: &str, old: &str, new: &str, hour: u32) -> AppSettingChangeRecord {
    use chrono::TimeZone;
    AppSettingChangeRecord {
        key: key.into(),
        old_value: old.into(),
        new_value: new.into(),
        actor: "roman".into(),
        at: Utc
            .with_ymd_and_hms(2026, 9, 23, hour, 0, 0)
            .single()
            .expect("a valid moment"),
    }
}

/// PROD's history, most recent first.
fn prod_changes() -> Vec<AppSettingChangeRecord> {
    vec![
        change("question_chat_effort", "high", "medium", 19),
        change(
            "question_chat_model",
            "claude-opus-5",
            "claude-opus-5-5",
            18,
        ),
    ]
}

fn files() -> Vec<String> {
    [
        "case_narrative_v1.md",
        "practice_read_prompt_v5.md",
        "question_chat_prompt_v1.md",
        "theme_scan_prompt_v3.md",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect()
}

struct Fixture {
    rows: Vec<AiJobSettingRow>,
    changes: Vec<AppSettingChangeRecord>,
    models: Vec<LlmModelRecord>,
    scan: HashSet<String>,
    files: Vec<String>,
    fixed: Option<&'static str>,
    cost: Option<f64>,
    wording: AdminWording,
    loose: Vec<(String, String)>,
}

impl Fixture {
    fn prod() -> Self {
        Fixture {
            rows: prod_rows(),
            changes: prod_changes(),
            models: prod_models(),
            scan: ["claude-opus-5", "claude-opus-5-5", "qwen"]
                .iter()
                .map(|s| s.to_string())
                .collect(),
            files: files(),
            fixed: Some("qwen"),
            cost: Some(2.951),
            wording: AdminWording::for_test(),
            loose: Vec::new(),
        }
    }

    fn panel(&self) -> AiJobsPanelDto {
        let when = |at: DateTime<Utc>| at.format("%H:%M").to_string();
        let who = |login: &str| format!("<{login}>");
        compose_panel(&PanelInputs {
            rows: &self.rows,
            changes: &self.changes,
            models: &self.models,
            scan_ids: &self.scan,
            files: &self.files,
            fixed_scan_model: self.fixed,
            reload_cost: self.cost,
            wording: &self.wording,
            loose: &self.loose,
            when: &when,
            who: &who,
        })
        .expect("PROD's rows compose")
    }
}

fn job<'a>(panel: &'a AiJobsPanelDto, token: &str) -> &'a AiJobDto {
    panel
        .jobs
        .iter()
        .find(|j| j.job == token)
        .expect("the job is on the panel")
}

fn shown(c: &AiJobControlDto) -> String {
    match &c.body {
        AiJobControlBody::Choose { current_label, .. } => current_label.clone(),
        AiJobControlBody::Fixed { text } | AiJobControlBody::Empty { text } => text.clone(),
    }
}

#[test]
fn prods_store_draws_board_one_in_order() {
    let p = Fixture::prod().panel();
    let order: Vec<&str> = p.jobs.iter().map(|j| j.job.as_str()).collect();
    assert_eq!(
        order,
        [
            "discuss_chat",
            "discuss_effort",
            "case_story",
            "answer_analysis",
            "chat_page",
            "theme_scan"
        ]
    );
    let cells: Vec<(String, String)> = p
        .jobs
        .iter()
        .map(|j| (shown(&j.first), shown(&j.instructions)))
        .collect();
    assert_eq!(
        cells,
        [
            (
                "Claude Opus 5.5".into(),
                "question_chat_prompt_v1.md".into()
            ),
            ("Medium".into(), "—".into()),
            ("—".into(), "case_narrative_v1.md".into()),
            ("Claude Opus 5".into(), "practice_read_prompt_v5.md".into()),
            (
                "Claude Opus 5".into(),
                "Built in, not changeable here".into()
            ),
            (
                "Fixed by the server: Qwen3.8 27B".into(),
                "theme_scan_prompt_v3.md".into()
            ),
        ]
    );
    assert!(
        p.jobs.iter().all(|j| j.warnings.is_empty()),
        "PROD is healthy: {:?}",
        p.jobs
    );
}

#[test]
fn the_meta_line_says_who_changed_what_and_the_price() {
    let p = Fixture::prod().panel();
    assert_eq!(
        job(&p, "discuss_chat").meta,
        "Model changed by <roman>, 18:00 · Changing the model or the instructions reloads the \
         case file on the next question (about $2.95)."
    );
    assert_eq!(
        job(&p, "discuss_effort").meta,
        "Changed by <roman>, 19:00 · No reload."
    );
    assert_eq!(
        job(&p, "case_story").meta,
        "Set by the install, never changed · Changing it reloads the case file (about $2.95)."
    );
    assert_eq!(
        job(&p, "answer_analysis").meta,
        "Set by the install, never changed"
    );
    assert_eq!(
        job(&p, "theme_scan").meta,
        "Set in the server's configuration. Change it there, or remove that line to choose here."
    );
}

#[test]
fn an_unknown_price_uses_the_no_price_line() {
    let mut f = Fixture::prod();
    f.cost = None;
    let p = f.panel();
    assert_eq!(
        job(&p, "case_story").meta,
        "Set by the install, never changed · Changing it reloads the case file."
    );
}

#[test]
fn instructions_changed_last_says_instructions() {
    let mut f = Fixture::prod();
    f.changes.insert(
        0,
        change(
            "practice_read_prompt_file",
            "practice_read_prompt_v4.md",
            "practice_read_prompt_v5.md",
            20,
        ),
    );
    let p = f.panel();
    assert_eq!(
        job(&p, "answer_analysis").meta,
        "Instructions changed by <roman>, 20:00"
    );
}

#[test]
fn without_the_server_override_the_scan_row_is_a_dropdown() {
    let mut f = Fixture::prod();
    f.fixed = None;
    let p = f.panel();
    let scan = job(&p, "theme_scan");
    assert_eq!(shown(&scan.first), "Claude Opus 5");
    assert!(matches!(scan.first.body, AiJobControlBody::Choose { .. }));
    assert_eq!(scan.meta, "Set by the install, never changed");
}

#[test]
fn a_switched_off_model_says_what_broke_and_what_to_do() {
    let mut f = Fixture::prod();
    f.models[0] = model("claude-opus-5", "Claude Opus 5", "anthropic", false, true);
    let p = f.panel();
    assert_eq!(
        job(&p, "answer_analysis").warnings,
        [
            "Claude Opus 5 is switched off on Prompt Management → Models, so the answer analysis \
          can't run. Pick another model."
        ]
    );
    assert_eq!(job(&p, "chat_page").warnings.len(), 1);
}

#[test]
fn a_missing_instructions_file_says_what_broke_and_what_to_do() {
    let mut f = Fixture::prod();
    f.files.retain(|name| name != "question_chat_prompt_v1.md");
    let p = f.panel();
    assert_eq!(
        job(&p, "discuss_chat").warnings,
        [
            "question_chat_prompt_v1.md is not in the instructions folder, so the Discuss chat \
          can't run. Pick another version."
        ]
    );
}

#[test]
fn the_fixed_scan_model_is_checked_too() {
    let mut f = Fixture::prod();
    f.scan.remove("qwen");
    let p = f.panel();
    assert_eq!(job(&p, "theme_scan").warnings.len(), 1);
    assert!(job(&p, "theme_scan").warnings[0].starts_with("Qwen3.8 27B is not offered for scans"));
}

#[test]
fn a_job_the_page_does_not_know_is_shown_last_by_its_token() {
    let mut f = Fixture::prod();
    f.rows.push(row(
        "brief_writer_model",
        "claude-opus-5",
        "brief_writer",
        "model",
    ));
    let p = f.panel();
    assert_eq!(
        p.jobs.last().map(|j| j.title.as_str()),
        Some("brief_writer")
    );
}

#[test]
fn a_row_with_an_unknown_role_is_refused_by_name() {
    let mut f = Fixture::prod();
    f.rows
        .push(row("odd_key", "x", "answer_analysis", "temperature"));
    let when = |_: DateTime<Utc>| String::new();
    let who = |_: &str| String::new();
    let err = compose_panel(&PanelInputs {
        rows: &f.rows,
        changes: &f.changes,
        models: &f.models,
        scan_ids: &f.scan,
        files: &f.files,
        fixed_scan_model: f.fixed,
        reload_cost: f.cost,
        wording: &f.wording,
        loose: &f.loose,
        when: &when,
        who: &who,
    })
    .expect_err("an unknown role cannot be drawn");
    assert!(err.to_string().contains("odd_key"), "{err}");
}

#[test]
fn no_sentence_leaves_a_placeholder_unfilled() {
    let p = Fixture::prod().panel();
    for j in &p.jobs {
        for line in [&j.meta, &shown(&j.first), &shown(&j.instructions)] {
            assert!(!line.contains('{'), "{line}");
        }
    }
}

#[test]
fn the_links_point_at_three_targets() {
    let p = Fixture::prod().panel();
    let targets: Vec<&str> = p.links.iter().map(|l| l.target.as_str()).collect();
    assert_eq!(targets, ["models", "files", "data"]);
    assert_eq!(p.links[0].label, "Prompt Management → Models");
}

#[test]
fn dollars_have_two_decimals() {
    assert_eq!(dollars(2.951), "$2.95");
    assert_eq!(dollars(0.07), "$0.07");
}

#[test]
fn a_job_changed_in_the_app_offers_what_it_used_before() {
    let p = Fixture::prod().panel();
    let chat = job(&p, "discuss_chat");
    assert_eq!(chat.was.as_deref(), Some("was Claude Opus 5"));
    assert_eq!(chat.switch_back.as_deref(), Some("Switch back"));
    let effort = job(&p, "discuss_effort");
    assert_eq!(
        effort.was.as_deref(),
        Some("was High"),
        "a level is named in plain words"
    );
}

#[test]
fn a_job_never_changed_here_offers_no_switch_back() {
    let p = Fixture::prod().panel();
    for token in ["case_story", "answer_analysis", "chat_page"] {
        assert_eq!(job(&p, token).was, None, "{token}");
        assert_eq!(job(&p, token).switch_back, None, "{token}");
    }
}

#[test]
fn a_job_the_server_fixes_offers_no_switch_back_even_with_history() {
    let mut f = Fixture::prod();
    f.changes.insert(
        0,
        change(
            "theme_scan_prompt_file",
            "theme_scan_prompt_v2.md",
            "theme_scan_prompt_v3.md",
            20,
        ),
    );
    let p = f.panel();
    assert_eq!(job(&p, "theme_scan").was, None);
}

#[test]
fn a_setting_naming_a_model_outside_every_job_is_listed() {
    let mut f = Fixture::prod();
    f.loose = vec![(
        "The model the brief writer uses.".into(),
        "claude-opus-5".into(),
    )];
    let p = f.panel();
    assert_eq!(
        p.others,
        ["The model the brief writer uses. names Claude Opus 5. It is not on this panel yet."]
    );
    assert_eq!(p.others_title, "Other settings that name a model");
}

#[test]
fn a_job_with_two_rows_of_one_role_is_refused_naming_both() {
    let mut f = Fixture::prod();
    f.rows.push(row(
        "practice_read_model_b",
        "claude-opus-5",
        "answer_analysis",
        "model",
    ));
    let when = |_: DateTime<Utc>| String::new();
    let who = |_: &str| String::new();
    let err = compose_panel(&PanelInputs {
        rows: &f.rows,
        changes: &f.changes,
        models: &f.models,
        scan_ids: &f.scan,
        files: &f.files,
        fixed_scan_model: f.fixed,
        reload_cost: f.cost,
        wording: &f.wording,
        loose: &f.loose,
        when: &when,
        who: &who,
    })
    .expect_err("two model rows for one job cannot be drawn");
    assert!(matches!(err, ComposeError::DuplicateRole { .. }));
    let text = err.to_string();
    assert!(
        text.contains("practice_read_model") && text.contains("practice_read_model_b"),
        "{text}"
    );
}
