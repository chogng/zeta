use super::ChatComposer;
use super::ChatComposerOutcome;
use crate::components::chat_input::ChatInput;
use crate::components::chat_input::ChatInputItem;
use crate::components::list_selection::ListSelectionGroup;
use crate::components::list_selection::ListSelectionItem;
use crate::components::list_selection::ListSelectionModel;
use crate::components::pane::PaneSpec;
use crate::components::search_box::SearchBoxModel;
use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyModifiers;

#[test]
fn composer_routes_submission_from_thread_owned_input() {
    let mut composer = ChatComposer::new();
    let mut input = ChatInput::new();
    composer.insert_text(&mut input, "hello");

    let outcome = composer.handle_key(
        &mut input,
        KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE),
    );

    let ChatComposerOutcome::Submit(submission) = outcome else {
        panic!("expected submission");
    };
    assert_eq!(submission.display_text, "hello");
    assert_eq!(submission.input, vec![ChatInputItem::Text("hello".into())]);
    assert_eq!(input.text(), "");
}

#[test]
fn pane_preserves_thread_owned_draft_until_dismissed() {
    let mut composer = ChatComposer::new();
    let mut input = ChatInput::new();
    composer.insert_text(&mut input, "draft");
    let pane_id = composer.push_list_selection(PaneSpec::new(
        ListSelectionModel::new(
            "Help",
            vec![ListSelectionGroup::new(
                "Commands",
                vec![ListSelectionItem::new("/status")],
            )],
        )
        .with_search(SearchBoxModel::new("Search commands")),
        "↑ search · Esc back",
    ));

    composer.handle_key(&mut input, KeyEvent::new(KeyCode::Up, KeyModifiers::NONE));
    composer.handle_key(
        &mut input,
        KeyEvent::new(KeyCode::Char('s'), KeyModifiers::NONE),
    );
    assert_eq!(composer.list_selection().unwrap().query(), "s");
    assert_eq!(input.text(), "draft");
    assert_eq!(
        composer.handle_key(&mut input, KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE),),
        ChatComposerOutcome::PaneDismissed(pane_id)
    );
    assert_eq!(input.text(), "draft");
}

#[test]
fn queue_target_returns_content_to_the_feature_owner() {
    let mut composer = ChatComposer::new();
    let mut input = ChatInput::new();
    composer.insert_text(&mut input, "follow up");

    let outcome = composer.handle_queued_turn_key(
        &mut input,
        KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE),
    );

    let ChatComposerOutcome::Queued(queued) = outcome else {
        panic!("expected Queue content");
    };
    assert_eq!(queued.display_text(), "follow up");
    assert!(composer.pane_views().is_empty());
}
