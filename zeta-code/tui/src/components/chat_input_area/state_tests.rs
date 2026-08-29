use super::ChatInputArea;
use super::ChatInputAreaOutcome;
use crate::components::chat_input::ChatInputItem;
use crate::components::chat_input_area::ChatInputAreaHeightEntryKind;
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
use zeta_protocol::PlanStep;
use zeta_protocol::PlanStepStatus;
use zeta_protocol::PlanUpdate;

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
    area.replace_turn_status(Some(active_plan()), Vec::new());
    area.replace_turn_status(Some(active_plan()), vec!["follow up".into()]);

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

    area.replace_turn_status(Some(active_plan()), Vec::new());
    assert_eq!(
        area.height_entries()[0].kind(),
        ChatInputAreaHeightEntryKind::PlanProgress
    );

    area.replace_turn_status(None, Vec::new());
    assert!(area.height_entries().is_empty());
}

#[test]
fn completed_plan_leaves_the_height_stack() {
    let mut area = ChatInputArea::new();
    let mut plan = active_plan();
    plan.steps[0].status = PlanStepStatus::Completed;

    area.replace_turn_status(Some(plan), Vec::new());

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
