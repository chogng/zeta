use super::{
    ProductBindingCondition, ProductKeybindingContext, ProductKeybindingFacts,
    ProductKeybindingResolution, ProductKeybindings, ProductUserBinding, ProductUserBindingTarget,
};
use std::time::{Duration, Instant};
use zeta_commands::AppCommandId;
use zeta_keybinding::{
    ContextExpression, HostPlatform, KeyStroke, LogicalKey, Modifiers, parse_key_sequence,
};

#[test]
fn text_inputs_use_portable_copy_and_paste() {
    let mut bindings = ProductKeybindings::for_platform(HostPlatform::Linux);

    assert_eq!(
        bindings.resolve_stroke(
            &stroke("c", Modifiers::none().with_control()),
            &ProductKeybindingContext::text_input(),
        ),
        ProductKeybindingResolution::Command(AppCommandId::Copy)
    );
    assert_eq!(
        bindings.resolve_stroke(
            &stroke("v", Modifiers::none().with_control()),
            &ProductKeybindingContext::text_input(),
        ),
        ProductKeybindingResolution::Command(AppCommandId::Paste)
    );
    assert_eq!(
        bindings.resolve_stroke(
            &stroke("c", Modifiers::none().with_control().with_shift()),
            &ProductKeybindingContext::text_input(),
        ),
        ProductKeybindingResolution::Command(AppCommandId::Copy)
    );
}

#[test]
fn workspace_save_is_available_independently_from_the_focused_surface() {
    let mut bindings = ProductKeybindings::for_platform(HostPlatform::Linux);

    assert_eq!(
        bindings.resolve_stroke(
            &stroke("s", Modifiers::none().with_control()),
            &ProductKeybindingContext::direct_terminal(),
        ),
        ProductKeybindingResolution::Command(AppCommandId::Save)
    );
}

#[test]
fn direct_terminal_preserves_unshifted_control_keys() {
    let mut bindings = ProductKeybindings::for_platform(HostPlatform::Linux);

    assert_eq!(
        bindings.resolve_stroke(
            &stroke("c", Modifiers::none().with_control()),
            &ProductKeybindingContext::direct_terminal(),
        ),
        ProductKeybindingResolution::NoMatch
    );
    assert_eq!(
        bindings.resolve_stroke(
            &stroke("c", Modifiers::none().with_control().with_shift()),
            &ProductKeybindingContext::direct_terminal(),
        ),
        ProductKeybindingResolution::Command(AppCommandId::Copy)
    );
}

#[test]
fn macos_direct_terminal_uses_command_modifier() {
    let mut bindings = ProductKeybindings::for_platform(HostPlatform::MacOs);

    assert_eq!(
        bindings.resolve_stroke(
            &stroke("c", Modifiers::none().with_meta()),
            &ProductKeybindingContext::direct_terminal(),
        ),
        ProductKeybindingResolution::Command(AppCommandId::Copy)
    );
}

#[test]
fn chord_completes_or_expires_as_one_consumed_interaction() {
    let mut bindings = ProductKeybindings::for_platform(HostPlatform::Linux);
    bindings.replace_user_bindings(vec![ProductUserBinding {
        keybinding: parse_key_sequence("ctrl+k ctrl+c").expect("chord"),
        target: ProductUserBindingTarget::Command(AppCommandId::ToggleTabContainer),
        when: ProductBindingCondition::Always,
        when_source: None,
    }]);
    let now = Instant::now();

    assert_eq!(
        bindings.resolve_stroke_at(
            &stroke("k", Modifiers::none().with_control()),
            &ProductKeybindingContext::text_input(),
            now,
        ),
        ProductKeybindingResolution::Consumed
    );
    assert_eq!(
        bindings.resolve_stroke_at(
            &stroke("c", Modifiers::none().with_control()),
            &ProductKeybindingContext::text_input(),
            now + Duration::from_millis(100),
        ),
        ProductKeybindingResolution::Command(AppCommandId::ToggleTabContainer)
    );

    assert_eq!(
        bindings.resolve_stroke_at(
            &stroke("k", Modifiers::none().with_control()),
            &ProductKeybindingContext::text_input(),
            now + Duration::from_secs(1),
        ),
        ProductKeybindingResolution::Consumed
    );
    assert!(bindings.advance_chord(now + Duration::from_secs(3)));
    assert_eq!(bindings.chord_deadline(), None);

    assert_eq!(
        bindings.resolve_stroke_at(
            &stroke("k", Modifiers::none().with_control()),
            &ProductKeybindingContext::text_input(),
            now + Duration::from_secs(4),
        ),
        ProductKeybindingResolution::Consumed
    );
    assert_eq!(
        bindings.resolve_stroke_at(
            &stroke("x", Modifiers::none().with_control()),
            &ProductKeybindingContext::text_input(),
            now + Duration::from_millis(4_100),
        ),
        ProductKeybindingResolution::Consumed
    );
    assert_eq!(bindings.chord_deadline(), None);
}

#[test]
fn desktop_context_exposes_boolean_and_string_facts_to_when_expressions() {
    let mut bindings = ProductKeybindings::for_platform(HostPlatform::Linux);
    bindings.replace_user_bindings(vec![ProductUserBinding {
        keybinding: parse_key_sequence("ctrl+k").expect("binding"),
        target: ProductUserBindingTarget::Command(AppCommandId::ToggleTabContainer),
        when: ProductBindingCondition::Expression(
            ContextExpression::parse(
                "agentSurfaceVisible && composerRoute == 'agent' && !fileSearchVisible",
            )
            .expect("condition"),
        ),
        when_source: Some(
            "agentSurfaceVisible && composerRoute == 'agent' && !fileSearchVisible".to_owned(),
        ),
    }]);
    let context = &ProductKeybindingContext::from_facts(ProductKeybindingFacts {
        direct_terminal: false,
        terminal_surface_visible: false,
        tab_container_visible: false,
        inspector_visible: false,
        file_search_visible: false,
        composer_route: "agent",
    });

    assert_eq!(
        bindings.resolve_stroke_at(
            &stroke("k", Modifiers::none().with_control()),
            context,
            Instant::now(),
        ),
        ProductKeybindingResolution::Command(AppCommandId::ToggleTabContainer)
    );
}

fn stroke(key: &str, modifiers: Modifiers) -> KeyStroke {
    KeyStroke::new(LogicalKey::new(key).expect("logical key"), None, modifiers)
}
