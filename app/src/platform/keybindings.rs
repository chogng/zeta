use std::sync::OnceLock;

use zeta_keybinding::BindingPriority;
use zeta_keybinding::BindingSet;
use zeta_keybinding::BindingSource;
use zeta_keybinding::Chord;
use zeta_keybinding::ContextExpression;
use zeta_keybinding::ContextValue;
use zeta_keybinding::HostPlatform;
use zeta_keybinding::KeySequence;
use zeta_keybinding::ShortcutModifiers;
use zeta_keybindings_host::KeybindingCatalog;
use zeta_keybindings_host::KeybindingResolution;
use zeta_keybindings_host::Keybindings;
#[cfg(test)]
use zeta_keybindings_host::UserBinding;
#[cfg(test)]
use zeta_keybindings_host::UserBindingTarget;

use zeta_commands::AppCommandId;

pub(crate) type NativeKeybindings = Keybindings<NativeKeybindingCatalog>;
pub(crate) type NativeKeybindingResolution = KeybindingResolution<AppCommandId>;
#[cfg(test)]
pub(crate) type NativeUserBinding = UserBinding<NativeKeybindingCatalog>;
#[cfg(test)]
pub(crate) type NativeUserBindingTarget = UserBindingTarget<AppCommandId>;
pub(crate) type KeybindingsResource =
    zeta_keybindings_host::KeybindingsResource<NativeKeybindingCatalog>;

pub(crate) use zeta_keybindings_host::KeybindingsResourcePoll;

/// Product context facts projected into the generic keybinding catalog.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct NativeKeybindingContext {
    facts: NativeKeybindingFacts,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct NativeKeybindingFacts {
    pub(crate) direct_terminal: bool,
    pub(crate) terminal_surface_visible: bool,
    pub(crate) tab_container_visible: bool,
    pub(crate) inspector_visible: bool,
    pub(crate) file_search_visible: bool,
    pub(crate) composer_route: &'static str,
}

impl NativeKeybindingContext {
    #[cfg(test)]
    pub(crate) const fn text_input() -> Self {
        Self {
            facts: NativeKeybindingFacts {
                direct_terminal: false,
                terminal_surface_visible: false,
                tab_container_visible: false,
                inspector_visible: false,
                file_search_visible: false,
                composer_route: "agent",
            },
        }
    }

    #[cfg(test)]
    pub(crate) const fn direct_terminal() -> Self {
        Self {
            facts: NativeKeybindingFacts {
                direct_terminal: true,
                terminal_surface_visible: true,
                tab_container_visible: false,
                inspector_visible: false,
                file_search_visible: false,
                composer_route: "agent",
            },
        }
    }

    pub(crate) const fn from_facts(facts: NativeKeybindingFacts) -> Self {
        Self { facts }
    }

    pub(super) fn supports_key(key: &str) -> bool {
        matches!(
            key,
            "textInputFocus"
                | "terminalFocus"
                | "agentSurfaceVisible"
                | "terminalSurfaceVisible"
                | "tabContainerVisible"
                | "inspectorVisible"
                | "fileSearchVisible"
                | "composerRoute"
        )
    }

    fn value(self, key: &str) -> Option<ContextValue> {
        match key {
            "textInputFocus" => Some(ContextValue::Boolean(!self.facts.direct_terminal)),
            "terminalFocus" => Some(ContextValue::Boolean(self.facts.direct_terminal)),
            "agentSurfaceVisible" => {
                Some(ContextValue::Boolean(!self.facts.terminal_surface_visible))
            }
            "terminalSurfaceVisible" => {
                Some(ContextValue::Boolean(self.facts.terminal_surface_visible))
            }
            "tabContainerVisible" => Some(ContextValue::Boolean(self.facts.tab_container_visible)),
            "inspectorVisible" => Some(ContextValue::Boolean(self.facts.inspector_visible)),
            "fileSearchVisible" => Some(ContextValue::Boolean(self.facts.file_search_visible)),
            "composerRoute" => Some(ContextValue::String(self.facts.composer_route.to_owned())),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum NativeBindingCondition {
    Always,
    TextInput,
    DirectTerminal,
    Expression(ContextExpression),
}

#[derive(Debug, Eq, PartialEq)]
pub(crate) struct NativeKeybindingCatalog;

impl KeybindingCatalog for NativeKeybindingCatalog {
    type Command = AppCommandId;
    type Condition = NativeBindingCondition;
    type Context = NativeKeybindingContext;

    fn builtin_bindings(platform: HostPlatform) -> BindingSet<Self::Condition, Self::Command> {
        builtin_bindings(platform)
    }

    fn default_keybinding(command: Self::Command) -> Option<&'static KeySequence> {
        default_keybinding(command)
    }

    fn command_id(command: Self::Command) -> &'static str {
        command.id()
    }

    fn command_from_id(id: &str) -> Option<Self::Command> {
        AppCommandId::bindable_from_id(id)
    }

    fn parse_condition(source: Option<&str>) -> Result<Self::Condition, String> {
        let Some(source) = source else {
            return Ok(NativeBindingCondition::Always);
        };
        let expression = ContextExpression::parse(source).map_err(|error| error.to_string())?;
        if let Some(key) = expression
            .referenced_keys()
            .into_iter()
            .find(|key| !NativeKeybindingContext::supports_key(key))
        {
            return Err(format!("unknown context key `{key}`"));
        }
        Ok(NativeBindingCondition::Expression(expression))
    }

    fn condition_matches(condition: &Self::Condition, context: &Self::Context) -> bool {
        condition_matches(condition, context)
    }
}

fn condition_matches(
    condition: &NativeBindingCondition,
    context: &NativeKeybindingContext,
) -> bool {
    match condition {
        NativeBindingCondition::Always => true,
        NativeBindingCondition::TextInput => !context.facts.direct_terminal,
        NativeBindingCondition::DirectTerminal => context.facts.direct_terminal,
        NativeBindingCondition::Expression(expression) => {
            expression.evaluate(|key| context.value(key))
        }
    }
}

fn default_keybinding(command: AppCommandId) -> Option<&'static KeySequence> {
    static TOGGLE_TERMINAL: OnceLock<KeySequence> = OnceLock::new();
    static COPY: OnceLock<KeySequence> = OnceLock::new();
    static PASTE: OnceLock<KeySequence> = OnceLock::new();
    static SAVE: OnceLock<KeySequence> = OnceLock::new();
    static SPLIT_HORIZONTAL: OnceLock<KeySequence> = OnceLock::new();
    static SPLIT_VERTICAL: OnceLock<KeySequence> = OnceLock::new();
    match command {
        AppCommandId::ToggleTerminalSurface => Some(TOGGLE_TERMINAL.get_or_init(|| {
            KeySequence::single(
                Chord::logical("j", ShortcutModifiers::primary()).expect("builtin key"),
            )
        })),
        AppCommandId::OpenKeyboardShortcuts => Some(static_keyboard_shortcuts_binding()),
        AppCommandId::Copy => Some(COPY.get_or_init(|| {
            KeySequence::single(
                Chord::logical("c", ShortcutModifiers::primary()).expect("builtin key"),
            )
        })),
        AppCommandId::Paste => Some(PASTE.get_or_init(|| {
            KeySequence::single(
                Chord::logical("v", ShortcutModifiers::primary()).expect("builtin key"),
            )
        })),
        AppCommandId::Save => Some(SAVE.get_or_init(|| {
            KeySequence::single(
                Chord::logical("s", ShortcutModifiers::primary()).expect("builtin key"),
            )
        })),
        AppCommandId::SplitTerminalHorizontal => Some(SPLIT_HORIZONTAL.get_or_init(|| {
            KeySequence::single(
                Chord::logical("\\", ShortcutModifiers::primary()).expect("builtin key"),
            )
        })),
        AppCommandId::SplitTerminalVertical => Some(SPLIT_VERTICAL.get_or_init(|| {
            KeySequence::single(
                Chord::logical("\\", ShortcutModifiers::primary().with_shift())
                    .expect("builtin key"),
            )
        })),
        AppCommandId::OpenLanguageServerSettings
        | AppCommandId::ManageRemoteTunnels
        | AppCommandId::ToggleTabContainer
        | AppCommandId::ToggleWorkspacePane
        | AppCommandId::ActivateSessionTab
        | AppCommandId::AddSession
        | AppCommandId::ShowAgentChanges
        | AppCommandId::ShowAgentFiles
        | AppCommandId::RefreshAgentFiles
        | AppCommandId::ToggleAgentFileSearch
        | AppCommandId::PinSession
        | AppCommandId::CloseSession
        | AppCommandId::RenameSession
        | AppCommandId::ForkSession
        | AppCommandId::PickExecutionLocation
        | AppCommandId::PickWorkingDirectory
        | AppCommandId::PickGitBranch
        | AppCommandId::ShowWorkspaceDiff
        | AppCommandId::FocusNextPane
        | AppCommandId::FocusPreviousPane
        | AppCommandId::ClosePane => None,
    }
}

fn static_keyboard_shortcuts_binding() -> &'static KeySequence {
    static KEYBOARD_SHORTCUTS: OnceLock<KeySequence> = OnceLock::new();
    KEYBOARD_SHORTCUTS.get_or_init(|| {
        KeySequence::single(Chord::logical(",", ShortcutModifiers::primary()).expect("builtin key"))
    })
}

fn builtin_bindings(platform: HostPlatform) -> BindingSet<NativeBindingCondition, AppCommandId> {
    let mut bindings = BindingSet::default();
    register(
        &mut bindings,
        "j",
        ShortcutModifiers::primary(),
        AppCommandId::ToggleTerminalSurface,
        NativeBindingCondition::Always,
    );
    register(
        &mut bindings,
        ",",
        ShortcutModifiers::primary(),
        AppCommandId::OpenKeyboardShortcuts,
        NativeBindingCondition::Always,
    );
    register_text_input_clipboard(&mut bindings);
    register(
        &mut bindings,
        "s",
        ShortcutModifiers::primary(),
        AppCommandId::Save,
        NativeBindingCondition::Always,
    );
    register(
        &mut bindings,
        "\\",
        ShortcutModifiers::primary(),
        AppCommandId::SplitTerminalHorizontal,
        NativeBindingCondition::Always,
    );
    register(
        &mut bindings,
        "\\",
        ShortcutModifiers::primary().with_shift(),
        AppCommandId::SplitTerminalVertical,
        NativeBindingCondition::Always,
    );
    register_direct_terminal_clipboard(&mut bindings, platform);
    bindings
}

fn register_text_input_clipboard(bindings: &mut BindingSet<NativeBindingCondition, AppCommandId>) {
    register(
        bindings,
        "c",
        ShortcutModifiers::primary(),
        AppCommandId::Copy,
        NativeBindingCondition::TextInput,
    );
    register(
        bindings,
        "v",
        ShortcutModifiers::primary(),
        AppCommandId::Paste,
        NativeBindingCondition::TextInput,
    );
    register(
        bindings,
        "c",
        ShortcutModifiers::primary().with_shift(),
        AppCommandId::Copy,
        NativeBindingCondition::TextInput,
    );
    register(
        bindings,
        "v",
        ShortcutModifiers::primary().with_shift(),
        AppCommandId::Paste,
        NativeBindingCondition::TextInput,
    );
}

fn register_direct_terminal_clipboard(
    bindings: &mut BindingSet<NativeBindingCondition, AppCommandId>,
    platform: HostPlatform,
) {
    let modifiers = if platform == HostPlatform::MacOs {
        ShortcutModifiers::primary()
    } else {
        ShortcutModifiers::control().with_shift()
    };
    register(
        bindings,
        "c",
        modifiers,
        AppCommandId::Copy,
        NativeBindingCondition::DirectTerminal,
    );
    register(
        bindings,
        "v",
        modifiers,
        AppCommandId::Paste,
        NativeBindingCondition::DirectTerminal,
    );
}

fn register(
    bindings: &mut BindingSet<NativeBindingCondition, AppCommandId>,
    key: &str,
    modifiers: ShortcutModifiers,
    command: AppCommandId,
    condition: NativeBindingCondition,
) {
    let chord = Chord::logical(key, modifiers).expect("builtin shortcut key must be valid");
    bindings.register_command(
        KeySequence::single(chord),
        command,
        condition,
        BindingSource::Builtin,
        BindingPriority::NORMAL,
    );
}

#[cfg(test)]
pub(crate) fn compile_user_bindings(
    contents: &[u8],
    platform: HostPlatform,
) -> Result<Vec<NativeUserBinding>, zeta_keybindings_host::KeybindingsResourceError> {
    zeta_keybindings_host::compile_user_bindings::<NativeKeybindingCatalog>(contents, platform)
}

#[cfg(test)]
pub(crate) fn binding_diagnostics(
    rules: &[NativeUserBinding],
    platform: HostPlatform,
) -> Vec<String> {
    zeta_keybindings_host::binding_diagnostics::<NativeKeybindingCatalog>(rules, platform)
}

#[cfg(test)]
#[path = "keybindings_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "keybindings_resource_tests.rs"]
mod resource_tests;
