use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyModifiers;

use super::ShortcutCaptureOutcome;
use super::capture_view;
use crate::components::selection::SelectionViewState;
use crate::features::shortcuts::ShortcutCaptureMode;
use crate::features::shortcuts::ShortcutEditIntent;
use crate::features::shortcuts::ShortcutEditKind;
use crate::keymap::AppKeymap;
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use zeta_keybinding::HostPlatform;

fn copy_action() -> crate::keymap::KeymapActionSnapshot {
    AppKeymap::default()
        .setup_actions()
        .into_iter()
        .find(|action| action.command_id == "zetaCode.action.copyLastResponse")
        .unwrap()
}

#[test]
fn chord_capture_waits_for_two_strokes_and_emits_canonical_edit() {
    let (_, mut capture) = capture_view(
        copy_action(),
        4,
        ShortcutEditIntent::AddAlternate,
        ShortcutCaptureMode::Chord,
    );

    assert!(matches!(
        capture.handle_key(KeyEvent::new(KeyCode::Char('k'), KeyModifiers::CONTROL,)),
        ShortcutCaptureOutcome::Pending(_)
    ));
    let ShortcutCaptureOutcome::Edit(edit) =
        capture.handle_key(KeyEvent::new(KeyCode::Char('y'), KeyModifiers::CONTROL))
    else {
        panic!("expected completed chord edit");
    };

    assert_eq!(edit.expected_revision, 4);
    assert_eq!(edit.command_id, "zetaCode.action.copyLastResponse");
    assert_eq!(
        edit.kind,
        ShortcutEditKind::Set {
            key: "ctrl+k ctrl+y".into(),
            intent: ShortcutEditIntent::AddAlternate,
        }
    );
}

#[test]
fn escape_cancels_capture_without_emitting_an_edit() {
    let (_, mut capture) = capture_view(
        copy_action(),
        1,
        ShortcutEditIntent::ReplaceUser,
        ShortcutCaptureMode::SingleKey,
    );

    assert!(matches!(
        capture.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)),
        ShortcutCaptureOutcome::Cancelled
    ));
}

#[test]
fn shortcut_view_lists_keys_before_responsibilities() {
    let view = super::shortcut_view(
        AppKeymap::default().setup_actions(),
        std::path::Path::new("/profile/zeta-code/keybindings.json"),
        &[],
        1,
    );
    let state = SelectionViewState::new(view.model.into_body());
    let labels = state
        .visible_items()
        .into_iter()
        .map(|item| item.label())
        .collect::<Vec<_>>();

    assert_eq!(
        labels,
        vec![
            "shift+tab",
            "unbound",
            "ctrl+v",
            "ctrl+c",
            "ctrl+d",
            "ctrl+o",
            "ctrl+z",
            "Enter",
            "Shift-Enter / Alt-Enter / Ctrl-J",
            "Esc Esc",
            "Tab",
            "Esc",
            "↑ / ↓",
            "← / →",
            "Home / End",
            "PageUp / PageDown",
            "Ctrl-Home / Ctrl-End",
        ]
    );
}

#[test]
fn shortcut_rows_align_responsibility_and_source_columns_without_command_ids() {
    let rules = crate::keymap::compile_app_user_bindings(
        br#"[{"key":"ctrl+y","command":"zetaCode.action.copyLastResponse"}]"#,
        HostPlatform::Linux,
    )
    .unwrap();
    let mut keymap = AppKeymap::default();
    keymap.replace_user_bindings(rules).unwrap();
    let view = super::shortcut_view(
        keymap.setup_actions(),
        std::path::Path::new("/profile/zeta-code/keybindings.json"),
        &[],
        1,
    );
    let state = SelectionViewState::new(view.model.into_body());
    let backend = TestBackend::new(100, 20);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|frame| crate::components::selection::draw(frame, frame.area(), &state))
        .unwrap();

    let buffer = terminal.backend().buffer();
    let rows = (0..20)
        .map(|row| {
            (0..100)
                .map(|column| buffer[(column, row)].symbol())
                .collect::<String>()
        })
        .collect::<Vec<_>>();
    let default_row = rows.iter().find(|row| row.contains("ctrl+o")).unwrap();
    let user_row = rows.iter().find(|row| row.contains("ctrl+y")).unwrap();
    let fixed_row = rows.iter().find(|row| row.contains("Enter")).unwrap();

    let responsibility_column = default_row.find("Copy last response").unwrap();
    assert_eq!(
        user_row.find("Copy last response"),
        Some(responsibility_column)
    );
    assert_eq!(
        fixed_row.find("send the current prompt"),
        Some(responsibility_column)
    );
    assert_eq!(default_row.find("default"), fixed_row.find("default"));
    assert_eq!(user_row.find("user"), default_row.find("default"));
    assert!(!rows.join("\n").contains("zetaCode."));
    assert!(!rows.join("\n").contains("Built in"));
}
