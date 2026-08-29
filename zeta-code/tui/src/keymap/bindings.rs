use super::AppKeymap;
use super::AppUserBinding;
use super::chords::validate_specs;
use super::chords::validate_user_bindings;
use super::input::normalized_key;
use crossterm::event::KeyEvent;
use zeta_keybinding::BindingPriority;
use zeta_keybinding::BindingSet;
use zeta_keybinding::BindingSource;
use zeta_keybinding::ContextExpression;
use zeta_keybinding::ContextValue;
use zeta_keybinding::HostPlatform;
use zeta_keybinding::KeySequence;
use zeta_keybinding::KeybindingResolver;
use zeta_keybinding::ResolveResult;
use zeta_keybinding::UserBindingTarget;
use zeta_keybinding::compile_user_bindings;
use zeta_keybinding::parse_key_sequence;
use zeta_keybinding::serialize_key_sequence;

/// Cross-component actions owned by the Zeta Code TUI root.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum AppKeymapAction {
    CycleApprovalMode,
    RootEscape,
    OpenRewind,
    ReadClipboardImage,
    InterruptOrQuit,
    CopyLastResponse,
    Suspend,
}

impl AppKeymapAction {
    pub(crate) const fn command_id(self) -> Option<&'static str> {
        match self {
            Self::CycleApprovalMode => Some("zetaCode.action.cycleApprovalMode"),
            Self::RootEscape => None,
            Self::OpenRewind => Some("zetaCode.action.openRewind"),
            Self::ReadClipboardImage => Some("zetaCode.action.attachClipboardImage"),
            Self::InterruptOrQuit => Some("zetaCode.action.interruptOrQuit"),
            Self::CopyLastResponse => Some("zetaCode.action.copyLastResponse"),
            Self::Suspend => Some("zetaCode.action.suspend"),
        }
    }

    fn from_command_id(id: &str) -> Option<Self> {
        Self::USER_BINDABLE
            .into_iter()
            .find(|action| action.command_id() == Some(id))
    }

    const USER_BINDABLE: [Self; 6] = [
        Self::CycleApprovalMode,
        Self::OpenRewind,
        Self::ReadClipboardImage,
        Self::InterruptOrQuit,
        Self::CopyLastResponse,
        Self::Suspend,
    ];

    const fn label(self) -> &'static str {
        match self {
            Self::CycleApprovalMode => "Cycle approval mode",
            Self::RootEscape => "Root escape",
            Self::OpenRewind => "Open rewind checkpoints",
            Self::ReadClipboardImage => "Attach clipboard image",
            Self::InterruptOrQuit => "Interrupt or quit",
            Self::CopyLastResponse => "Copy last response",
            Self::Suspend => "Suspend Zeta",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct KeymapUserBindingSnapshot {
    pub(crate) key: String,
    pub(crate) when: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct KeymapActionSnapshot {
    pub(crate) command_id: &'static str,
    pub(crate) label: &'static str,
    pub(crate) default_bindings: Vec<String>,
    pub(crate) user_bindings: Vec<KeymapUserBindingSnapshot>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum AppKeymapCondition {
    Always,
    AcceptsInput,
    EmptyChatInput,
    PressWithInput,
    PressWithInputWithoutSelection,
    Expression(ContextExpression),
}

/// State needed to decide a root binding without exposing component internals to the resolver.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct AppKeymapContext {
    pub(crate) accepts_input: bool,
    pub(crate) has_selection: bool,
    pub(crate) chat_input_empty: bool,
    pub(crate) is_press: bool,
}

impl AppKeymapContext {
    fn supports_key(key: &str) -> bool {
        matches!(
            key,
            "inputFocus" | "chatInputEmpty" | "selectionVisible" | "keyEventPress"
        )
    }

    fn value(self, key: &str) -> Option<ContextValue> {
        match key {
            "inputFocus" => Some(ContextValue::Boolean(self.accepts_input)),
            "chatInputEmpty" => Some(ContextValue::Boolean(self.chat_input_empty)),
            "selectionVisible" => Some(ContextValue::Boolean(self.has_selection)),
            "keyEventPress" => Some(ContextValue::Boolean(self.is_press)),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct AppKeybindingSpec {
    pub(super) keybinding: &'static str,
    pub(super) action: AppKeymapAction,
    pub(super) condition: AppKeymapCondition,
}

const APP_KEYBINDINGS: &[AppKeybindingSpec] = &[
    AppKeybindingSpec {
        keybinding: "shift+tab",
        action: AppKeymapAction::CycleApprovalMode,
        condition: AppKeymapCondition::PressWithInputWithoutSelection,
    },
    AppKeybindingSpec {
        keybinding: "escape",
        action: AppKeymapAction::RootEscape,
        condition: AppKeymapCondition::PressWithInput,
    },
    AppKeybindingSpec {
        keybinding: "ctrl+v",
        action: AppKeymapAction::ReadClipboardImage,
        condition: AppKeymapCondition::AcceptsInput,
    },
    AppKeybindingSpec {
        keybinding: "ctrl+c",
        action: AppKeymapAction::InterruptOrQuit,
        condition: AppKeymapCondition::Always,
    },
    AppKeybindingSpec {
        keybinding: "ctrl+d",
        action: AppKeymapAction::InterruptOrQuit,
        condition: AppKeymapCondition::EmptyChatInput,
    },
    AppKeybindingSpec {
        keybinding: "ctrl+o",
        action: AppKeymapAction::CopyLastResponse,
        condition: AppKeymapCondition::Always,
    },
    AppKeybindingSpec {
        keybinding: "ctrl+z",
        action: AppKeymapAction::Suspend,
        condition: AppKeymapCondition::Always,
    },
];

impl Default for AppKeymap {
    fn default() -> Self {
        Self::from_specs(APP_KEYBINDINGS)
    }
}

impl AppKeymap {
    pub(super) fn from_specs(specs: &[AppKeybindingSpec]) -> Self {
        let parsed = specs
            .iter()
            .map(|binding| {
                (
                    binding.clone(),
                    parse_key_sequence(binding.keybinding)
                        .expect("fixed TUI binding must use portable keybinding syntax"),
                )
            })
            .collect::<Vec<_>>();
        validate_specs(&parsed);
        let mut single_bindings = BindingSet::default();
        let mut chord_bindings = BindingSet::default();
        for (binding, keybinding) in parsed {
            register_command(
                &mut single_bindings,
                &mut chord_bindings,
                keybinding,
                binding.action,
                binding.condition,
                BindingSource::Builtin,
            );
        }
        Self {
            single_bindings,
            chord_bindings,
            platform: HostPlatform::current(),
            pending: None,
            user_bindings: Vec::new(),
        }
    }

    pub(crate) fn replace_user_bindings(
        &mut self,
        rules: Vec<AppUserBinding>,
    ) -> Result<(), String> {
        let parsed_builtins = APP_KEYBINDINGS
            .iter()
            .map(|binding| {
                parse_key_sequence(binding.keybinding)
                    .map(|keybinding| (binding.clone(), keybinding))
                    .map_err(|error| error.to_string())
            })
            .collect::<Result<Vec<_>, _>>()?;
        validate_user_bindings(&rules, &parsed_builtins)?;

        let mut single_bindings = BindingSet::default();
        let mut chord_bindings = BindingSet::default();
        for (binding, keybinding) in parsed_builtins {
            register_command(
                &mut single_bindings,
                &mut chord_bindings,
                keybinding,
                binding.action,
                binding.condition,
                BindingSource::Builtin,
            );
        }
        for rule in &rules {
            let bindings =
                bindings_for_sequence(&mut single_bindings, &mut chord_bindings, &rule.keybinding);
            match rule.target {
                UserBindingTarget::Command(action) => bindings.register_command(
                    rule.keybinding.clone(),
                    action,
                    rule.when.clone(),
                    BindingSource::User,
                    BindingPriority::NORMAL,
                ),
                UserBindingTarget::Block => bindings.register_blocker(
                    rule.keybinding.clone(),
                    rule.when.clone(),
                    BindingSource::User,
                    BindingPriority::NORMAL,
                ),
            }
        }
        self.single_bindings = single_bindings;
        self.chord_bindings = chord_bindings;
        self.user_bindings = rules;
        self.cancel_chord();
        Ok(())
    }

    pub(crate) fn setup_actions(&self) -> Vec<KeymapActionSnapshot> {
        AppKeymapAction::USER_BINDABLE
            .into_iter()
            .map(|action| KeymapActionSnapshot {
                command_id: action
                    .command_id()
                    .expect("a user-bindable TUI action has a command ID"),
                label: action.label(),
                default_bindings: APP_KEYBINDINGS
                    .iter()
                    .filter(|binding| binding.action == action)
                    .map(|binding| {
                        parse_key_sequence(binding.keybinding)
                            .map(|key| serialize_key_sequence(&key))
                            .expect("fixed TUI binding must use portable keybinding syntax")
                    })
                    .collect(),
                user_bindings: self
                    .user_bindings
                    .iter()
                    .filter_map(|binding| match binding.target {
                        UserBindingTarget::Command(candidate) if candidate == action => {
                            Some(KeymapUserBindingSnapshot {
                                key: serialize_key_sequence(&binding.keybinding),
                                when: binding.when_source.clone(),
                            })
                        }
                        UserBindingTarget::Command(_) | UserBindingTarget::Block => None,
                    })
                    .collect(),
            })
            .collect()
    }

    pub(crate) fn resolve_single(
        &self,
        key: &KeyEvent,
        context: AppKeymapContext,
    ) -> Option<AppKeymapAction> {
        let event = normalized_key(key)?.stroke;
        let resolver = KeybindingResolver::new(&self.single_bindings, self.platform);
        match resolver.resolve(&context, &[event], condition_matches) {
            ResolveResult::Command { command, .. } => Some(command),
            ResolveResult::NoMatch
            | ResolveResult::PendingChord { .. }
            | ResolveResult::Blocked { .. } => None,
        }
    }
}

pub(crate) fn compile_app_user_bindings(
    contents: &[u8],
    platform: HostPlatform,
) -> Result<Vec<AppUserBinding>, String> {
    compile_user_bindings(
        contents,
        platform,
        AppKeymapAction::from_command_id,
        parse_user_condition,
    )
    .map_err(|error| error.to_string())
}

fn parse_user_condition(source: Option<&str>) -> Result<AppKeymapCondition, String> {
    let Some(source) = source else {
        return Ok(AppKeymapCondition::Always);
    };
    let expression = ContextExpression::parse(source).map_err(|error| error.to_string())?;
    if let Some(key) = expression
        .referenced_keys()
        .into_iter()
        .find(|key| !AppKeymapContext::supports_key(key))
    {
        return Err(format!("unknown context key `{key}`"));
    }
    Ok(AppKeymapCondition::Expression(expression))
}

fn register_command(
    single_bindings: &mut BindingSet<AppKeymapCondition, AppKeymapAction>,
    chord_bindings: &mut BindingSet<AppKeymapCondition, AppKeymapAction>,
    keybinding: KeySequence,
    action: AppKeymapAction,
    condition: AppKeymapCondition,
    source: BindingSource,
) {
    let bindings = bindings_for_sequence(single_bindings, chord_bindings, &keybinding);
    bindings.register_command(
        keybinding,
        action,
        condition,
        source,
        BindingPriority::NORMAL,
    );
}

fn bindings_for_sequence<'a>(
    single_bindings: &'a mut BindingSet<AppKeymapCondition, AppKeymapAction>,
    chord_bindings: &'a mut BindingSet<AppKeymapCondition, AppKeymapAction>,
    keybinding: &KeySequence,
) -> &'a mut BindingSet<AppKeymapCondition, AppKeymapAction> {
    if keybinding.chords().len() == 1 {
        single_bindings
    } else {
        chord_bindings
    }
}

pub(super) fn condition_matches(
    condition: &AppKeymapCondition,
    context: &AppKeymapContext,
) -> bool {
    match condition {
        AppKeymapCondition::Always => true,
        AppKeymapCondition::AcceptsInput => context.accepts_input,
        AppKeymapCondition::EmptyChatInput => context.chat_input_empty,
        AppKeymapCondition::PressWithInput => context.is_press && context.accepts_input,
        AppKeymapCondition::PressWithInputWithoutSelection => {
            context.is_press && context.accepts_input && !context.has_selection
        }
        AppKeymapCondition::Expression(expression) => expression.evaluate(|key| context.value(key)),
    }
}
