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

pub(crate) type WorkbenchKeybindings = Keybindings<WorkbenchKeybindingCatalog>;
pub(crate) type WorkbenchKeybindingResolution = KeybindingResolution<AppCommandId>;
#[cfg(test)]
pub(crate) type WorkbenchUserBinding = UserBinding<WorkbenchKeybindingCatalog>;
#[cfg(test)]
pub(crate) type WorkbenchUserBindingTarget = UserBindingTarget<AppCommandId>;
pub(crate) type KeybindingsResource =
    zeta_keybindings_host::KeybindingsResource<WorkbenchKeybindingCatalog>;

pub(crate) use zeta_keybindings_host::KeybindingsResourcePoll;

/// Application context facts projected into the generic keybinding catalog.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct WorkbenchKeybindingContext {
    facts: WorkbenchKeybindingFacts,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct WorkbenchKeybindingFacts {
    pub(crate) direct_terminal: bool,
    pub(crate) terminal_surface_visible: bool,
    pub(crate) tab_container_visible: bool,
    pub(crate) inspector_visible: bool,
    pub(crate) file_search_visible: bool,
    pub(crate) composer_route: &'static str,
}

impl WorkbenchKeybindingContext {
    #[cfg(test)]
    pub(crate) const fn text_input() -> Self {
        Self {
            facts: WorkbenchKeybindingFacts {
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
            facts: WorkbenchKeybindingFacts {
                direct_terminal: true,
                terminal_surface_visible: true,
                tab_container_visible: false,
                inspector_visible: false,
                file_search_visible: false,
                composer_route: "agent",
            },
        }
    }

    pub(crate) const fn from_facts(facts: WorkbenchKeybindingFacts) -> Self {
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
pub(crate) enum WorkbenchBindingCondition {
    Always,
    TextInput,
    DirectTerminal,
    Expression(ContextExpression),
}

#[derive(Debug, Eq, PartialEq)]
pub(crate) struct WorkbenchKeybindingCatalog;

impl KeybindingCatalog for WorkbenchKeybindingCatalog {
    type Command = AppCommandId;
    type Condition = WorkbenchBindingCondition;
    type Context = WorkbenchKeybindingContext;

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
            return Ok(WorkbenchBindingCondition::Always);
        };
        let expression = ContextExpression::parse(source).map_err(|error| error.to_string())?;
        if let Some(key) = expression
            .referenced_keys()
            .into_iter()
            .find(|key| !WorkbenchKeybindingContext::supports_key(key))
        {
            return Err(format!("unknown context key `{key}`"));
        }
        Ok(WorkbenchBindingCondition::Expression(expression))
    }

    fn condition_matches(condition: &Self::Condition, context: &Self::Context) -> bool {
        condition_matches(condition, context)
    }
}

fn condition_matches(
    condition: &WorkbenchBindingCondition,
    context: &WorkbenchKeybindingContext,
) -> bool {
    match condition {
        WorkbenchBindingCondition::Always => true,
        WorkbenchBindingCondition::TextInput => !context.facts.direct_terminal,
        WorkbenchBindingCondition::DirectTerminal => context.facts.direct_terminal,
        WorkbenchBindingCondition::Expression(expression) => {
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
        AppCommandId::ManageRemoteTunnels
        | AppCommandId::ToggleTabContainer
        | AppCommandId::ToggleFilesPane
        | AppCommandId::AddSession
        | AppCommandId::ShowAgentChanges
        | AppCommandId::ShowAgentFiles
        | AppCommandId::RefreshAgentFiles
        | AppCommandId::ToggleAgentFileSearch
        | AppCommandId::PinSession
        | AppCommandId::CloseSession
        | AppCommandId::RenameSession
        | AppCommandId::GroupSession
        | AppCommandId::ForkSession
        | AppCommandId::PickExecutionLocation
        | AppCommandId::PickWorkingDirectory
        | AppCommandId::PickGitBranch
        | AppCommandId::ShowGitDiff
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

fn builtin_bindings(platform: HostPlatform) -> BindingSet<WorkbenchBindingCondition, AppCommandId> {
    let mut bindings = BindingSet::default();
    register(
        &mut bindings,
        "j",
        ShortcutModifiers::primary(),
        AppCommandId::ToggleTerminalSurface,
        WorkbenchBindingCondition::Always,
    );
    register(
        &mut bindings,
        ",",
        ShortcutModifiers::primary(),
        AppCommandId::OpenKeyboardShortcuts,
        WorkbenchBindingCondition::Always,
    );
    register_text_input_clipboard(&mut bindings);
    register(
        &mut bindings,
        "s",
        ShortcutModifiers::primary(),
        AppCommandId::Save,
        WorkbenchBindingCondition::Always,
    );
    register(
        &mut bindings,
        "\\",
        ShortcutModifiers::primary(),
        AppCommandId::SplitTerminalHorizontal,
        WorkbenchBindingCondition::Always,
    );
    register(
        &mut bindings,
        "\\",
        ShortcutModifiers::primary().with_shift(),
        AppCommandId::SplitTerminalVertical,
        WorkbenchBindingCondition::Always,
    );
    register_direct_terminal_clipboard(&mut bindings, platform);
    bindings
}

fn register_text_input_clipboard(
    bindings: &mut BindingSet<WorkbenchBindingCondition, AppCommandId>,
) {
    register(
        bindings,
        "c",
        ShortcutModifiers::primary(),
        AppCommandId::Copy,
        WorkbenchBindingCondition::TextInput,
    );
    register(
        bindings,
        "v",
        ShortcutModifiers::primary(),
        AppCommandId::Paste,
        WorkbenchBindingCondition::TextInput,
    );
    register(
        bindings,
        "c",
        ShortcutModifiers::primary().with_shift(),
        AppCommandId::Copy,
        WorkbenchBindingCondition::TextInput,
    );
    register(
        bindings,
        "v",
        ShortcutModifiers::primary().with_shift(),
        AppCommandId::Paste,
        WorkbenchBindingCondition::TextInput,
    );
}

fn register_direct_terminal_clipboard(
    bindings: &mut BindingSet<WorkbenchBindingCondition, AppCommandId>,
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
        WorkbenchBindingCondition::DirectTerminal,
    );
    register(
        bindings,
        "v",
        modifiers,
        AppCommandId::Paste,
        WorkbenchBindingCondition::DirectTerminal,
    );
}

fn register(
    bindings: &mut BindingSet<WorkbenchBindingCondition, AppCommandId>,
    key: &str,
    modifiers: ShortcutModifiers,
    command: AppCommandId,
    condition: WorkbenchBindingCondition,
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
) -> Result<Vec<WorkbenchUserBinding>, zeta_keybindings_host::KeybindingsResourceError> {
    zeta_keybindings_host::compile_user_bindings::<WorkbenchKeybindingCatalog>(contents, platform)
}

#[cfg(test)]
pub(crate) fn binding_diagnostics(
    rules: &[WorkbenchUserBinding],
    platform: HostPlatform,
) -> Vec<String> {
    zeta_keybindings_host::binding_diagnostics::<WorkbenchKeybindingCatalog>(rules, platform)
}

#[cfg(test)]
#[path = "keybindings_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "keybindings_resource_tests.rs"]
mod resource_tests;
