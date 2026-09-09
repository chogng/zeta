use super::*;
use crossterm::event::KeyModifiers;
use std::collections::BTreeSet;
use github::IssueIdentity;
use github::IssueWorkItem;

fn key(panel: &mut Panel, code: KeyCode) -> Outcome {
    panel.handle_key(KeyEvent::new(code, KeyModifiers::NONE))
}
fn workflow() -> Workflow {
    Workflow {
        repository: IssueRepositoryIdentity {
            host: "github.com".into(),
            node_id: "repo".into(),
            owner: "team".into(),
            name: "project".into(),
        },
        revision: 7,
        settings: IssueWorkflow::default(),
        default_branch: "develop".into(),
        labels: Vec::new(),
        assignees: vec!["tester".into()],
    }
}
fn plan() -> IssueAssignmentPlan {
    let settings = workflow();
    IssueAssignmentPlan {
        repository: settings.repository,
        workflow: settings.settings,
        config_revision: 7,
        model: None,
        planning_tokens: 0,
        base_commit: "a".repeat(40),
        target_branch: "develop".into(),
        items: [3, 5]
            .into_iter()
            .map(|number| IssueWorkItem {
                id: format!("item-{number}"),
                issues: vec![IssueIdentity {
                    node_id: format!("node-{number}"),
                    number,
                    title: format!("Fix {number}"),
                    updated_at: "now".into(),
                    material_digest: "digest".into(),
                }],
                objective: format!("Fix {number}"),
                acceptance_conditions: vec!["Affected behavior passes".into()],
                scope: Default::default(),
                dependencies: if number == 5 {
                    BTreeSet::from(["item-3".into()])
                } else {
                    BTreeSet::new()
                },
                agent: String::new(),
            })
            .collect(),
    }
}

fn snapshot(panel: &Panel, name: &str) {
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;
    let mut terminal = Terminal::new(TestBackend::new(100, 28)).unwrap();
    terminal
        .draw(|frame| panel.draw(frame, frame.area(), crate::render::test_context()))
        .unwrap();
    let buffer = terminal.backend().buffer();
    let text = (0..28)
        .map(|y| {
            (0..100)
                .map(|x| buffer[(x, y)].symbol())
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n");
    insta::assert_snapshot!(name, text);
}

#[test]
fn workflow_edit_preserves_repository_revision_and_other_fields() {
    let mut panel = Panel::default();
    panel.open(&Request::Workflow);
    panel.update(Ok(Reply::Workflow(workflow())));
    for _ in 0..5 {
        key(&mut panel, KeyCode::Down);
    }
    key(&mut panel, KeyCode::Enter);
    panel.paste("tester".into());
    key(&mut panel, KeyCode::Enter);
    let Outcome::Request(Request::SaveWorkflow {
        workflow: edited, ..
    }) = key(&mut panel, KeyCode::Char('s'))
    else {
        panic!("save workflow");
    };
    let mut expected = workflow();
    expected.settings.assignee = "tester".into();
    assert_eq!(edited, expected);
    snapshot(&panel, "issue_workflow_saving");
}

#[test]
fn a_cyclic_plan_edit_keeps_the_last_valid_plan() {
    let original = plan();
    let mut panel = Panel::default();
    panel.open(&Request::Plan {
        numbers: vec![3, 5],
        mode: PlanMode::Distributed,
    });
    panel.update(Ok(Reply::Plan(original.clone())));
    key(&mut panel, KeyCode::Enter);
    for _ in 0..3 {
        key(&mut panel, KeyCode::Down);
    }
    key(&mut panel, KeyCode::Enter);
    panel.paste("item-5".into());
    key(&mut panel, KeyCode::Enter);
    assert!(panel.notice.contains("cycle"));
    let Screen::Plan(Some(current)) = &panel.screen else {
        panic!();
    };
    assert_eq!(current, &original);
    snapshot(&panel, "issue_plan_rejects_cycle");
}

#[test]
fn retrying_a_failed_start_reuses_its_command_identity() {
    let mut panel = Panel::default();
    panel.open(&Request::Plan {
        numbers: vec![3, 5],
        mode: PlanMode::Distributed,
    });
    panel.update(Ok(Reply::Plan(plan())));
    let Outcome::Request(Request::Start { command_id, .. }) = key(&mut panel, KeyCode::Char('s'))
    else {
        panic!();
    };
    assert!(matches!(key(&mut panel, KeyCode::Char('s')), Outcome::None));
    panel.update(Err("connection lost".into()));
    let Outcome::Request(Request::Start {
        command_id: retry, ..
    }) = key(&mut panel, KeyCode::Char('s'))
    else {
        panic!();
    };
    assert_eq!(command_id, retry);
}

#[test]
fn background_refresh_preserves_an_action_error_until_the_next_user_action() {
    let mut panel = Panel::default();
    panel.open(&Request::List);
    panel.update(Err("Target changed; verify again".into()));
    assert_eq!(
        panel.poll(std::time::Instant::now() + std::time::Duration::from_secs(3)),
        Some(Request::List)
    );
    assert!(!panel.busy);
    panel.update(Ok(Reply::Assignments(Vec::new())));
    assert_eq!(panel.notice, "Target changed; verify again");
    key(&mut panel, KeyCode::Char('r'));
    assert_eq!(panel.notice, "Loading...");
}

#[test]
fn error_details_scroll_without_closing_the_assignment_list() {
    let mut panel = Panel::default();
    panel.open(&Request::List);
    panel.update(Err(
        "A long validation failure that must remain readable".into()
    ));
    key(&mut panel, KeyCode::Char('i'));
    key(&mut panel, KeyCode::PageDown);
    assert!(panel.details);
    assert_eq!(panel.detail_scroll, 10);
    assert!(
        panel
            .poll(std::time::Instant::now() + std::time::Duration::from_secs(5))
            .is_none()
    );
    key(&mut panel, KeyCode::Esc);
    assert!(panel.is_open());
    assert!(!panel.details);
    assert!(panel.notice.contains("validation failure"));
}
