use super::schedule_action;
use super::select_hovered_popup_item;
use crate::app::App;
use crate::app::AppCommand;
use ratatui::layout::Rect;
use std::collections::VecDeque;
use std::path::PathBuf;

#[test]
fn request_actions_wait_for_the_active_request_without_losing_order() {
    let mut queued = VecDeque::new();
    let first = AppCommand::OpenWorkspaceDirectory {
        path: PathBuf::from("src"),
    };
    let second = AppCommand::PreviewWorkspaceFile {
        path: PathBuf::from("src/lib.rs"),
    };

    assert!(schedule_action(Some(first), true, &mut queued).is_none());
    assert!(schedule_action(Some(second), true, &mut queued).is_none());
    assert_eq!(queued.len(), 2);
    assert!(matches!(
        schedule_action(None, false, &mut queued),
        Some(AppCommand::OpenWorkspaceDirectory { .. })
    ));
    assert!(matches!(
        schedule_action(None, false, &mut queued),
        Some(AppCommand::PreviewWorkspaceFile { .. })
    ));
}

#[test]
fn quit_bypasses_a_pending_request() {
    let mut queued = VecDeque::new();

    assert!(matches!(
        schedule_action(Some(AppCommand::Quit), true, &mut queued),
        Some(AppCommand::Quit)
    ));
    assert!(queued.is_empty());
}

#[test]
fn pointer_move_selects_the_hovered_popup_row_and_preserves_it_outside() {
    let mut app = App::new();
    app.insert_text("/");
    let area = Rect::new(0, 0, 80, 20);

    select_hovered_popup_item(&mut app, area, 2, 11);
    assert_eq!(app.slash_popup().unwrap().selected, 2);

    select_hovered_popup_item(&mut app, area, 1, 11);
    assert_eq!(app.slash_popup().unwrap().selected, 2);
}
