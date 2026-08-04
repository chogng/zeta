use std::fs;
use std::time::Duration;
use std::time::Instant;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

use zeta_keybinding::BindingPriority;
use zeta_keybinding::BindingSet;
use zeta_keybinding::BindingSource;
use zeta_keybinding::Chord;
use zeta_keybinding::HostPlatform;
use zeta_keybinding::KeySequence;
use zeta_keybinding::KeyStroke;
use zeta_keybinding::Modifiers;

use super::KeybindingCatalog;
use super::KeybindingResolution;
use super::Keybindings;
use super::KeybindingsResource;
use super::KeybindingsResourcePoll;
use super::UserBinding;
use super::UserBindingTarget;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Command {
    Toggle,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum Condition {
    Always,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Context;

struct Catalog;

impl KeybindingCatalog for Catalog {
    type Command = Command;
    type Condition = Condition;
    type Context = Context;

    fn builtin_bindings(_platform: HostPlatform) -> BindingSet<Self::Condition, Self::Command> {
        let mut bindings = BindingSet::default();
        bindings.register_command(
            KeySequence::single(
                Chord::logical("k", zeta_keybinding::ShortcutModifiers::control()).expect("key"),
            ),
            Command::Toggle,
            Condition::Always,
            BindingSource::Builtin,
            BindingPriority::NORMAL,
        );
        bindings
    }

    fn default_keybinding(_command: Self::Command) -> Option<&'static KeySequence> {
        None
    }

    fn command_id(_command: Self::Command) -> &'static str {
        "test.toggle"
    }

    fn command_from_id(id: &str) -> Option<Self::Command> {
        (id == "test.toggle").then_some(Command::Toggle)
    }

    fn parse_condition(source: Option<&str>) -> Result<Self::Condition, String> {
        match source {
            None => Ok(Condition::Always),
            Some("always") => Ok(Condition::Always),
            Some(source) => Err(format!("unknown condition `{source}`")),
        }
    }

    fn condition_matches(_condition: &Self::Condition, _context: &Self::Context) -> bool {
        true
    }
}

#[test]
fn resolves_builtin_and_user_chord_rules() {
    let mut keybindings = Keybindings::<Catalog>::for_platform(HostPlatform::Linux);
    let context = Context;
    assert_eq!(
        keybindings.resolve_stroke_at(
            &stroke("k", Modifiers::none().with_control()),
            &context,
            Instant::now(),
        ),
        KeybindingResolution::Command(Command::Toggle)
    );

    keybindings.replace_user_bindings(vec![UserBinding {
        keybinding: zeta_keybinding::parse_key_sequence("ctrl+x ctrl+c").expect("chord"),
        target: UserBindingTarget::Command(Command::Toggle),
        when: Condition::Always,
        when_source: None,
    }]);
    let now = Instant::now();
    assert_eq!(
        keybindings.resolve_stroke_at(
            &stroke("x", Modifiers::none().with_control()),
            &context,
            now,
        ),
        KeybindingResolution::Consumed
    );
    assert_eq!(
        keybindings.resolve_stroke_at(
            &stroke("c", Modifiers::none().with_control()),
            &context,
            now + Duration::from_millis(100),
        ),
        KeybindingResolution::Command(Command::Toggle)
    );
}

#[test]
fn rejected_resource_does_not_replace_the_previous_rule_set() {
    let root = temporary_root();
    fs::create_dir_all(&root).expect("temporary root");
    let path = root.join("keybindings.json");
    fs::write(&path, br#"[{"key":"ctrl+x","command":"test.toggle"}]"#).expect("valid resource");
    let now = Instant::now();
    let mut resource = KeybindingsResource::<Catalog>::new(path.clone(), HostPlatform::Linux, now);
    let mut keybindings = Keybindings::<Catalog>::default();
    assert_eq!(
        resource.poll(now, &mut keybindings),
        KeybindingsResourcePoll::Updated
    );
    fs::write(&path, b"{").expect("invalid resource");
    assert!(matches!(
        resource.poll(now + Duration::from_secs(1), &mut keybindings),
        KeybindingsResourcePoll::Rejected(_)
    ));
    let context = Context;
    assert_eq!(
        keybindings.resolve_stroke_at(
            &stroke("x", Modifiers::none().with_control()),
            &context,
            now + Duration::from_secs(2),
        ),
        KeybindingResolution::Command(Command::Toggle)
    );
    let _ = fs::remove_dir_all(root);
}

fn stroke(key: &str, modifiers: Modifiers) -> KeyStroke {
    KeyStroke::new(
        zeta_keybinding::LogicalKey::new(key).expect("logical key"),
        None,
        modifiers,
    )
}

fn temporary_root() -> std::path::PathBuf {
    std::env::temp_dir().join(format!(
        "zeta-keybindings-host-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time")
            .as_nanos()
    ))
}
