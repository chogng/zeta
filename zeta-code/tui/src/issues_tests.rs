use super::*;
use crossterm::event::KeyModifiers;

fn loaded() -> Manager {
    let mut manager = Manager::default();
    let Command::List { generation, .. } = manager.open().unwrap() else {
        panic!();
    };
    manager.update(Event::Listed {
        generation,
        page: 1,
        result: Ok(Page {
            repository: Repository {
                host: "github.com".into(),
                owner: "team".into(),
                name: "repo".into(),
            },
            issues: vec![
                Issue {
                    number: 3,
                    title: "first".into(),
                },
                Issue {
                    number: 5,
                    title: "second".into(),
                },
            ],
            next: None,
        }),
    });
    manager
}

fn key(manager: &mut Manager, code: KeyCode) -> Option<Command> {
    manager.handle_key(KeyEvent::new(code, KeyModifiers::NONE))
}

#[test]
fn multiple_issues_produce_one_creation_command_and_retry_identity() {
    let mut manager = loaded();
    key(&mut manager, KeyCode::Char(' '));
    key(&mut manager, KeyCode::Down);
    key(&mut manager, KeyCode::Char(' '));
    key(&mut manager, KeyCode::Down);
    let command = key(&mut manager, KeyCode::Enter).unwrap();
    let Command::Start {
        generation,
        command_id,
        numbers,
        start,
        ..
    } = command
    else {
        panic!();
    };
    assert_eq!((numbers, start), (vec![3, 5], StartPoint::CurrentBranch));
    assert!(key(&mut manager, KeyCode::Enter).is_none());
    manager.finish_start(generation, Some("disconnected".into()));
    let Command::Start {
        command_id: retry, ..
    } = key(&mut manager, KeyCode::Enter).unwrap()
    else {
        panic!();
    };
    assert_eq!(retry, command_id);
}

#[test]
fn closed_manager_does_not_reopen_when_loading_finishes() {
    let mut manager = Manager::default();
    let Command::List { generation, .. } = manager.open().unwrap() else {
        panic!();
    };
    key(&mut manager, KeyCode::Esc);
    manager.update(Event::Listed {
        generation,
        page: 1,
        result: Err("offline".into()),
    });
    assert!(!manager.is_open());
    assert!(!manager.busy);
    assert_eq!(manager.status, "offline");
}

#[test]
fn empty_selection_does_not_create_a_task() {
    let mut manager = loaded();
    key(&mut manager, KeyCode::End);
    assert!(key(&mut manager, KeyCode::Enter).is_none());
    assert!(!manager.busy);
}

#[test]
fn reopening_for_a_pr_ignores_the_previous_issue_query() {
    let mut manager = Manager::default();
    let Command::List { generation, .. } = manager.open().unwrap() else {
        panic!();
    };
    key(&mut manager, KeyCode::Esc);
    let session_id = zeta_protocol::SessionId::new("issue-session").unwrap();
    let Command::PreviewPr {
        generation: next, ..
    } = manager.open_pr(session_id.clone()).unwrap()
    else {
        panic!();
    };
    manager.update(Event::Listed {
        generation,
        page: 1,
        result: Err("stale error".into()),
    });
    assert_eq!(manager.pr_session, Some(session_id));
    assert!(manager.busy);
    assert_eq!(manager.generation, next);
    assert!(!manager.status.contains("stale error"));
}

#[test]
fn issue_tabs_query_the_selected_state_and_reject_old_results() {
    let mut manager = loaded();
    key(&mut manager, KeyCode::Char(' '));
    manager.next = Some(2);
    let Command::List {
        generation: closed,
        state,
        page,
    } = key(&mut manager, KeyCode::Tab).unwrap()
    else {
        panic!();
    };
    assert_eq!((state, page), (IssueState::Closed, 1));
    assert!(manager.selected.is_empty());
    assert!(manager.issues.is_empty());
    assert_eq!(manager.next, None);
    let Command::List {
        generation: open,
        state,
        page,
    } = key(&mut manager, KeyCode::BackTab).unwrap()
    else {
        panic!();
    };
    assert_eq!((state, page), (IssueState::Open, 1));
    manager.update(Event::Listed {
        generation: closed,
        page: 1,
        result: Err("old closed request".into()),
    });
    assert!(manager.busy);
    assert_eq!(manager.generation, open);
    assert!(!manager.status.contains("old closed"));
    key(&mut manager, KeyCode::Esc);
    assert!(!manager.is_open());
}

#[test]
fn closed_issue_empty_results_retry_and_pagination_keep_the_state() {
    let mut manager = loaded();
    let Command::List { generation, .. } = key(&mut manager, KeyCode::Tab).unwrap() else {
        panic!();
    };
    manager.update(Event::Listed {
        generation,
        page: 1,
        result: Ok(Page {
            repository: manager.repository.clone().unwrap(),
            issues: vec![],
            next: Some(2),
        }),
    });
    assert_eq!(manager.status, "No closed issues");
    key(&mut manager, KeyCode::Down);
    assert!(matches!(
        key(&mut manager, KeyCode::Char('n')),
        Some(Command::List {
            state: IssueState::Closed,
            page: 2,
            ..
        })
    ));
    manager.update(Event::Listed {
        generation: manager.generation,
        page: 2,
        result: Err("offline".into()),
    });
    assert_eq!(manager.status, "offline");
    assert!(matches!(
        key(&mut manager, KeyCode::Char('r')),
        Some(Command::List {
            state: IssueState::Closed,
            page: 1,
            ..
        })
    ));
}

#[test]
fn issue_arrows_reach_list_actions_and_tabs() {
    let mut manager = loaded();
    key(&mut manager, KeyCode::Down);
    key(&mut manager, KeyCode::Down);
    assert_eq!(manager.cursor, 2);
    key(&mut manager, KeyCode::Down);
    assert_eq!(manager.cursor, 3);
    key(&mut manager, KeyCode::Home);
    key(&mut manager, KeyCode::Up);
    assert!(manager.tabs.focused);
    key(&mut manager, KeyCode::Down);
    assert!(!manager.tabs.focused);
    assert!(matches!(
        key(&mut manager, KeyCode::Enter),
        Some(Command::Read { number: 3, .. })
    ));
}

#[test]
fn issue_panel_has_shared_header_tabs_margins_and_selection_styles() {
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;
    let manager = loaded();
    for (width, height) in [(100, 24), (60, 12), (20, 12), (1, 1)] {
        let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
        terminal
            .draw(|frame| manager.draw(frame, frame.area(), crate::render::test_context()))
            .unwrap();
        let buffer = terminal.backend().buffer();
        if width >= 20 {
            let row = |y| {
                (0..width)
                    .map(|x| buffer[(x, y)].symbol())
                    .collect::<String>()
            };
            assert!(row(0).contains("Issues"));
            assert!(row(2).contains("Open"));
            assert!(row(2).contains("Closed"));
            assert!(row(7).starts_with("  > [ ] #3 first"));
            assert_eq!(
                buffer[(2, 7)].fg,
                crate::render::test_context().foreground()
            );
            assert_eq!(buffer[(0, 7)].symbol(), " ");
        }
    }
}

#[test]
fn issue_search_keeps_text_editing_separate_from_tab_navigation() {
    let mut manager = loaded();
    key(&mut manager, KeyCode::Char('/'));
    for character in "first".chars() {
        key(&mut manager, KeyCode::Char(character));
    }
    assert_eq!(
        manager
            .filtered()
            .iter()
            .map(|issue| issue.number)
            .collect::<Vec<_>>(),
        [3]
    );
    key(&mut manager, KeyCode::Left);
    assert_eq!(manager.state(), IssueState::Open);
    assert!(matches!(
        key(&mut manager, KeyCode::Tab),
        Some(Command::List {
            state: IssueState::Closed,
            ..
        })
    ));
    assert!(manager.search.input_active());
    assert!(manager.is_open());
    assert_eq!(manager.search.query(), "first");
    assert_eq!(manager.state(), IssueState::Closed);
}

#[test]
fn tab_does_not_repeat_or_leave_issue_detail_and_pending_creation() {
    use crossterm::event::KeyEventKind;
    let mut manager = loaded();
    for kind in [KeyEventKind::Repeat, KeyEventKind::Release] {
        assert!(
            manager
                .handle_key(KeyEvent::new_with_kind(
                    KeyCode::Tab,
                    KeyModifiers::NONE,
                    kind
                ))
                .is_none()
        );
        assert_eq!(manager.state(), IssueState::Open);
    }
    manager.detail = Some("Issue details".into());
    assert!(key(&mut manager, KeyCode::Tab).is_none());
    assert_eq!(manager.state(), IssueState::Open);
    assert_eq!(manager.detail.as_deref(), Some("Issue details"));
    key(&mut manager, KeyCode::Esc);
    key(&mut manager, KeyCode::Char(' '));
    key(&mut manager, KeyCode::End);
    let Command::Start { generation, .. } = key(&mut manager, KeyCode::Enter).unwrap() else {
        panic!()
    };
    assert!(key(&mut manager, KeyCode::Tab).is_none());
    assert_eq!(manager.generation, generation);
    assert_eq!(manager.state(), IssueState::Open);
    manager.finish_start(generation, Some("retry".into()));
    assert!(matches!(
        key(&mut manager, KeyCode::Tab),
        Some(Command::List {
            state: IssueState::Closed,
            ..
        })
    ));
}
