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
                    labels: Vec::new(),
                    assignees: Vec::new(),
                    number: 3,
                    title: "first".into(),
                },
                Issue {
                    labels: Vec::new(),
                    assignees: Vec::new(),
                    number: 5,
                    title: "second".into(),
                },
            ],
            next: None,
            refresh_after_seconds: Some(600),
            fetched_at: 10,
            freshness: "Updated".into(),
            notice: String::new(),
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
    let command = manager
        .handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::CONTROL))
        .unwrap();
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
    } = manager
        .handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::CONTROL))
        .unwrap()
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
    assert!(
        manager
            .handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::CONTROL))
            .is_none()
    );
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
fn closed_group_loads_lazily_without_removing_open_issues_or_selection() {
    let mut manager = loaded();
    key(&mut manager, KeyCode::Char(' '));
    key(&mut manager, KeyCode::End);
    let Command::List {
        generation,
        state,
        page,
        ..
    } = key(&mut manager, KeyCode::Right).unwrap()
    else {
        panic!()
    };
    assert_eq!((state, page), (IssueState::Closed, 1));
    assert_eq!(manager.board.loaded(), 2);
    assert_eq!(manager.selected, BTreeSet::from([3]));
    key(&mut manager, KeyCode::Esc);
    manager.open();
    manager.update(Event::Listed {
        generation,
        page: 1,
        result: Err("old closed request".into()),
    });
    assert!(!manager.status.contains("old closed"));
}

#[test]
fn closed_issue_empty_results_retry_and_pagination_keep_the_state() {
    let mut manager = loaded();
    key(&mut manager, KeyCode::End);
    let Command::List { generation, .. } = key(&mut manager, KeyCode::Right).unwrap() else {
        panic!();
    };
    manager.update(Event::Listed {
        generation,
        page: 1,
        result: Ok(Page {
            repository: manager.repository.clone().unwrap(),
            issues: vec![],
            next: Some(2),
            refresh_after_seconds: Some(600),
            fetched_at: 10,
            freshness: "Cached".into(),
            notice: String::new(),
        }),
    });
    assert_eq!(manager.board.loaded(), 2);
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
fn group_navigation_and_folding_match_the_sessions_tree() {
    let mut manager = loaded();
    assert_eq!(manager.board.selected, Some(board::Target::Issue(3)));
    key(&mut manager, KeyCode::Up);
    assert_eq!(
        manager.board.selected,
        Some(board::Target::Group(board::Group::Todo))
    );
    key(&mut manager, KeyCode::Left);
    assert!(manager.is_open());
    assert!(manager.board.collapsed(board::Group::Todo));
    key(&mut manager, KeyCode::Right);
    key(&mut manager, KeyCode::Down);
    assert!(matches!(
        key(&mut manager, KeyCode::Enter),
        Some(Command::Read { number: 3, .. })
    ));
}

#[test]
fn grouped_issue_page_uses_shared_margins_and_survives_narrow_terminals() {
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;
    let manager = loaded();
    for (width, height) in [(100, 24), (60, 12), (20, 12), (1, 1)] {
        let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
        terminal
            .draw(|frame| manager.draw(frame, frame.area(), crate::render::test_context()))
            .unwrap();
        let buffer = terminal.backend().buffer();
        let text = (0..height)
            .map(|y| {
                (0..width)
                    .map(|x| buffer[(x, y)].symbol())
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n");
        if width >= 20 {
            insta::assert_snapshot!(format!("issue_groups_{width}x{height}"), text);
            assert!(text.contains("Issues"));
            assert!(text.contains("Todo (2)"));
            assert!(text.contains("Closed (0)"));
            assert!(text.contains("#3"));
            if width >= 60 {
                assert!(text.contains("#3 first"));
                assert_eq!(
                    buffer[(2, 7)].bg,
                    crate::render::test_context().selection_background()
                );
                assert_eq!(
                    buffer[(width - 3, 7)].bg,
                    crate::render::test_context().selection_background()
                );
            }
        }
    }
}

#[test]
fn issue_search_keeps_cursor_keys_inside_the_query() {
    let mut manager = loaded();
    key(&mut manager, KeyCode::Char('/'));
    for character in "first".chars() {
        key(&mut manager, KeyCode::Char(character));
    }
    assert_eq!(
        manager
            .board
            .entries(manager.search.query(), true)
            .iter()
            .map(|entry| entry.issue.number)
            .collect::<Vec<_>>(),
        [3]
    );
    key(&mut manager, KeyCode::Left);
    assert!(manager.is_open());
    key(&mut manager, KeyCode::Esc);
    assert!(manager.search.query().is_empty());
    assert!(manager.is_open());
}

#[test]
fn issue_search_submits_remote_query_cancels_edits_and_keeps_body_matches() {
    let mut manager = loaded();
    key(&mut manager, KeyCode::Char('/'));
    for character in "missing in titles".chars() {
        key(&mut manager, KeyCode::Char(character));
    }
    let Command::List {
        query,
        generation,
        mode,
        page,
        ..
    } = key(&mut manager, KeyCode::Enter).unwrap()
    else {
        panic!();
    };
    assert_eq!(
        (query.as_str(), mode, page),
        ("missing in titles", ListMode::Cached, 1)
    );
    manager.update(Event::Listed {
        generation,
        page,
        result: Ok(Page {
            repository: manager.repository.clone().unwrap(),
            issues: vec![Issue {
                labels: Vec::new(),
                assignees: Vec::new(),
                number: 1005,
                title: "Body contains the terms".into(),
            }],
            next: None,
            refresh_after_seconds: None,
            fetched_at: 10,
            freshness: "Cached".into(),
            notice: "Search truncated".into(),
        }),
    });
    assert_eq!(
        manager.board.entries(manager.search.query(), false)[0]
            .issue
            .number,
        1005
    );
    assert_eq!(manager.status, "Search truncated");
    key(&mut manager, KeyCode::Char('/'));
    key(&mut manager, KeyCode::Char('x'));
    key(&mut manager, KeyCode::Esc);
    assert_eq!(manager.search.query(), "missing in titles");
    assert!(!manager.busy);
}

#[test]
fn issue_refresh_is_visible_idle_only_and_failures_preserve_rows_without_request_storms() {
    let mut manager = loaded();
    manager.overview_at = Instant::now() + Duration::from_secs(10000);
    let deadline = manager.refresh_at.unwrap();
    assert!(
        manager
            .poll_refresh(deadline - Duration::from_secs(1))
            .is_none()
    );
    key(&mut manager, KeyCode::Char('/'));
    assert!(manager.poll_refresh(deadline).is_none());
    key(&mut manager, KeyCode::Esc);
    manager.detail = Some("details".into());
    assert!(manager.poll_refresh(deadline).is_none());
    manager.detail = None;
    key(&mut manager, KeyCode::Esc);
    assert!(manager.poll_refresh(deadline).is_none());
    manager.open = true;
    let Command::List {
        generation, mode, ..
    } = manager.poll_refresh(deadline).unwrap()
    else {
        panic!();
    };
    assert_eq!(mode, ListMode::Auto);
    assert_eq!(manager.board.loaded(), 2);
    assert!(manager.poll_refresh(deadline).is_none());
    manager.update(Event::Listed {
        generation,
        page: 1,
        result: Err("rate limited".into()),
    });
    assert_eq!(manager.board.loaded(), 2);
    assert_eq!(manager.status, "rate limited");
    assert!(
        manager
            .poll_refresh(Instant::now() + Duration::from_secs(30))
            .is_none()
    );
    let Command::List { mode, .. } = key(&mut manager, KeyCode::Char('r')).unwrap() else {
        panic!();
    };
    assert_eq!(mode, ListMode::Refresh);
}

#[test]
fn issue_manual_clear_is_repository_scoped_and_never_refresh_has_no_timer() {
    let mut manager = loaded();
    manager.overview_at = Instant::now() + Duration::from_secs(10000);
    manager.refresh_at = None;
    manager.auto_refresh = false;
    assert!(
        manager
            .poll_refresh(Instant::now() + Duration::from_secs(7200))
            .is_none()
    );
    let Command::List {
        generation,
        mode,
        page,
        ..
    } = manager
        .handle_key(KeyEvent::new(KeyCode::Char('r'), KeyModifiers::CONTROL))
        .unwrap()
    else {
        panic!();
    };
    assert_eq!((mode, page), (ListMode::ClearCache, 1));
    manager.update(Event::Listed {
        generation,
        page,
        result: Err("offline".into()),
    });
    assert!(manager.refresh_at.is_none());
    assert_eq!(manager.board.loaded(), 2);
}

fn work(number: u64, stage: github::IssueStage) -> assignment::View {
    let value = serde_json::json!({
        "id":format!("work-{number}"), "configRevision":1, "batchId":"batch-one",
        "repository":{"host":"github.com","nodeId":"repo","owner":"team","name":"repo"},
        "item":{"id":"item","issues":[{"nodeId":format!("issue-{number}"),"number":number,"title":format!("Work {number}"),"updatedAt":"now","materialDigest":"digest"}],"objective":"Implement issue","acceptanceConditions":["Tests pass"],"scope":{"components":[],"paths":[],"contracts":[],"resources":[]},"dependencies":[],"agent":"coder"},
        "workflow":github::IssueWorkflow::default(),
        "baseCommit":"a".repeat(40),"targetBranch":"main","branch":format!("codex/issue-{number}"),
        "owner":"tester","autoStart":false,"revision":1,"epoch":1,"ownership":"held",
        "threadId":format!("thread-{number}"),"syncState":"synced","attemptedStages":[],"desiredStage":stage,"paused":false,"syncedLabels":{},"detail":"","updatedAt":20
    });
    assignment::View {
        assignment: serde_json::from_value(value).unwrap(),
        stage,
        health: "Working".into(),
        branch_url: None,
        pr_url: None,
    }
}

#[test]
fn raw_and_assigned_issues_merge_by_identity_and_follow_status_without_losing_focus() {
    use github::IssueStage;
    let mut manager = loaded();
    manager.board.views = vec![work(3, IssueStage::Queued), work(18, IssueStage::Review)];
    manager.board.reconcile("", false);
    assert_eq!(manager.board.entries("", false).len(), 3);
    assert_eq!(manager.board.selected, Some(board::Target::Issue(3)));
    manager.selected.insert(3);
    manager.board.views[0].stage = IssueStage::InProgress;
    manager.board.reconcile("", false);
    assert_eq!(
        manager.board.selected_entry("", false).unwrap().group,
        board::Group::InProgress
    );
    assert!(manager.selected.contains(&3));
    key(&mut manager, KeyCode::Enter);
    assert_eq!(manager.detail_issue, Some(3));
    assert!(manager.assignment.is_open());
    key(&mut manager, KeyCode::Esc);
    assert!(manager.is_open());
    assert!(!manager.assignment.is_open());
}

#[test]
fn configured_labels_group_external_work_and_closed_facts_win_over_running_records() {
    let mut manager = loaded();
    let mut labels = github::IssueLabels::default();
    labels.in_progress = "development".into();
    manager.board.labels = Some(labels);
    manager.board.page(
        IssueState::Open,
        1,
        vec![Issue {
            number: 3,
            title: "External work".into(),
            labels: vec!["development".into()],
            assignees: vec!["other".into()],
        }],
        None,
        30,
    );
    assert_eq!(
        manager.board.entries("", false)[0].group,
        board::Group::InProgress
    );
    manager.board.views = vec![work(3, github::IssueStage::InProgress)];
    manager.board.page(
        IssueState::Closed,
        1,
        vec![Issue {
            number: 3,
            title: "Closed manually".into(),
            labels: vec![],
            assignees: vec![],
        }],
        None,
        40,
    );
    assert_eq!(
        manager.board.entries("", false)[0].group,
        board::Group::Closed
    );
    manager.board.page(
        IssueState::Open,
        1,
        vec![Issue {
            number: 3,
            title: "Stale open cache".into(),
            labels: vec![],
            assignees: vec![],
        }],
        None,
        10,
    );
    assert_eq!(
        manager.board.entries("", false)[0].issue.title,
        "Closed manually"
    );
}

#[test]
fn delivered_but_open_issues_and_cancelled_work_do_not_fake_github_closure() {
    use github::IssueStage;
    let mut manager = loaded();
    let mut delivered = work(18, IssueStage::Completed);
    delivered.assignment.workflow.close_on_completion = false;
    manager.board.views = vec![delivered, work(19, IssueStage::Cancelled)];
    let entries = manager.board.entries("", false);
    assert_eq!(
        entries
            .iter()
            .find(|entry| entry.issue.number == 18)
            .unwrap()
            .group,
        board::Group::Delivered
    );
    assert_eq!(
        entries
            .iter()
            .find(|entry| entry.issue.number == 19)
            .unwrap()
            .group,
        board::Group::Blocked
    );
}

#[test]
fn assignment_completion_returns_to_the_same_grouped_page_and_old_metadata_is_ignored() {
    let mut manager = loaded();
    let generation = manager.generation;
    manager.overview_loading = true;
    key(&mut manager, KeyCode::Char('g'));
    assert!(!manager.overview_loading);
    manager.update(Event::Overview {
        generation,
        views: Ok(vec![work(5, github::IssueStage::Blocked)]),
        labels: None,
    });
    assert!(manager.board.views.is_empty());
    manager.update(Event::Assignment {
        generation: manager.generation,
        result: Ok(assignment::Reply::Assignments(vec![work(
            3,
            github::IssueStage::Queued,
        )])),
    });
    assert!(!manager.assignment.is_open());
    assert!(manager.is_open());
    assert_eq!(
        manager
            .board
            .selected_entry("", false)
            .unwrap()
            .issue
            .number,
        3
    );
}

#[test]
fn control_c_closes_the_manager_without_cancelling_the_selected_work() {
    let mut manager = loaded();
    manager.board.views = vec![work(3, github::IssueStage::InProgress)];
    let result = manager.handle_key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL));
    assert!(result.is_none());
    assert!(!manager.is_open());
    assert_eq!(
        manager.board.views[0].assignment.ownership,
        github::IssueOwnership::Held
    );
}

#[test]
fn control_u_only_edits_text_and_never_releases_work() {
    let mut manager = loaded();
    manager.board.views = vec![work(3, github::IssueStage::Queued)];
    assert!(
        manager
            .handle_key(KeyEvent::new(KeyCode::Char('u'), KeyModifiers::CONTROL))
            .is_none()
    );
    assert!(!manager.assignment.is_open());
    key(&mut manager, KeyCode::Char('/'));
    manager.handle_paste("test".into());
    manager.handle_key(KeyEvent::new(KeyCode::Char('u'), KeyModifiers::CONTROL));
    assert!(manager.search.query().is_empty());
}

#[test]
fn committing_a_search_selects_the_result_instead_of_a_previously_focused_group() {
    let mut manager = loaded();
    key(&mut manager, KeyCode::Char('/'));
    manager.handle_paste("#18".into());
    let Some(Command::List { generation, .. }) = key(&mut manager, KeyCode::Enter) else {
        panic!()
    };
    manager.update(Event::Listed {
        generation,
        page: 1,
        result: Ok(Page {
            repository: manager.repository.clone().unwrap(),
            issues: vec![Issue {
                number: 18,
                title: "Found remotely".into(),
                labels: vec![],
                assignees: vec![],
            }],
            next: None,
            refresh_after_seconds: None,
            freshness: "Cached".into(),
            fetched_at: 50,
            notice: String::new(),
        }),
    });
    assert_eq!(manager.board.selected, Some(board::Target::Issue(18)));
    assert!(matches!(
        key(&mut manager, KeyCode::Enter),
        Some(Command::Read { number: 18, .. })
    ));
}

#[test]
fn repeated_action_keys_cannot_reopen_or_modify_an_issue_after_a_transition() {
    let mut manager = loaded();
    manager.board.views = vec![work(3, github::IssueStage::Queued)];
    for code in [
        KeyCode::Enter,
        KeyCode::Char(' '),
        KeyCode::Char('c'),
        KeyCode::Char('s'),
        KeyCode::Char('d'),
    ] {
        assert!(
            manager
                .handle_key(KeyEvent::new_with_kind(
                    code,
                    KeyModifiers::NONE,
                    KeyEventKind::Repeat
                ))
                .is_none()
        );
        assert!(!manager.assignment.is_open());
    }
    assert!(manager.selected.is_empty());
}

#[test]
fn automatic_refresh_does_not_fetch_an_unexpanded_closed_group() {
    let mut manager = loaded();
    key(&mut manager, KeyCode::End);
    let Some(Command::List { state, .. }) = manager.poll_refresh(manager.refresh_at.unwrap())
    else {
        panic!()
    };
    assert_eq!(state, IssueState::Open);
    assert!(!manager.board.closed_loaded);
}

#[test]
fn tab_does_not_repeat_or_leave_issue_detail_and_pending_creation() {
    let mut manager = loaded();
    let selected = manager.board.selected.clone();
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
        assert_eq!(manager.board.selected, selected);
    }
    manager.detail = Some("Issue details".into());
    assert!(key(&mut manager, KeyCode::Tab).is_none());
    assert_eq!(manager.detail.as_deref(), Some("Issue details"));
    key(&mut manager, KeyCode::Esc);
    key(&mut manager, KeyCode::Char(' '));
    let Some(Command::Start { generation, .. }) =
        manager.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::CONTROL))
    else {
        panic!("start selected issue")
    };
    assert!(key(&mut manager, KeyCode::Tab).is_none());
    assert_eq!(manager.generation, generation);
    assert_eq!(manager.board.selected, selected);
}
