use super::{
    KeybindingsResource, KeybindingsResourcePoll, binding_diagnostics, compile_user_bindings,
};
use crate::keybindings::{
    WorkbenchKeybindingContext, WorkbenchKeybindingResolution, WorkbenchKeybindings,
};
use std::fs;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use zeta_commands::AppCommandId;
use zeta_keybinding::{HostPlatform, KeyStroke, LogicalKey, Modifiers, parse_key_sequence};

#[test]
fn compiles_platform_overrides_chords_and_blockers() {
    let rules = compile_user_bindings(
        br#"[
            {
                "key": "ctrl+k ctrl+c",
                "mac": "primary+k primary+c",
                "command": "workbench.action.toggleTabContainer",
                "when": "textInputFocus && composerRoute == 'agent'"
            },
            {
                "key": "ctrl+v",
                "command": null,
                "when": "terminalFocus"
            }
        ]"#,
        HostPlatform::MacOs,
    )
    .expect("valid resource");

    assert_eq!(rules.len(), 2);
}

#[test]
fn exact_duplicate_contexts_produce_a_non_fatal_conflict_diagnostic() {
    let rules = compile_user_bindings(
        br#"[
            {"key":"ctrl+k","command":"workbench.action.toggleTabContainer","when":"textInputFocus"},
            {"key":"ctrl+k","command":"workbench.action.toggleAuxiliaryBar","when":"textInputFocus"}
        ]"#,
        HostPlatform::Linux,
    )
    .expect("valid conflicting resource");

    let diagnostics = binding_diagnostics(&rules, HostPlatform::Linux);

    assert_eq!(diagnostics.len(), 1);
    assert!(diagnostics[0].contains("later rule wins"));
}

#[test]
fn rejects_unknown_commands_fields_and_context_keys() {
    assert!(
        compile_user_bindings(
            br#"[{"key":"ctrl+x","command":"missing.command"}]"#,
            HostPlatform::Linux
        )
        .is_err()
    );
    assert!(
        compile_user_bindings(
            br#"[{"key":"ctrl+x","command":null,"args":[]}]"#,
            HostPlatform::Linux
        )
        .is_err()
    );
    assert!(
        compile_user_bindings(
            br#"[{"key":"ctrl+x","mac":42,"command":null}]"#,
            HostPlatform::Linux
        )
        .is_err()
    );
    assert!(
        compile_user_bindings(
            br#"[{"key":"ctrl+x","command":null,"when":"composerMode == 'agent'"}]"#,
            HostPlatform::Linux
        )
        .is_err()
    );
}

#[test]
fn user_blocker_consumes_an_otherwise_unmatched_key() {
    let rules = compile_user_bindings(
        br#"[{"key":"ctrl+x","command":null,"when":"terminalFocus"}]"#,
        HostPlatform::Linux,
    )
    .expect("valid blocker");
    let mut keybindings = WorkbenchKeybindings::for_platform(HostPlatform::Linux);
    keybindings.replace_user_bindings(rules);

    assert_eq!(
        keybindings.resolve_stroke_at(
            &stroke("x"),
            &WorkbenchKeybindingContext::direct_terminal(),
            Instant::now(),
        ),
        WorkbenchKeybindingResolution::Consumed
    );
}

#[test]
fn invalid_hot_update_preserves_the_previous_complete_rule_set() {
    let root = std::env::temp_dir().join(format!(
        "zeta-keybindings-resource-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time")
            .as_nanos()
    ));
    fs::create_dir_all(&root).expect("temporary root");
    let path = root.join("keybindings.json");
    fs::write(
        &path,
        br#"[{"key":"ctrl+k ctrl+c","command":"workbench.action.toggleTabContainer"}]"#,
    )
    .expect("write valid resource");
    let now = Instant::now();
    let mut resource = KeybindingsResource::new(path.clone(), HostPlatform::Linux, now);
    let mut keybindings = WorkbenchKeybindings::for_platform(HostPlatform::Linux);
    assert_eq!(
        resource.poll(now, &mut keybindings),
        KeybindingsResourcePoll::Updated
    );
    assert_eq!(
        keybindings.resolve_stroke_at(&stroke("k"), &WorkbenchKeybindingContext::text_input(), now),
        WorkbenchKeybindingResolution::Consumed
    );
    assert_eq!(
        keybindings.resolve_stroke_at(
            &stroke("c"),
            &WorkbenchKeybindingContext::text_input(),
            now + Duration::from_millis(10)
        ),
        WorkbenchKeybindingResolution::Command(AppCommandId::ToggleTabContainer)
    );

    fs::write(&path, b"{").expect("write invalid resource");
    assert!(matches!(
        resource.poll(now + Duration::from_secs(1), &mut keybindings),
        KeybindingsResourcePoll::Rejected(_)
    ));
    assert_eq!(resource.diagnostics().len(), 1);
    assert_eq!(
        keybindings.resolve_stroke_at(
            &stroke("k"),
            &WorkbenchKeybindingContext::text_input(),
            now + Duration::from_secs(2)
        ),
        WorkbenchKeybindingResolution::Consumed
    );
    let _ = fs::remove_file(path);
    let _ = fs::remove_dir(root);
}

#[test]
fn recorder_update_atomically_replaces_the_commands_user_rule() {
    let root = temporary_root();
    fs::create_dir_all(&root).expect("temporary root");
    let path = root.join("keybindings.json");
    fs::write(
        &path,
        br#"[{"key":"ctrl+b","command":"workbench.action.toggleTabContainer"}]"#,
    )
    .expect("existing resource");
    let now = Instant::now();
    let mut resource = KeybindingsResource::new(path.clone(), HostPlatform::Linux, now);
    let sequence = parse_key_sequence("primary+k primary+b").expect("recorded shortcut");

    resource
        .update_command_binding(AppCommandId::ToggleTabContainer, &sequence, now)
        .expect("save recording");

    let value: serde_json::Value =
        serde_json::from_slice(&fs::read(&path).expect("saved resource")).expect("valid JSON");
    assert_eq!(value.as_array().expect("array").len(), 1);
    assert_eq!(value[0]["key"], "primary+k primary+b");
    let _ = fs::remove_file(path);
    let _ = fs::remove_dir(root);
}

fn stroke(key: &str) -> KeyStroke {
    KeyStroke::new(
        LogicalKey::new(key).expect("logical key"),
        None,
        Modifiers::none().with_control(),
    )
}

fn temporary_root() -> std::path::PathBuf {
    std::env::temp_dir().join(format!(
        "zeta-keybindings-resource-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time")
            .as_nanos()
    ))
}
