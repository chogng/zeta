use crate::BindingPriority;
use crate::BindingSet;
use crate::BindingSource;
use crate::HostPlatform;
use crate::KeyStroke;
use crate::KeybindingResolver;
use crate::LogicalKey;
use crate::Modifiers;
use crate::ResolveResult;
use crate::parse_key_sequence;
use crate::serialize_key_sequence;
use serde::Deserialize;
use std::collections::HashSet;

#[derive(Deserialize)]
struct ConformanceFixtures {
    parser: Vec<ParserFixture>,
    resolver: Vec<ResolverFixture>,
}

#[derive(Deserialize)]
struct ParserFixture {
    input: String,
    valid: bool,
    canonical: Option<String>,
    chords: Option<usize>,
}

#[derive(Deserialize)]
struct ResolverFixture {
    name: String,
    #[serde(default)]
    context: Vec<String>,
    events: Vec<EventFixture>,
    rules: Vec<RuleFixture>,
    result: ResolverResultFixture,
}

#[derive(Deserialize)]
struct EventFixture {
    key: String,
    #[serde(default)]
    control: bool,
    #[serde(default)]
    shift: bool,
    #[serde(default)]
    alt: bool,
    #[serde(default)]
    meta: bool,
}

#[derive(Deserialize)]
struct RuleFixture {
    binding: String,
    command: Option<String>,
    #[serde(default)]
    block: bool,
    source: SourceFixture,
    priority: i32,
    when: Option<String>,
}

#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "camelCase")]
enum SourceFixture {
    Builtin,
    User,
}

#[derive(Deserialize)]
struct ResolverResultFixture {
    kind: String,
    command: Option<String>,
}

#[test]
fn shared_parser_fixtures_match_the_rust_implementation() {
    let fixtures = load_fixtures();
    for fixture in fixtures.parser {
        let parsed = parse_key_sequence(&fixture.input);
        assert_eq!(
            parsed.is_ok(),
            fixture.valid,
            "unexpected validity for {:?}",
            fixture.input
        );
        if let Ok(parsed) = parsed {
            assert_eq!(
                Some(parsed.chords().len()),
                fixture.chords,
                "unexpected chord count for {:?}",
                fixture.input
            );
            assert_eq!(
                Some(serialize_key_sequence(&parsed)),
                fixture.canonical,
                "unexpected canonical form for {:?}",
                fixture.input
            );
        }
    }
}

#[test]
fn shared_resolver_fixtures_match_the_rust_implementation() {
    let fixtures = load_fixtures();
    for fixture in fixtures.resolver {
        let mut bindings = BindingSet::default();
        for rule in fixture.rules {
            let keybinding = parse_key_sequence(&rule.binding)
                .unwrap_or_else(|error| panic!("{} has an invalid binding: {error}", fixture.name));
            let source = match rule.source {
                SourceFixture::Builtin => BindingSource::Builtin,
                SourceFixture::User => BindingSource::User,
            };
            if rule.block {
                bindings.register_blocker(
                    keybinding,
                    rule.when,
                    source,
                    BindingPriority::new(rule.priority),
                );
            } else {
                bindings.register_command(
                    keybinding,
                    rule.command
                        .unwrap_or_else(|| panic!("{} command rule has no command", fixture.name)),
                    rule.when,
                    source,
                    BindingPriority::new(rule.priority),
                );
            }
        }
        let context = fixture.context.into_iter().collect::<HashSet<_>>();
        let events = fixture
            .events
            .into_iter()
            .map(key_stroke)
            .collect::<Vec<_>>();
        let result = KeybindingResolver::new(&bindings, HostPlatform::Windows).resolve(
            &context,
            &events,
            |when, context| when.as_ref().is_none_or(|key| context.contains(key)),
        );
        let (kind, command) = match result {
            ResolveResult::NoMatch => ("noMatch", None),
            ResolveResult::PendingChord { .. } => ("pending", None),
            ResolveResult::Command { command, .. } => ("command", Some(command)),
            ResolveResult::Blocked { .. } => ("blocked", None),
        };
        assert_eq!(
            kind, fixture.result.kind,
            "unexpected kind for {}",
            fixture.name
        );
        assert_eq!(
            command, fixture.result.command,
            "unexpected command for {}",
            fixture.name
        );
    }
}

fn key_stroke(event: EventFixture) -> KeyStroke {
    let mut modifiers = Modifiers::none();
    if event.control {
        modifiers = modifiers.with_control();
    }
    if event.shift {
        modifiers = modifiers.with_shift();
    }
    if event.alt {
        modifiers = modifiers.with_alt();
    }
    if event.meta {
        modifiers = modifiers.with_meta();
    }
    KeyStroke::new(
        LogicalKey::new(event.key).expect("fixture event key must not be empty"),
        None,
        modifiers,
    )
}

fn load_fixtures() -> ConformanceFixtures {
    let contents = include_str!("../../../resources/keybindings/conformance.json");
    serde_json::from_str(contents).expect("shared keybinding fixtures must be valid JSON")
}
