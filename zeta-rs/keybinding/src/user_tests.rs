use super::UserBindingTarget;
use super::UserBindingsError;
use super::compile_user_bindings;
use super::user_binding_diagnostics;
use crate::ContextExpression;
use crate::HostPlatform;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Command {
    Copy,
}

fn compile(
    contents: &[u8],
) -> Result<Vec<super::UserBinding<Command, ContextExpression>>, UserBindingsError> {
    compile_user_bindings(
        contents,
        HostPlatform::Linux,
        |id| (id == "copy").then_some(Command::Copy),
        |source| {
            source
                .map(ContextExpression::parse)
                .transpose()
                .map(|condition| condition.unwrap_or_else(ContextExpression::always))
                .map_err(|error| error.to_string())
        },
    )
}

#[test]
fn compiles_commands_blockers_conditions_and_platform_overrides() {
    let rules = compile(
        br#"[
            {"key":"primary+c","linux":"ctrl+c","command":"copy","when":"inputFocus"},
            {"key":"ctrl+x","command":null}
        ]"#,
    )
    .unwrap();

    assert_eq!(rules.len(), 2);
    assert_eq!(rules[0].target, UserBindingTarget::Command(Command::Copy));
    assert_eq!(rules[0].when_source.as_deref(), Some("inputFocus"));
    assert_eq!(rules[1].target, UserBindingTarget::Block);
}

#[test]
fn rejects_unknown_commands_without_partially_compiling_the_resource() {
    let error = compile(br#"[{"key":"ctrl+x","command":"missing"}]"#).unwrap_err();

    assert_eq!(
        error,
        UserBindingsError::UnknownCommand {
            index: 1,
            command: "missing".into(),
        }
    );
}

#[test]
fn duplicate_diagnostics_preserve_later_rule_precedence() {
    let rules = compile(
        br#"[
            {"key":"ctrl+c","command":"copy"},
            {"key":"ctrl+c","command":null}
        ]"#,
    )
    .unwrap();

    let diagnostics = user_binding_diagnostics(&rules, HostPlatform::Linux);
    assert_eq!(diagnostics.len(), 1);
    assert!(diagnostics[0].contains("later rule wins"));
}
