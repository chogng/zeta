use super::ChatInputArea;
use super::ChatInputAreaOutcome;
use crate::components::chat_input::ChatInputItem;
use crate::components::chat_input::SkillSelectorItem;
use crate::components::chat_input::default_slash_command_catalog;
use crate::components::chat_input_area::ChatInputAreaHeightEntryKind;
use crate::components::chat_input_area::ChatInputAreaHeightEntryView;
use crate::components::list_selection::ListSelectionGroup;
use crate::components::list_selection::ListSelectionItem;
use crate::components::list_selection::ListSelectionModel;
use crate::components::pane::PaneSpec;
use crate::components::query::QueryChoice;
use crate::components::query::QueryCustomAnswer;
use crate::components::query::QueryQuestion;
use crate::components::search_box::SearchBoxModel;
use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyEventKind;
use crossterm::event::KeyModifiers;
use zeta_protocol::ContentDigest;
use zeta_protocol::PlanStep;
use zeta_protocol::PlanStepStatus;
use zeta_protocol::PlanUpdate;
use zeta_protocol::SkillId;
use zeta_protocol::SkillName;
use zeta_protocol::SkillRef;
use zeta_protocol::SkillSourceId;

#[test]
fn pane_routes_submission_from_its_chat_input() {
    let mut pane = ChatInputArea::new();
    pane.insert_text("hello");

    let outcome = pane.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

    let ChatInputAreaOutcome::Submit(submission) = outcome else {
        panic!("expected submission");
    };
    assert_eq!(submission.display_text, "hello");
    assert_eq!(submission.input, vec![ChatInputItem::Text("hello".into())]);
    assert_eq!(pane.text(), "");
}

#[test]
fn list_selection_pane_preserves_chat_input_draft_and_owns_input_until_dismissed() {
    let mut pane = ChatInputArea::new();
    pane.insert_text("draft");
    let pane_id = pane.push_list_selection(PaneSpec::new(
        ListSelectionModel::new(
            "Help",
            vec![ListSelectionGroup::new(
                "Commands",
                vec![ListSelectionItem::new("/status")],
            )],
        )
        .with_search(SearchBoxModel::new("Search commands")),
        "Space search  ·  Esc back",
    ));

    assert_eq!(
        pane.handle_key(KeyEvent::new(KeyCode::Char(' '), KeyModifiers::NONE)),
        ChatInputAreaOutcome::Consumed
    );
    assert_eq!(
        pane.handle_key(KeyEvent::new(KeyCode::Char('s'), KeyModifiers::NONE)),
        ChatInputAreaOutcome::Consumed
    );
    assert_eq!(pane.list_selection().unwrap().query(), "s");
    assert_eq!(pane.text(), "draft");

    assert_eq!(
        pane.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)),
        ChatInputAreaOutcome::PaneDismissed(pane_id)
    );

    assert!(pane.list_selection().is_none());
    assert_eq!(pane.text(), "draft");
}

#[test]
fn repeated_escape_does_not_close_the_parent_after_dismissing_a_search_view() {
    let mut pane = ChatInputArea::new();
    pane.push_list_selection(list_selection("Parent"));
    let child_id = pane.push_list_selection(PaneSpec::new(
        ListSelectionModel::new(
            "Help",
            vec![ListSelectionGroup::new(
                "Commands",
                vec![ListSelectionItem::new("/status")],
            )],
        )
        .with_search(SearchBoxModel::new("Search commands")),
        "Space search  ·  Esc back",
    ));
    pane.handle_key(KeyEvent::new(KeyCode::Char(' '), KeyModifiers::NONE));

    assert_eq!(
        pane.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)),
        ChatInputAreaOutcome::PaneDismissed(child_id)
    );
    assert_eq!(
        pane.handle_key(KeyEvent::new_with_kind(
            KeyCode::Esc,
            KeyModifiers::NONE,
            KeyEventKind::Repeat,
        )),
        ChatInputAreaOutcome::Consumed
    );
    assert_eq!(pane.list_selection().unwrap().title(), "Parent");
}

#[test]
fn escape_pops_one_interaction_view_at_a_time() {
    let mut pane = ChatInputArea::new();
    pane.push_list_selection(list_selection("Parent"));
    pane.push_list_selection(list_selection("Child"));

    assert_eq!(pane.list_selection().unwrap().title(), "Child");
    pane.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
    assert_eq!(pane.list_selection().unwrap().title(), "Parent");
    pane.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
    assert!(pane.list_selection().is_none());
}

#[test]
fn queue_stacks_above_an_existing_plan_and_each_can_be_removed_independently() {
    let mut area = ChatInputArea::new();
    area.replace_plan_progress(Some(active_plan()));
    area.insert_text("follow up");
    assert_eq!(
        area.handle_queued_turn_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)),
        ChatInputAreaOutcome::Consumed
    );

    assert_eq!(
        area.height_entries()
            .iter()
            .map(|entry| entry.kind())
            .collect::<Vec<_>>(),
        [
            ChatInputAreaHeightEntryKind::PlanProgress,
            ChatInputAreaHeightEntryKind::Queue,
        ]
    );

    assert_eq!(
        area.handle_active_turn_key(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE)),
        ChatInputAreaOutcome::Consumed
    );
    assert_eq!(area.text(), "follow up");
    assert_eq!(
        area.height_entries()[0].kind(),
        ChatInputAreaHeightEntryKind::PlanProgress
    );

    area.replace_plan_progress(None);
    assert!(area.height_entries().is_empty());
}

#[test]
fn pending_steer_stacks_above_plan_and_queue_until_its_request_finishes() {
    let mut area = ChatInputArea::new();
    area.replace_plan_progress(Some(active_plan()));
    area.insert_text("follow up");
    area.handle_queued_turn_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

    let steer_id = area.begin_steer("change direction".into());

    assert_eq!(
        area.height_entries()
            .iter()
            .map(|entry| entry.kind())
            .collect::<Vec<_>>(),
        [
            ChatInputAreaHeightEntryKind::PlanProgress,
            ChatInputAreaHeightEntryKind::Queue,
            ChatInputAreaHeightEntryKind::Steer,
        ]
    );

    assert!(area.finish_steer(steer_id));
    assert_eq!(
        area.height_entries()
            .iter()
            .map(|entry| entry.kind())
            .collect::<Vec<_>>(),
        [
            ChatInputAreaHeightEntryKind::PlanProgress,
            ChatInputAreaHeightEntryKind::Queue,
        ]
    );
}

#[test]
fn follow_up_handlers_keep_steer_queue_and_suggest_keys_distinct() {
    let mut area = ChatInputArea::new();
    area.insert_text("steer me");

    let ChatInputAreaOutcome::Submit(steer) =
        area.handle_active_turn_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE))
    else {
        panic!("expected steer submission");
    };
    assert_eq!(steer.display_text, "steer me");

    area.insert_text("queue me");
    assert_eq!(
        area.handle_active_turn_key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE)),
        ChatInputAreaOutcome::Unhandled
    );
    assert_eq!(area.text(), "queue me");
    assert_eq!(
        area.handle_queued_turn_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)),
        ChatInputAreaOutcome::Consumed
    );
    assert!(matches!(
        area.height_entries().as_slice(),
        [ChatInputAreaHeightEntryView::Queue(view)] if view.items[0].text == "queue me"
    ));

    assert_eq!(
        area.handle_active_turn_key(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE)),
        ChatInputAreaOutcome::Consumed
    );
    assert_eq!(area.text(), "queue me");
    let ChatInputAreaOutcome::Submit(send_now) =
        area.handle_active_turn_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE))
    else {
        panic!("retrieved Queue input should send as a steer");
    };
    assert_eq!(send_now.display_text, "queue me");

    area.insert_text("/");
    assert_eq!(
        area.handle_active_turn_key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE)),
        ChatInputAreaOutcome::Consumed
    );
    assert!(!area.text().is_empty());
}

#[test]
fn active_turn_keeps_a_skill_draft_until_it_is_queued() {
    let mut area = ChatInputArea::new();
    area.replace_chat_input_catalog(
        default_slash_command_catalog(),
        vec![SkillSelectorItem::new(
            "commit".into(),
            "draft a commit message".into(),
            SkillRef::pinned(
                SkillId::new(
                    SkillSourceId::new("user:skill-source:test").unwrap(),
                    SkillName::new("commit").unwrap(),
                ),
                ContentDigest::sha256(b"commit skill"),
            ),
        )],
        Vec::new(),
    );
    area.insert_text("$com");
    assert_eq!(
        area.handle_active_turn_key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE)),
        ChatInputAreaOutcome::Consumed
    );
    area.insert_text("staged changes");

    let outcome = area.handle_active_turn_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

    assert!(matches!(
        outcome,
        ChatInputAreaOutcome::SubmissionRejected(message)
            if message.contains("switch follow-up messages to Queue")
    ));
    assert_eq!(area.text(), "$commit staged changes");
    assert_eq!(
        area.handle_queued_turn_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)),
        ChatInputAreaOutcome::Consumed
    );
    assert_eq!(area.text(), "");
    assert!(matches!(
        area.handle_active_turn_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)),
        ChatInputAreaOutcome::SubmissionRejected(message)
            if message.contains("queued message with a Skill")
    ));
    assert!(matches!(
        area.height_entries().as_slice(),
        [ChatInputAreaHeightEntryView::Queue(view)] if view.items[0].text == "$commit staged changes"
    ));
    area.handle_active_turn_key(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE));
    assert_eq!(area.text(), "$commit staged changes");
    assert!(matches!(
        area.handle_active_turn_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)),
        ChatInputAreaOutcome::SubmissionRejected(_)
    ));
}

#[test]
fn completed_plan_leaves_the_height_stack() {
    let mut area = ChatInputArea::new();
    let mut plan = active_plan();
    plan.steps[0].status = PlanStepStatus::Completed;

    area.replace_plan_progress(Some(plan));

    assert!(area.height_entries().is_empty());
}

#[test]
fn query_custom_answer_borrows_the_input_without_replacing_the_chat_draft() {
    let mut area = ChatInputArea::new();
    area.insert_text("chat draft");
    area.show_query(vec![QueryQuestion {
        id: "detail".into(),
        header: "Detail".into(),
        prompt: "What value?".into(),
        choices: vec![QueryChoice {
            label: "Default".into(),
            description: "Use default".into(),
        }],
        custom_answer: QueryCustomAnswer::Allowed,
    }])
    .unwrap();

    assert_eq!(
        area.activate_overlay_choice(1),
        Some(ChatInputAreaOutcome::Consumed)
    );
    area.handle_paste("custom value".into()).unwrap();
    let outcome = area.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

    let ChatInputAreaOutcome::QueryResponse { answers, .. } = outcome else {
        panic!("expected query response");
    };
    assert_eq!(answers[0].value, "custom value");
    assert_eq!(area.text(), "chat draft");
}

fn list_selection(title: &str) -> PaneSpec<ListSelectionModel> {
    PaneSpec::new(
        ListSelectionModel::new(
            title,
            vec![ListSelectionGroup::new(
                "Items",
                vec![ListSelectionItem::new("Item")],
            )],
        ),
        "Esc back",
    )
}

fn active_plan() -> PlanUpdate {
    PlanUpdate {
        explanation: None,
        steps: vec![PlanStep {
            step: "Implement".into(),
            status: PlanStepStatus::InProgress,
        }],
    }
}
