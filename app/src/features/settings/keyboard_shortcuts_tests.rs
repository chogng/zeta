use super::{KeyboardShortcutsState, keyboard_shortcut_rows, keyboard_shortcuts_ids, row_element};
use crate::keybindings::NativeKeybindings;
use app_keybinding_ui::KeyboardShortcuts;
use app_keybinding_ui::paint_chord_hint;
use std::time::{Duration, Instant};
use zeta_commands::AppCommandId;
use zeta_keybinding::Chord;
use zeta_keybinding::HostPlatform;
use zeta_keybinding::ShortcutModifiers;
use zeta_keybinding::serialize_key_sequence;
use zeta_ui::{Color, Point, Rect, UiScene};
use zui::ui::{InteractionFrame, UiDispatch, UiFrame};

#[test]
fn recording_collects_chords_and_commits_after_the_quiet_period() {
    let now = Instant::now();
    let mut state = KeyboardShortcutsState::default();
    state.toggle();
    state.start_recording(AppCommandId::ToggleTabContainer);
    state.record(
        Chord::logical("k", ShortcutModifiers::primary()).expect("first chord"),
        now,
    );
    state.record(
        Chord::logical("b", ShortcutModifiers::primary()).expect("second chord"),
        now + Duration::from_millis(100),
    );

    assert!(state.advance(now + Duration::from_millis(500)).is_none());
    let commit = state
        .advance(now + Duration::from_millis(1_200))
        .expect("completed recording");
    assert_eq!(commit.command, AppCommandId::ToggleTabContainer);
    assert_eq!(
        serialize_key_sequence(&commit.keybinding),
        "primary+k primary+b"
    );
}

#[test]
fn every_bindable_command_has_a_distinct_stable_row() {
    let mut ids = AppCommandId::BINDABLE
        .into_iter()
        .map(row_element)
        .collect::<Vec<_>>();
    ids.sort_unstable();
    ids.dedup();
    assert_eq!(ids.len(), AppCommandId::BINDABLE.len());
}

#[test]
fn visible_settings_are_modal_and_paint_keycaps_for_defaults() {
    let mut state = KeyboardShortcutsState::default();
    state.toggle();
    let keybindings = NativeKeybindings::default();
    let rows = keyboard_shortcut_rows(&keybindings);
    let dispatch = UiDispatch::default();
    let shortcuts = KeyboardShortcuts::new(
        Rect::from_xywh(0.0, 0.0, 1_000.0, 700.0),
        &state,
        &rows,
        &[],
        keyboard_shortcuts_ids(),
        keybindings.platform(),
        &dispatch,
    )
    .expect("visible shortcuts");
    let mut frame = UiFrame::<InteractionFrame>::new(Color::WHITE);
    frame.draw_component(&shortcuts);
    let scene = frame.scene();

    assert!(
        frame
            .interaction()
            .target_at(Point::new(0.0, 0.0))
            .is_none()
    );
    assert!(
        frame
            .interaction()
            .node(row_element(AppCommandId::Copy))
            .is_some()
    );
    let primary = if HostPlatform::current() == HostPlatform::MacOs {
        "⌘"
    } else {
        "Ctrl"
    };
    assert!(
        scene
            .text_blocks()
            .iter()
            .any(|text| text.text() == primary)
    );
    assert!(scene.text_blocks().iter().any(|text| text.text() == "C"));
}

#[test]
fn chord_hint_reuses_keycaps_and_explains_the_pending_state() {
    let sequence =
        zeta_keybinding::parse_key_sequence("primary+k primary+c").expect("chord sequence");
    let mut scene = UiScene::new(Color::WHITE);

    paint_chord_hint(
        &mut scene,
        Rect::from_xywh(0.0, 0.0, 1_000.0, 700.0),
        &sequence,
        1,
        HostPlatform::MacOs,
    );

    assert!(scene.text_blocks().iter().any(|text| text.text() == "⌘"));
    assert!(scene.text_blocks().iter().any(|text| text.text() == "K"));
    assert!(
        scene
            .text_blocks()
            .iter()
            .any(|text| text.text() == "waiting for next key…")
    );
}
