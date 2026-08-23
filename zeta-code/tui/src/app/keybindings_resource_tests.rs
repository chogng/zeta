use std::fs;
use std::time::Duration;
use std::time::Instant;

use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyModifiers;

use super::AppKeybindingsResource;
use super::AppKeybindingsResourcePoll;
use crate::app::keymap::AppKeymap;
use crate::app::keymap::AppKeymapAction;
use crate::app::keymap::AppKeymapContext;

fn context() -> AppKeymapContext {
    AppKeymapContext {
        accepts_input: true,
        has_selection: false,
        composer_empty: true,
        is_press: true,
    }
}

#[test]
fn valid_updates_replace_user_rules_and_missing_resource_restores_builtins() {
    let path = temporary_resource("valid");
    fs::write(
        &path,
        br#"[{"key":"ctrl+y","command":"zetaCode.action.copyLastResponse"}]"#,
    )
    .unwrap();
    let started = Instant::now();
    let mut resource = AppKeybindingsResource::new(path.clone(), started);
    let mut keymap = AppKeymap::default();

    assert_eq!(
        resource.poll(started, &mut keymap),
        AppKeybindingsResourcePoll::Updated
    );
    assert_eq!(
        keymap.resolve_single(
            &KeyEvent::new(KeyCode::Char('y'), KeyModifiers::CONTROL),
            context(),
        ),
        Some(AppKeymapAction::CopyLastResponse)
    );

    fs::remove_file(&path).unwrap();
    assert_eq!(
        resource.poll(started + Duration::from_secs(1), &mut keymap),
        AppKeybindingsResourcePoll::Updated
    );
    assert_eq!(
        keymap.resolve_single(
            &KeyEvent::new(KeyCode::Char('y'), KeyModifiers::CONTROL),
            context(),
        ),
        None
    );
}

#[test]
fn rejected_update_preserves_the_last_valid_keymap() {
    let path = temporary_resource("rejected");
    fs::write(
        &path,
        br#"[{"key":"ctrl+y","command":"zetaCode.action.copyLastResponse"}]"#,
    )
    .unwrap();
    let started = Instant::now();
    let mut resource = AppKeybindingsResource::new(path.clone(), started);
    let mut keymap = AppKeymap::default();
    resource.poll(started, &mut keymap);

    fs::write(&path, br#"[{"key":"ctrl+k escape","command":null}]"#).unwrap();
    assert!(matches!(
        resource.poll(started + Duration::from_secs(1), &mut keymap),
        AppKeybindingsResourcePoll::Rejected(message) if message.contains("plain Escape")
    ));
    assert_eq!(
        keymap.resolve_single(
            &KeyEvent::new(KeyCode::Char('y'), KeyModifiers::CONTROL),
            context(),
        ),
        Some(AppKeymapAction::CopyLastResponse)
    );

    fs::remove_file(path).unwrap();
}

fn temporary_resource(label: &str) -> std::path::PathBuf {
    let unique = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!(
        "zeta-code-keybindings-{label}-{}-{unique}.json",
        std::process::id(),
    ))
}
