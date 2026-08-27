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
        ShortcutEditIntent::ReplaceCustom,
        ShortcutCaptureMode::SingleKey,
    );

    assert!(matches!(
        capture.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)),
        ShortcutCaptureOutcome::Cancelled
    ));
}

#[test]
fn shortcut_view_collects_configurable_and_fixed_controls() {
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
            "Cycle approval mode",
            "Open rewind checkpoints",
            "Attach clipboard image",
            "Interrupt or quit",
            "Copy last response",
            "Suspend Zeta",
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
