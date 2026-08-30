use super::PaneBodyView;
use super::PaneOutcome;
use super::PaneSpec;
use super::PaneStack;
use crate::components::key_capture::KeyCapture;
use crate::components::list_selection::ListSelectionGroup;
use crate::components::list_selection::ListSelectionItem;
use crate::components::list_selection::ListSelectionItemId;
use crate::components::list_selection::ListSelectionModel;
use crate::components::text_prompt::TextPromptSpec;
use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyModifiers;

fn selection_spec(id: ListSelectionItemId) -> PaneSpec<ListSelectionModel> {
    PaneSpec::new(
        ListSelectionModel::new(
            "Choose",
            vec![ListSelectionGroup::new(
                "All",
                vec![ListSelectionItem::new("First").with_id(id)],
            )],
        )
        .with_activation_label("choose")
        .without_tab_bar(),
    )
}

#[test]
fn stack_owns_stable_ids_and_only_exposes_the_top_pane() {
    let mut stack = PaneStack::default();
    let first = stack.push_list_selection(selection_spec(ListSelectionItemId::new("first")));
    let second = stack.push_list_selection(selection_spec(ListSelectionItemId::new("first")));

    assert_ne!(first, second);
    assert_eq!(stack.top_id(), Some(second));
    assert!(matches!(
        stack.top_view().unwrap().body(),
        PaneBodyView::ListSelection(body) if body.title() == "Choose"
    ));
    assert_eq!(stack.pop(), Some(second));
    assert!(matches!(
        stack.top_view().unwrap().body(),
        PaneBodyView::ListSelection(body) if body.title() == "Choose"
    ));
}

#[test]
fn list_selection_input_is_normalized_into_pane_outcomes() {
    let item_id = ListSelectionItemId::new("first");
    let mut stack = PaneStack::default();
    let pane_id = stack.push_list_selection(selection_spec(item_id.clone()));

    assert_eq!(
        stack.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)),
        Some((pane_id, PaneOutcome::ActivateSelection(item_id)))
    );
    assert_eq!(
        stack.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)),
        Some((pane_id, PaneOutcome::Dismiss))
    );
}

#[test]
fn text_prompt_submission_uses_the_same_pane_outcome_boundary() {
    let mut stack = PaneStack::default();
    let pane_id = stack.push_text_prompt(
        PaneSpec::new(TextPromptSpec {
            title: "API key".into(),
            explanation: "Enter a value".into(),
            placeholder: "value".into(),
            masked: false,
        })
        .with_key_hint("Enter", "save")
        .with_key_hint("Esc", "to close"),
    );
    stack.handle_key(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE));

    assert_eq!(
        stack.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)),
        Some((pane_id, PaneOutcome::SubmitText("x".into())))
    );
}

#[test]
fn key_capture_emits_a_pane_outcome_before_the_feature_interprets_it() {
    let mut stack = PaneStack::default();
    let pane_id = stack.push_key_capture(
        PaneSpec::new(KeyCapture::new(
            "Record shortcut",
            vec!["Press a key".into()],
        ))
        .with_key_hint("Esc", "cancel"),
    );
    let key = KeyEvent::new(KeyCode::Char('y'), KeyModifiers::CONTROL);

    assert_eq!(
        stack.handle_key(key),
        Some((pane_id, PaneOutcome::KeyCaptured(key)))
    );
}
