use super::GlobalKeymap;
use super::GlobalKeymapAction;
use super::GlobalKeymapContext;
use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyEventKind;
use crossterm::event::KeyModifiers;

fn context() -> GlobalKeymapContext {
    GlobalKeymapContext {
        accepts_input: true,
        has_selection: false,
        composer_empty: true,
        is_press: true,
    }
}

#[test]
fn crossterm_adapter_normalizes_backtab_and_character_case() {
    let keymap = GlobalKeymap::default();

    assert_eq!(
        keymap.resolve(
            &KeyEvent::new(KeyCode::BackTab, KeyModifiers::SHIFT),
            context(),
        ),
        Some(GlobalKeymapAction::CycleApprovalMode)
    );
    assert_eq!(
        keymap.resolve(
            &KeyEvent::new(KeyCode::Char('C'), KeyModifiers::CONTROL),
            context(),
        ),
        Some(GlobalKeymapAction::InterruptOrQuit)
    );
}

#[test]
fn root_conditions_preserve_input_selection_and_press_boundaries() {
    let keymap = GlobalKeymap::default();
    let backtab = KeyEvent::new(KeyCode::BackTab, KeyModifiers::SHIFT);
    let escape = KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE);

    assert_eq!(
        keymap.resolve(
            &backtab,
            GlobalKeymapContext {
                has_selection: true,
                ..context()
            },
        ),
        None
    );
    assert_eq!(
        keymap.resolve(
            &escape,
            GlobalKeymapContext {
                accepts_input: false,
                ..context()
            },
        ),
        None
    );
    assert_eq!(
        keymap.resolve(
            &KeyEvent::new_with_kind(KeyCode::BackTab, KeyModifiers::SHIFT, KeyEventKind::Repeat,),
            GlobalKeymapContext {
                is_press: false,
                ..context()
            },
        ),
        None
    );
}

#[test]
fn control_d_only_matches_an_empty_composer() {
    let keymap = GlobalKeymap::default();
    let control_d = KeyEvent::new(KeyCode::Char('d'), KeyModifiers::CONTROL);

    assert_eq!(
        keymap.resolve(
            &control_d,
            GlobalKeymapContext {
                composer_empty: false,
                ..context()
            },
        ),
        None
    );
    assert_eq!(
        keymap.resolve(&control_d, context()),
        Some(GlobalKeymapAction::InterruptOrQuit)
    );
}

#[test]
fn unsupported_hyper_modifier_is_not_silently_dropped() {
    let keymap = GlobalKeymap::default();

    assert_eq!(
        keymap.resolve(
            &KeyEvent::new(
                KeyCode::Char('c'),
                KeyModifiers::CONTROL | KeyModifiers::HYPER
            ),
            context(),
        ),
        None
    );
}
