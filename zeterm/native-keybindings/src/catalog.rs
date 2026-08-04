use zeta_keybinding::BindingSet;
use zeta_keybinding::HostPlatform;
use zeta_keybinding::KeySequence;

/// Supplies product-specific commands, builtin rules, conditions, and context matching.
///
/// Implementations keep product vocabulary in the host while allowing the native keybinding
/// engine and user-resource lifecycle to remain independent of `NativeApp` and product transports.
pub trait KeybindingCatalog {
    type Command: Copy + Eq;
    type Condition: Clone + Eq;
    type Context;

    fn builtin_bindings(platform: HostPlatform) -> BindingSet<Self::Condition, Self::Command>;

    fn default_keybinding(command: Self::Command) -> Option<&'static KeySequence>;

    fn command_id(command: Self::Command) -> &'static str;

    fn command_from_id(id: &str) -> Option<Self::Command>;

    fn parse_condition(source: Option<&str>) -> Result<Self::Condition, String>;

    fn condition_matches(condition: &Self::Condition, context: &Self::Context) -> bool;
}
