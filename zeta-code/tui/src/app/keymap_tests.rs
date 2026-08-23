use super::AppChordMatch;
use super::AppKeybindingSpec;
use super::AppKeymap;
use super::AppKeymapAction;
use super::AppKeymapCondition;
use super::AppKeymapContext;
use super::KEY_CHORD_TIMEOUT;
use super::app_keybinding_help_items;
use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyEventKind;
use crossterm::event::KeyModifiers;
use std::time::Duration;
use std::time::Instant;

fn context() -> AppKeymapContext {
    AppKeymapContext {
        accepts_input: true,
        has_selection: false,
        composer_empty: true,
        is_press: true,
    }
}

fn chord_keymap() -> AppKeymap {
    AppKeymap::from_specs(&[AppKeybindingSpec {
        keybinding: "ctrl+k ctrl+o",
        action: AppKeymapAction::CopyLastResponse,
        condition: AppKeymapCondition::Always,
        help_label: "Ctrl-K Ctrl-O",
        help_description: "test chord",
    }])
}

fn control(character: char) -> KeyEvent {
    KeyEvent::new(KeyCode::Char(character), KeyModifiers::CONTROL)
}

#[test]
fn crossterm_adapter_normalizes_backtab_and_character_case() {
    let keymap = AppKeymap::default();

    assert_eq!(
        keymap.resolve_single(
            &KeyEvent::new(KeyCode::BackTab, KeyModifiers::SHIFT),
            context(),
        ),
        Some(AppKeymapAction::CycleApprovalMode)
    );
    assert_eq!(
        keymap.resolve_single(
            &KeyEvent::new(KeyCode::Char('C'), KeyModifiers::CONTROL),
            context(),
        ),
        Some(AppKeymapAction::InterruptOrQuit)
    );
}

#[test]
fn root_conditions_preserve_input_selection_and_press_boundaries() {
    let keymap = AppKeymap::default();
    let backtab = KeyEvent::new(KeyCode::BackTab, KeyModifiers::SHIFT);
    let escape = KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE);

    assert_eq!(
        keymap.resolve_single(
            &backtab,
            AppKeymapContext {
                has_selection: true,
                ..context()
            },
        ),
        None
    );
    assert_eq!(
        keymap.resolve_single(
            &escape,
            AppKeymapContext {
                accepts_input: false,
                ..context()
            },
        ),
        None
    );
    assert_eq!(
        keymap.resolve_single(
            &KeyEvent::new_with_kind(KeyCode::BackTab, KeyModifiers::SHIFT, KeyEventKind::Repeat),
            AppKeymapContext {
                is_press: false,
                ..context()
            },
        ),
        None
    );
}

#[test]
fn control_d_only_matches_an_empty_composer() {
    let keymap = AppKeymap::default();
    let control_d = control('d');

    assert_eq!(
        keymap.resolve_single(
            &control_d,
            AppKeymapContext {
                composer_empty: false,
                ..context()
            },
        ),
        None
    );
    assert_eq!(
        keymap.resolve_single(&control_d, context()),
        Some(AppKeymapAction::InterruptOrQuit)
    );
}

#[test]
fn unsupported_hyper_modifier_is_not_silently_dropped() {
    let keymap = AppKeymap::default();

    assert_eq!(
        keymap.resolve_single(
            &KeyEvent::new(
                KeyCode::Char('c'),
                KeyModifiers::CONTROL | KeyModifiers::HYPER
            ),
            context(),
        ),
        None
    );
}

#[test]
fn additional_modifiers_do_not_trigger_exact_app_bindings() {
    let keymap = AppKeymap::default();

    assert_eq!(
        keymap.resolve_single(
            &KeyEvent::new(
                KeyCode::Char('V'),
                KeyModifiers::CONTROL | KeyModifiers::SHIFT,
            ),
            context(),
        ),
        None,
    );
    assert_eq!(
        keymap.resolve_single(
            &KeyEvent::new(
                KeyCode::Char('c'),
                KeyModifiers::CONTROL | KeyModifiers::ALT,
            ),
            context(),
        ),
        None,
    );
}

#[test]
fn app_chord_tracks_prefix_and_dispatches_the_existing_action() {
    let mut keymap = chord_keymap();
    let started = Instant::now();

    assert_eq!(
        keymap.route_chord(&control('k'), context(), started),
        AppChordMatch::Pending
    );
    assert_eq!(keymap.pending_chord_label().as_deref(), Some("Ctrl+K"));
    assert_eq!(
        keymap.route_chord(
            &control('o'),
            context(),
            started + Duration::from_millis(100),
        ),
        AppChordMatch::Command(AppKeymapAction::CopyLastResponse)
    );
    assert_eq!(keymap.pending_chord_label(), None);
}

#[test]
fn wrong_second_key_passes_through_and_escape_cancels() {
    let mut keymap = chord_keymap();
    let started = Instant::now();

    assert_eq!(
        keymap.route_chord(&control('k'), context(), started),
        AppChordMatch::Pending
    );
    assert_eq!(
        keymap.route_chord(
            &KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE),
            context(),
            started + Duration::from_millis(100),
        ),
        AppChordMatch::PassThrough
    );
    assert_eq!(keymap.pending_chord_label(), None);

    assert_eq!(
        keymap.route_chord(&control('k'), context(), started),
        AppChordMatch::Pending
    );
    assert_eq!(
        keymap.route_chord(
            &KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE),
            context(),
            started + Duration::from_millis(100),
        ),
        AppChordMatch::Consumed
    );
    assert_eq!(keymap.pending_chord_label(), None);
}

#[test]
fn pending_chord_expires_on_timeout_or_context_change() {
    let mut keymap = chord_keymap();
    let started = Instant::now();

    keymap.route_chord(&control('k'), context(), started);
    assert!(keymap.expire(context(), started + KEY_CHORD_TIMEOUT));
    assert_eq!(keymap.pending_chord_label(), None);

    keymap.route_chord(&control('k'), context(), started);
    assert!(keymap.expire(
        AppKeymapContext {
            has_selection: true,
            ..context()
        },
        started + Duration::from_millis(100),
    ));
    assert_eq!(keymap.pending_chord_label(), None);
}

#[test]
#[should_panic(expected = "cannot use plain Escape")]
fn plain_escape_is_reserved_for_cancelling_pending_chords() {
    AppKeymap::from_specs(&[AppKeybindingSpec {
        keybinding: "ctrl+k escape",
        action: AppKeymapAction::CopyLastResponse,
        condition: AppKeymapCondition::Always,
        help_label: "Ctrl-K Esc",
        help_description: "invalid test chord",
    }]);
}

#[test]
#[should_panic(expected = "must use Control, Alt, Meta, primary, or a non-character prefix")]
fn plain_character_prefix_cannot_intercept_composer_text() {
    AppKeymap::from_specs(&[AppKeybindingSpec {
        keybinding: "k ctrl+o",
        action: AppKeymapAction::CopyLastResponse,
        condition: AppKeymapCondition::Always,
        help_label: "K Ctrl-O",
        help_description: "invalid test chord",
    }]);
}

#[test]
fn app_help_is_derived_from_every_registered_binding() {
    let labels = app_keybinding_help_items()
        .map(|(label, _)| label)
        .collect::<Vec<_>>();

    assert_eq!(
        labels,
        vec![
            "Shift-Tab",
            "Esc Esc",
            "Ctrl-V",
            "Ctrl-C",
            "Ctrl-D",
            "Ctrl-O",
            "Ctrl-Z",
        ],
    );
}
