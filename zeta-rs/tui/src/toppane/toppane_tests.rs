use super::ComposerInput;
use super::InteractionPane;
use super::InteractionPaneOutcome;
use super::SelectionItem;
use super::SelectionTab;
use super::SelectionViewModel;
use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyModifiers;

#[test]
fn pane_routes_submission_from_the_composer() {
    let mut pane = InteractionPane::new();
    pane.insert_text("hello");

    let outcome = pane.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

    let InteractionPaneOutcome::Submit(submission) = outcome else {
        panic!("expected submission");
    };
    assert_eq!(submission.display_text, "hello");
    assert_eq!(submission.input, vec![ComposerInput::Text("hello".into())]);
    assert_eq!(pane.text(), "");
}

#[test]
fn selection_view_preserves_composer_draft_and_owns_input_until_dismissed() {
    let mut pane = InteractionPane::new();
    pane.insert_text("draft");
    pane.show_selection_view(SelectionViewModel::new(
        "Help",
        vec![SelectionTab::new(
            "Commands",
            vec![SelectionItem::new("/status")],
        )],
    ));

    assert_eq!(
        pane.handle_key(KeyEvent::new(KeyCode::Char('s'), KeyModifiers::NONE)),
        InteractionPaneOutcome::Consumed
    );
    assert_eq!(pane.selection_view().unwrap().query(), "s");
    assert_eq!(pane.text(), "draft");

    pane.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));

    assert!(pane.selection_view().is_none());
    assert_eq!(pane.text(), "draft");
}

#[test]
fn escape_pops_one_interaction_view_at_a_time() {
    let mut pane = InteractionPane::new();
    pane.show_selection_view(selection_view("Parent"));
    pane.show_selection_view(selection_view("Child"));

    assert_eq!(pane.selection_view().unwrap().title(), "Child");
    pane.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
    assert_eq!(pane.selection_view().unwrap().title(), "Parent");
    pane.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
    assert!(pane.selection_view().is_none());
}

fn selection_view(title: &str) -> SelectionViewModel {
    SelectionViewModel::new(
        title,
        vec![SelectionTab::new("Items", vec![SelectionItem::new("Item")])],
    )
}
