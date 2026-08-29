use std::fs;
use std::time::Duration;
use std::time::Instant;

use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyModifiers;

use super::KeymapEdit;
use super::KeymapEditIntent;
use super::KeymapEditKind;
use super::KeymapResource;
use super::KeymapResourcePoll;
use crate::keymap::AppKeymap;
use crate::keymap::AppKeymapAction;
use crate::keymap::AppKeymapContext;

fn context() -> AppKeymapContext {
    AppKeymapContext {
        accepts_input: true,
        has_selection: false,
        chat_input_empty: true,
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
    let mut resource = KeymapResource::new(path.clone(), started);
    let mut keymap = AppKeymap::default();

    assert_eq!(
        resource.poll(started, &mut keymap),
        KeymapResourcePoll::Updated
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
        KeymapResourcePoll::Updated
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
    let mut resource = KeymapResource::new(path.clone(), started);
    let mut keymap = AppKeymap::default();
    resource.poll(started, &mut keymap);

    fs::write(&path, br#"[{"key":"ctrl+k escape","command":null}]"#).unwrap();
    assert!(matches!(
        resource.poll(started + Duration::from_secs(1), &mut keymap),
        KeymapResourcePoll::Rejected(message) if message.contains("plain Escape")
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

#[test]
fn shortcut_edit_preserves_unrelated_rules_and_rejects_stale_revision() {
    let path = temporary_resource("edit");
    fs::write(
        &path,
        br#"[{"key":"ctrl+x","command":"zetaCode.action.cycleApprovalMode"}]"#,
    )
    .unwrap();
    let started = Instant::now();
    let mut resource = KeymapResource::new(path.clone(), started);
    let mut keymap = AppKeymap::default();
    assert_eq!(
        resource.poll(started, &mut keymap),
        KeymapResourcePoll::Updated
    );
    let edit = KeymapEdit {
        expected_revision: 1,
        command_id: "zetaCode.action.copyLastResponse".into(),
        kind: KeymapEditKind::Set {
            key: "ctrl+y".into(),
            intent: KeymapEditIntent::AddAlternate,
        },
    };

    let notice = resource
        .apply_edit(&edit, &mut keymap, started + Duration::from_millis(10))
        .unwrap();

    assert_eq!(notice, "Added user shortcut `ctrl+y`.");
    let saved: serde_json::Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    assert_eq!(saved.as_array().unwrap().len(), 2);
    assert_eq!(
        keymap.resolve_single(
            &KeyEvent::new(KeyCode::Char('y'), KeyModifiers::CONTROL),
            context(),
        ),
        Some(AppKeymapAction::CopyLastResponse)
    );
    assert!(
        resource
            .apply_edit(&edit, &mut keymap, started + Duration::from_millis(20))
            .unwrap_err()
            .contains("changed after the editor opened")
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
