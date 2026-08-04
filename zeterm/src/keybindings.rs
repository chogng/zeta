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

use zeta_commands::ZetermCommandId;

pub(crate) type NativeKeybindings = Keybindings<NativeKeybindingCatalog>;
pub(crate) type NativeKeybindingResolution = KeybindingResolution<ZetermCommandId>;
#[cfg(test)]
pub(crate) type NativeUserBinding = UserBinding<NativeKeybindingCatalog>;
#[cfg(test)]
pub(crate) type NativeUserBindingTarget = UserBindingTarget<ZetermCommandId>;
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
    pub(crate) session_sidebar_visible: bool,
    pub(crate) agent_sidebar_visible: bool,
    pub(crate) file_search_visible: bool,
    pub(crate) composer_mode: &'static str,
}

impl NativeKeybindingContext {
    #[cfg(test)]
    pub(crate) const fn text_input() -> Self {
        Self {
            facts: NativeKeybindingFacts {
                direct_terminal: false,
                terminal_surface_visible: false,
                session_sidebar_visible: false,
                agent_sidebar_visible: false,
                file_search_visible: false,
                composer_mode: "agent",
            },
        }
    }

    #[cfg(test)]
    pub(crate) const fn direct_terminal() -> Self {
        Self {
            facts: NativeKeybindingFacts {
                direct_terminal: true,
                terminal_surface_visible: true,
                session_sidebar_visible: false,
                agent_sidebar_visible: false,
                file_search_visible: false,
                composer_mode: "agent",
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
                | "sessionSidebarVisible"
                | "agentSidebarVisible"
                | "fileSearchVisible"
                | "composerMode"
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
            "sessionSidebarVisible" => {
                Some(ContextValue::Boolean(self.facts.session_sidebar_visible))
            }
            "agentSidebarVisible" => Some(ContextValue::Boolean(self.facts.agent_sidebar_visible)),
            "fileSearchVisible" => Some(ContextValue::Boolean(self.facts.file_search_visible)),
            "composerMode" => Some(ContextValue::String(self.facts.composer_mode.to_owned())),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) enum NativeBindingCondition {
    Always,
    TextInput,
    DirectTerminal,
    Expression(ContextExpression),
}

#[derive(Debug, Eq, PartialEq)]
pub(crate) struct NativeKeybindingCatalog;

impl KeybindingCatalog for NativeKeybindingCatalog {
    type Command = ZetermCommandId;
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
        ZetermCommandId::bindable_from_id(id)
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

fn default_keybinding(command: ZetermCommandId) -> Option<&'static KeySequence> {
    static TOGGLE_TERMINAL: OnceLock<KeySequence> = OnceLock::new();
    static COPY: OnceLock<KeySequence> = OnceLock::new();
    static PASTE: OnceLock<KeySequence> = OnceLock::new();
    static SAVE: OnceLock<KeySequence> = OnceLock::new();
    match command {
        ZetermCommandId::ToggleTerminalSurface => Some(TOGGLE_TERMINAL.get_or_init(|| {
            KeySequence::single(
                Chord::logical("j", ShortcutModifiers::primary()).expect("builtin key"),
            )
        })),
        ZetermCommandId::OpenKeyboardShortcuts => Some(static_keyboard_shortcuts_binding()),
        ZetermCommandId::Copy => Some(COPY.get_or_init(|| {
            KeySequence::single(
                Chord::logical("c", ShortcutModifiers::primary()).expect("builtin key"),
            )
        })),
        ZetermCommandId::Paste => Some(PASTE.get_or_init(|| {
            KeySequence::single(
                Chord::logical("v", ShortcutModifiers::primary()).expect("builtin key"),
            )
        })),
        ZetermCommandId::Save => Some(SAVE.get_or_init(|| {
            KeySequence::single(
                Chord::logical("s", ShortcutModifiers::primary()).expect("builtin key"),
            )
        })),
        ZetermCommandId::ToggleComposerMode
        | ZetermCommandId::OpenLanguageServerSettings
        | ZetermCommandId::ToggleSessionSidebar
        | ZetermCommandId::ToggleAgentSidebar
        | ZetermCommandId::ActivateSessionTab
        | ZetermCommandId::AddSession
        | ZetermCommandId::ShowAgentChanges
        | ZetermCommandId::ShowAgentFiles
        | ZetermCommandId::RefreshAgentFiles
        | ZetermCommandId::ToggleAgentFileSearch
        | ZetermCommandId::PinSession
        | ZetermCommandId::CloseSession
        | ZetermCommandId::RenameSession
        | ZetermCommandId::ForkSession
        | ZetermCommandId::PickExecutionLocation
        | ZetermCommandId::PickWorkingDirectory
        | ZetermCommandId::PickGitBranch
        | ZetermCommandId::ShowWorkspaceDiff => None,
    }
}

fn static_keyboard_shortcuts_binding() -> &'static KeySequence {
    static KEYBOARD_SHORTCUTS: OnceLock<KeySequence> = OnceLock::new();
    KEYBOARD_SHORTCUTS.get_or_init(|| {
        KeySequence::single(Chord::logical(",", ShortcutModifiers::primary()).expect("builtin key"))
    })
}

fn builtin_bindings(platform: HostPlatform) -> BindingSet<NativeBindingCondition, ZetermCommandId> {
    let mut bindings = BindingSet::default();
    register(
        &mut bindings,
        "j",
        ShortcutModifiers::primary(),
        ZetermCommandId::ToggleTerminalSurface,
        NativeBindingCondition::Always,
    );
    register(
        &mut bindings,
        ",",
        ShortcutModifiers::primary(),
        ZetermCommandId::OpenKeyboardShortcuts,
        NativeBindingCondition::Always,
    );
    register_text_input_clipboard(&mut bindings);
    register(
        &mut bindings,
        "s",
        ShortcutModifiers::primary(),
        ZetermCommandId::Save,
        NativeBindingCondition::Always,
    );
    register_direct_terminal_clipboard(&mut bindings, platform);
    bindings
}

fn register_text_input_clipboard(
    bindings: &mut BindingSet<NativeBindingCondition, ZetermCommandId>,
) {
    register(
        bindings,
        "c",
        ShortcutModifiers::primary(),
        ZetermCommandId::Copy,
        NativeBindingCondition::TextInput,
    );
    register(
        bindings,
        "v",
        ShortcutModifiers::primary(),
        ZetermCommandId::Paste,
        NativeBindingCondition::TextInput,
    );
    register(
        bindings,
        "c",
        ShortcutModifiers::primary().with_shift(),
        ZetermCommandId::Copy,
        NativeBindingCondition::TextInput,
    );
    register(
        bindings,
        "v",
        ShortcutModifiers::primary().with_shift(),
        ZetermCommandId::Paste,
        NativeBindingCondition::TextInput,
    );
}

fn register_direct_terminal_clipboard(
    bindings: &mut BindingSet<NativeBindingCondition, ZetermCommandId>,
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
        ZetermCommandId::Copy,
        NativeBindingCondition::DirectTerminal,
    );
    register(
        bindings,
        "v",
        modifiers,
        ZetermCommandId::Paste,
        NativeBindingCondition::DirectTerminal,
    );
}

fn register(
    bindings: &mut BindingSet<NativeBindingCondition, ZetermCommandId>,
    key: &str,
    modifiers: ShortcutModifiers,
    command: ZetermCommandId,
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
