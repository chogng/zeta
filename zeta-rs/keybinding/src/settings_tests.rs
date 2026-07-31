use zeta_ui::{Color, Point, Rect, UiScene};
use zeta_ui_dispatch::{ElementId, InteractionFrame, UiDispatch};

use super::{KeyboardShortcutRow, KeyboardShortcuts, KeyboardShortcutsIds, paint_chord_hint};
use crate::{HostPlatform, KeyboardShortcutsState, parse_key_sequence};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Command {
    Copy,
}

const PARENT: ElementId = ElementId::scoped(90, 1);
const ROOT: ElementId = ElementId::scoped(90, 2);
const CLOSE: ElementId = ElementId::scoped(90, 3);
const COPY: ElementId = ElementId::scoped(90, 4);

#[test]
fn visible_settings_are_modal_and_paint_keycaps() {
    let mut state = KeyboardShortcutsState::default();
    state.toggle();
    let copy = parse_key_sequence("primary+c").expect("copy shortcut");
    let rows = [KeyboardShortcutRow::new(
        Command::Copy,
        COPY,
        "Copy",
        Some(&copy),
    )];
    let dispatch = UiDispatch::default();
    let shortcuts = KeyboardShortcuts::new(
        Rect::from_xywh(0.0, 0.0, 1_000.0, 700.0),
        &state,
        &rows,
        &[],
        KeyboardShortcutsIds::new(PARENT, ROOT, CLOSE),
        HostPlatform::MacOs,
        &dispatch,
    )
    .expect("visible shortcuts");
    let mut frame = InteractionFrame::default();
    let mut scene = UiScene::new(Color::WHITE);

    shortcuts.register_interactions(&mut frame);
    scene.draw_component(&shortcuts);

    assert!(frame.target_at(Point::new(0.0, 0.0)).is_none());
    assert!(frame.node(COPY).is_some());
    assert!(scene.text_blocks().iter().any(|text| text.text() == "⌘"));
    assert!(scene.text_blocks().iter().any(|text| text.text() == "C"));
}

#[test]
fn chord_hint_reuses_keycaps_and_explains_the_pending_state() {
    let sequence = parse_key_sequence("primary+k primary+c").expect("chord sequence");
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
