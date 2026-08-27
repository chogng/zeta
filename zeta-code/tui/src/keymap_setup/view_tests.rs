use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyModifiers;

use super::KeymapCaptureOutcome;
use super::capture_view;
use crate::keymap::AppKeymap;
use crate::keymap_setup::KeymapCaptureMode;
use crate::keymap_setup::KeymapEditIntent;
use crate::keymap_setup::KeymapEditKind;

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
        KeymapEditIntent::AddAlternate,
        KeymapCaptureMode::Chord,
    );

    assert!(matches!(
        capture.handle_key(KeyEvent::new(KeyCode::Char('k'), KeyModifiers::CONTROL,)),
        KeymapCaptureOutcome::Pending(_)
    ));
    let KeymapCaptureOutcome::Edit(edit) =
        capture.handle_key(KeyEvent::new(KeyCode::Char('y'), KeyModifiers::CONTROL))
    else {
        panic!("expected completed chord edit");
    };

    assert_eq!(edit.expected_revision, 4);
    assert_eq!(edit.command_id, "zetaCode.action.copyLastResponse");
    assert_eq!(
        edit.kind,
        KeymapEditKind::Set {
            key: "ctrl+k ctrl+y".into(),
            intent: KeymapEditIntent::AddAlternate,
        }
    );
}

#[test]
fn escape_cancels_capture_without_emitting_an_edit() {
    let (_, mut capture) = capture_view(
        copy_action(),
        1,
        KeymapEditIntent::ReplaceCustom,
        KeymapCaptureMode::SingleKey,
    );

    assert!(matches!(
        capture.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)),
        KeymapCaptureOutcome::Cancelled
    ));
}
