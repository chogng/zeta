use super::{
    InputMethodContext, InputMethodTarget, composer_composition_event, encode_terminal_ime_event,
};
use zeta_terminal::{GridSize, ScreenBuffer, TerminalCore};
use zeta_ui::{TextInputCompositionCursor, TextInputCompositionEvent};
use zeta_winit::Ime;

#[test]
fn target_requires_an_active_window_and_the_appropriate_editable_surface() {
    let composer = InputMethodContext {
        window_active: true,
        screen: ScreenBuffer::Primary,
        composer_focused: true,
    };
    let toolbar = InputMethodContext {
        composer_focused: false,
        ..composer
    };
    let terminal_grid = InputMethodContext {
        screen: ScreenBuffer::Alternate,
        composer_focused: false,
        ..composer
    };
    let inactive_window = InputMethodContext {
        window_active: false,
        ..composer
    };

    assert_eq!(
        InputMethodTarget::for_context(composer),
        InputMethodTarget::Composer
    );
    assert_eq!(
        InputMethodTarget::for_context(toolbar),
        InputMethodTarget::Disabled
    );
    assert_eq!(
        InputMethodTarget::for_context(terminal_grid),
        InputMethodTarget::TerminalGrid
    );
    assert_eq!(
        InputMethodTarget::for_context(inactive_window),
        InputMethodTarget::Disabled
    );
}

#[test]
fn composer_conversion_preserves_preedit_cursor_and_commit_boundaries() {
    assert_eq!(
        composer_composition_event(Ime::Preedit("你好".to_owned(), Some((0, 3)))),
        Some(TextInputCompositionEvent::Preedit {
            text: "你好".to_owned(),
            cursor: TextInputCompositionCursor::Visible(0..3),
        })
    );
    assert_eq!(
        composer_composition_event(Ime::Preedit("世界".to_owned(), None)),
        Some(TextInputCompositionEvent::Preedit {
            text: "世界".to_owned(),
            cursor: TextInputCompositionCursor::Hidden,
        })
    );
    assert_eq!(
        composer_composition_event(Ime::Commit("完成".to_owned())),
        Some(TextInputCompositionEvent::Commit("完成".to_owned()))
    );
    assert_eq!(
        composer_composition_event(Ime::Disabled),
        Some(TextInputCompositionEvent::Cancel)
    );
    assert_eq!(composer_composition_event(Ime::Enabled), None);
}

#[test]
fn terminal_grid_forwards_only_committed_ime_text() {
    let terminal = TerminalCore::new(GridSize::new(24, 80));

    assert_eq!(
        encode_terminal_ime_event(&terminal, &Ime::Commit("终端".to_owned())),
        "终端".as_bytes().to_vec()
    );
    assert!(encode_terminal_ime_event(&terminal, &Ime::Preedit("终".to_owned(), None)).is_empty());
    assert!(encode_terminal_ime_event(&terminal, &Ime::Disabled).is_empty());
}
