use super::{
    NativeBindingCondition, NativeKeybindingContext, NativeKeybindingFacts,
    NativeKeybindingResolution, NativeKeybindings, NativeUserBinding, NativeUserBindingTarget,
};
use crate::commands::NativeCommand;
use std::time::{Duration, Instant};
use zeta_keybinding::{
    ContextExpression, HostPlatform, KeyStroke, LogicalKey, Modifiers, parse_key_sequence,
};

#[test]
fn text_inputs_use_portable_copy_and_paste() {
    let mut bindings = NativeKeybindings::for_platform(HostPlatform::Linux);

    assert_eq!(
        bindings.resolve_stroke(
            &stroke("c", Modifiers::none().with_control()),
            NativeKeybindingContext::text_input(),
        ),
        NativeKeybindingResolution::Command(NativeCommand::Copy)
    );
    assert_eq!(
        bindings.resolve_stroke(
            &stroke("v", Modifiers::none().with_control()),
            NativeKeybindingContext::text_input(),
        ),
        NativeKeybindingResolution::Command(NativeCommand::Paste)
    );
    assert_eq!(
        bindings.resolve_stroke(
            &stroke("c", Modifiers::none().with_control().with_shift()),
            NativeKeybindingContext::text_input(),
        ),
        NativeKeybindingResolution::Command(NativeCommand::Copy)
    );
}

#[test]
fn workspace_save_is_available_independently_from_the_focused_surface() {
    let mut bindings = NativeKeybindings::for_platform(HostPlatform::Linux);

    assert_eq!(
        bindings.resolve_stroke(
            &stroke("s", Modifiers::none().with_control()),
            NativeKeybindingContext::direct_terminal(),
        ),
        NativeKeybindingResolution::Command(NativeCommand::Save)
    );
}

#[test]
fn direct_terminal_preserves_unshifted_control_keys() {
    let mut bindings = NativeKeybindings::for_platform(HostPlatform::Linux);

    assert_eq!(
        bindings.resolve_stroke(
            &stroke("c", Modifiers::none().with_control()),
            NativeKeybindingContext::direct_terminal(),
        ),
        NativeKeybindingResolution::NoMatch
    );
    assert_eq!(
        bindings.resolve_stroke(
            &stroke("c", Modifiers::none().with_control().with_shift()),
            NativeKeybindingContext::direct_terminal(),
        ),
        NativeKeybindingResolution::Command(NativeCommand::Copy)
    );
}

#[test]
fn macos_direct_terminal_uses_command_modifier() {
    let mut bindings = NativeKeybindings::for_platform(HostPlatform::MacOs);

    assert_eq!(
        bindings.resolve_stroke(
            &stroke("c", Modifiers::none().with_meta()),
            NativeKeybindingContext::direct_terminal(),
        ),
        NativeKeybindingResolution::Command(NativeCommand::Copy)
    );
}

#[test]
fn chord_completes_or_expires_as_one_consumed_interaction() {
    let mut bindings = NativeKeybindings::for_platform(HostPlatform::Linux);
    bindings.replace_user_bindings(vec![NativeUserBinding {
        keybinding: parse_key_sequence("ctrl+k ctrl+c").expect("chord"),
        target: NativeUserBindingTarget::Command(NativeCommand::ToggleSessionSidebar),
        when: NativeBindingCondition::Always,
        when_source: None,
    }]);
    let now = Instant::now();

    assert_eq!(
        bindings.resolve_stroke_at(
            &stroke("k", Modifiers::none().with_control()),
            NativeKeybindingContext::text_input(),
            now,
        ),
        NativeKeybindingResolution::Consumed
    );
    assert_eq!(
        bindings.resolve_stroke_at(
            &stroke("c", Modifiers::none().with_control()),
            NativeKeybindingContext::text_input(),
            now + Duration::from_millis(100),
        ),
        NativeKeybindingResolution::Command(NativeCommand::ToggleSessionSidebar)
    );

    assert_eq!(
        bindings.resolve_stroke_at(
            &stroke("k", Modifiers::none().with_control()),
            NativeKeybindingContext::text_input(),
            now + Duration::from_secs(1),
        ),
        NativeKeybindingResolution::Consumed
    );
    assert!(bindings.advance_chord(now + Duration::from_secs(3)));
    assert_eq!(bindings.chord_deadline(), None);

    assert_eq!(
        bindings.resolve_stroke_at(
            &stroke("k", Modifiers::none().with_control()),
            NativeKeybindingContext::text_input(),
            now + Duration::from_secs(4),
        ),
        NativeKeybindingResolution::Consumed
    );
    assert_eq!(
        bindings.resolve_stroke_at(
            &stroke("x", Modifiers::none().with_control()),
            NativeKeybindingContext::text_input(),
            now + Duration::from_millis(4_100),
        ),
        NativeKeybindingResolution::Consumed
    );
    assert_eq!(bindings.chord_deadline(), None);
}

#[test]
fn native_context_exposes_boolean_and_string_facts_to_when_expressions() {
    let mut bindings = NativeKeybindings::for_platform(HostPlatform::Linux);
    bindings.replace_user_bindings(vec![NativeUserBinding {
        keybinding: parse_key_sequence("ctrl+k").expect("binding"),
        target: NativeUserBindingTarget::Command(NativeCommand::ToggleSessionSidebar),
        when: NativeBindingCondition::Expression(
            ContextExpression::parse(
                "agentSurfaceVisible && composerMode == 'agent' && !fileSearchVisible",
            )
            .expect("condition"),
        ),
        when_source: Some(
            "agentSurfaceVisible && composerMode == 'agent' && !fileSearchVisible".to_owned(),
        ),
    }]);
    let context = NativeKeybindingContext::from_facts(NativeKeybindingFacts {
        direct_terminal: false,
        terminal_surface_visible: false,
        session_sidebar_visible: false,
        agent_sidebar_visible: false,
        file_search_visible: false,
        composer_mode: "agent",
    });

    assert_eq!(
        bindings.resolve_stroke_at(
            &stroke("k", Modifiers::none().with_control()),
            context,
            Instant::now(),
        ),
        NativeKeybindingResolution::Command(NativeCommand::ToggleSessionSidebar)
    );
}

fn stroke(key: &str, modifiers: Modifiers) -> KeyStroke {
    KeyStroke::new(LogicalKey::new(key).expect("logical key"), None, modifiers)
}
