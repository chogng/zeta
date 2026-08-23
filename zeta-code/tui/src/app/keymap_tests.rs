use super::AppChordMatch;
use super::AppKeybindingSpec;
use super::AppKeymap;
use super::AppKeymapAction;
use super::AppKeymapCondition;
use super::AppKeymapContext;
use super::KEY_CHORD_TIMEOUT;
use super::app_keybinding_help_items;
use super::compile_app_user_bindings;
use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyEventKind;
use crossterm::event::KeyModifiers;
use std::time::Duration;
use std::time::Instant;
use zeta_keybinding::HostPlatform;

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

#[test]
fn user_rules_override_or_block_builtins_and_evaluate_context() {
    let rules = compile_app_user_bindings(
        br#"[
            {
                "key":"ctrl+o",
                "command":"zetaCode.action.cycleApprovalMode",
                "when":"inputFocus && !selectionVisible"
            },
            {"key":"ctrl+c","command":null}
        ]"#,
        HostPlatform::Linux,
    )
    .unwrap();
    let mut keymap = AppKeymap::default();
    keymap.replace_user_bindings(rules).unwrap();

    assert_eq!(
        keymap.resolve_single(&control('o'), context()),
        Some(AppKeymapAction::CycleApprovalMode)
    );
    assert_eq!(keymap.resolve_single(&control('c'), context()), None);
    assert_eq!(
        keymap.resolve_single(
            &control('o'),
            AppKeymapContext {
                has_selection: true,
                ..context()
            },
        ),
        Some(AppKeymapAction::CopyLastResponse)
    );
}

#[test]
fn user_chords_use_the_existing_pending_state_machine() {
    let rules = compile_app_user_bindings(
        br#"[{
            "key":"ctrl+k ctrl+y",
            "command":"zetaCode.action.copyLastResponse"
        }]"#,
        HostPlatform::Linux,
    )
    .unwrap();
    let mut keymap = AppKeymap::default();
    keymap.replace_user_bindings(rules).unwrap();
    let started = Instant::now();

    assert_eq!(
        keymap.route_chord(&control('k'), context(), started),
        AppChordMatch::Pending
    );
    assert_eq!(
        keymap.route_chord(
            &control('y'),
            context(),
            started + Duration::from_millis(100),
        ),
        AppChordMatch::Command(AppKeymapAction::CopyLastResponse)
    );
}

#[test]
fn user_resource_rejects_unknown_commands_and_context_keys() {
    let unknown_command = compile_app_user_bindings(
        br#"[{"key":"ctrl+y","command":"zetaCode.action.missing"}]"#,
        HostPlatform::Linux,
    )
    .unwrap_err();
    let unknown_context = compile_app_user_bindings(
        br#"[{
            "key":"ctrl+y",
            "command":"zetaCode.action.copyLastResponse",
            "when":"desktopFocus"
        }]"#,
        HostPlatform::Linux,
    )
    .unwrap_err();

    assert!(unknown_command.contains("unknown command"));
    assert!(unknown_context.contains("unknown context key"));
}
