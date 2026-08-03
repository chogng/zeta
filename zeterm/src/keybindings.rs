use std::time::{Duration, Instant};

use zeta_keybinding::{
    BindingPriority, BindingSet, BindingSource, Chord, ContextExpression, ContextValue,
    HostPlatform, KeySequence, KeyStroke, KeybindingResolver, LogicalKey, Modifiers,
    PhysicalKey as ShortcutPhysicalKey, ResolveResult, ShortcutModifiers,
};
use zeta_winit::{Key, KeyEvent, ModifiersState, PhysicalKey};

use crate::commands::NativeCommand;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) enum NativeBindingCondition {
    Always,
    TextInput,
    DirectTerminal,
    Expression(ContextExpression),
}

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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum NativeKeybindingResolution {
    NoMatch,
    Command(NativeCommand),
    Consumed,
}

pub(crate) struct NativeKeybindings {
    bindings: BindingSet<NativeBindingCondition, NativeCommand>,
    platform: HostPlatform,
    user_bindings: Vec<NativeUserBinding>,
    pending: Vec<KeyStroke>,
    pending_keybinding: Option<KeySequence>,
    chord_deadline: Option<Instant>,
}

const CHORD_TIMEOUT: Duration = Duration::from_millis(1_500);

impl Default for NativeKeybindings {
    fn default() -> Self {
        Self::for_platform(HostPlatform::current())
    }
}

impl NativeKeybindings {
    pub(super) fn for_platform(platform: HostPlatform) -> Self {
        Self {
            bindings: builtin_bindings(platform),
            platform,
            user_bindings: Vec::new(),
            pending: Vec::new(),
            pending_keybinding: None,
            chord_deadline: None,
        }
    }

    pub(crate) fn resolve(
        &mut self,
        event: &KeyEvent,
        modifiers: ModifiersState,
        context: NativeKeybindingContext,
    ) -> NativeKeybindingResolution {
        let Some(stroke) = key_stroke(event, modifiers) else {
            return NativeKeybindingResolution::NoMatch;
        };
        self.resolve_stroke_at(&stroke, context, Instant::now())
    }

    #[cfg(test)]
    fn resolve_stroke(
        &mut self,
        stroke: &KeyStroke,
        context: NativeKeybindingContext,
    ) -> NativeKeybindingResolution {
        self.resolve_stroke_at(stroke, context, Instant::now())
    }

    pub(super) fn resolve_stroke_at(
        &mut self,
        stroke: &KeyStroke,
        context: NativeKeybindingContext,
        now: Instant,
    ) -> NativeKeybindingResolution {
        self.advance_chord(now);
        let was_pending = !self.pending.is_empty();
        self.pending.push(stroke.clone());
        let resolver = KeybindingResolver::new(&self.bindings, self.platform);
        match resolver.resolve(&context, &self.pending, condition_matches) {
            ResolveResult::NoMatch => {
                self.cancel_chord();
                if was_pending {
                    NativeKeybindingResolution::Consumed
                } else {
                    NativeKeybindingResolution::NoMatch
                }
            }
            ResolveResult::Command { command, .. } => {
                self.cancel_chord();
                NativeKeybindingResolution::Command(command)
            }
            ResolveResult::PendingChord { keybinding } => {
                self.pending_keybinding = Some(keybinding);
                self.chord_deadline = Some(now + CHORD_TIMEOUT);
                NativeKeybindingResolution::Consumed
            }
            ResolveResult::Blocked { .. } => {
                self.cancel_chord();
                NativeKeybindingResolution::Consumed
            }
        }
    }

    pub(crate) fn advance_chord(&mut self, now: Instant) -> bool {
        let expired = self.chord_deadline.is_some_and(|deadline| now >= deadline);
        if expired {
            self.cancel_chord();
        }
        expired
    }

    pub(crate) fn cancel_chord(&mut self) {
        self.pending.clear();
        self.pending_keybinding = None;
        self.chord_deadline = None;
    }

    pub(crate) const fn chord_deadline(&self) -> Option<Instant> {
        self.chord_deadline
    }

    pub(crate) fn pending_keybinding(&self) -> Option<(&KeySequence, usize)> {
        self.pending_keybinding
            .as_ref()
            .map(|keybinding| (keybinding, self.pending.len()))
    }

    pub(crate) const fn platform(&self) -> HostPlatform {
        self.platform
    }

    pub(crate) fn binding_for_command(&self, command: NativeCommand) -> Option<&KeySequence> {
        self.user_bindings
            .iter()
            .rev()
            .find_map(|binding| match binding.target {
                NativeUserBindingTarget::Command(candidate) if candidate == command => {
                    Some(&binding.keybinding)
                }
                NativeUserBindingTarget::Command(_) | NativeUserBindingTarget::Block => None,
            })
            .or_else(|| default_keybinding(command))
    }

    pub(super) fn replace_user_bindings(&mut self, rules: Vec<NativeUserBinding>) {
        let mut bindings = builtin_bindings(self.platform);
        for rule in &rules {
            match rule.target {
                NativeUserBindingTarget::Command(command) => bindings.register_command(
                    rule.keybinding.clone(),
                    command,
                    rule.when.clone(),
                    BindingSource::User,
                    BindingPriority::NORMAL,
                ),
                NativeUserBindingTarget::Block => bindings.register_blocker(
                    rule.keybinding.clone(),
                    rule.when.clone(),
                    BindingSource::User,
                    BindingPriority::NORMAL,
                ),
            }
        }
        self.bindings = bindings;
        self.user_bindings = rules;
        self.cancel_chord();
    }
}

#[derive(Clone)]
pub(super) struct NativeUserBinding {
    pub(super) keybinding: KeySequence,
    pub(super) target: NativeUserBindingTarget,
    pub(super) when: NativeBindingCondition,
    pub(super) when_source: Option<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum NativeUserBindingTarget {
    Command(NativeCommand),
    Block,
}

fn default_keybinding(command: NativeCommand) -> Option<&'static KeySequence> {
    static TOGGLE_TERMINAL: std::sync::OnceLock<KeySequence> = std::sync::OnceLock::new();
    static COPY: std::sync::OnceLock<KeySequence> = std::sync::OnceLock::new();
    static PASTE: std::sync::OnceLock<KeySequence> = std::sync::OnceLock::new();
    static SAVE: std::sync::OnceLock<KeySequence> = std::sync::OnceLock::new();
    match command {
        NativeCommand::ToggleTerminalSurface => Some(TOGGLE_TERMINAL.get_or_init(|| {
            KeySequence::single(
                Chord::logical("j", ShortcutModifiers::primary()).expect("builtin key"),
            )
        })),
        NativeCommand::OpenKeyboardShortcuts => Some(static_keyboard_shortcuts_binding()),
        NativeCommand::Copy => Some(COPY.get_or_init(|| {
            KeySequence::single(
                Chord::logical("c", ShortcutModifiers::primary()).expect("builtin key"),
            )
        })),
        NativeCommand::Paste => Some(PASTE.get_or_init(|| {
            KeySequence::single(
                Chord::logical("v", ShortcutModifiers::primary()).expect("builtin key"),
            )
        })),
        NativeCommand::Save => Some(SAVE.get_or_init(|| {
            KeySequence::single(
                Chord::logical("s", ShortcutModifiers::primary()).expect("builtin key"),
            )
        })),
        NativeCommand::ToggleComposerMode
        | NativeCommand::OpenLanguageServerSettings
        | NativeCommand::ToggleSessionSidebar
        | NativeCommand::ToggleAgentSidebar
        | NativeCommand::ActivateSessionTab
        | NativeCommand::AddSession
        | NativeCommand::SelectAgentPane(_)
        | NativeCommand::RefreshFiles
        | NativeCommand::ToggleFileSearch
        | NativeCommand::SessionContextMenu(_)
        | NativeCommand::Context(_) => None,
    }
}

fn static_keyboard_shortcuts_binding() -> &'static KeySequence {
    static KEYBOARD_SHORTCUTS: std::sync::OnceLock<KeySequence> = std::sync::OnceLock::new();
    KEYBOARD_SHORTCUTS.get_or_init(|| {
        KeySequence::single(Chord::logical(",", ShortcutModifiers::primary()).expect("builtin key"))
    })
}

fn builtin_bindings(platform: HostPlatform) -> BindingSet<NativeBindingCondition, NativeCommand> {
    let mut bindings = BindingSet::default();
    register(
        &mut bindings,
        "j",
        ShortcutModifiers::primary(),
        NativeCommand::ToggleTerminalSurface,
        NativeBindingCondition::Always,
    );
    register(
        &mut bindings,
        ",",
        ShortcutModifiers::primary(),
        NativeCommand::OpenKeyboardShortcuts,
        NativeBindingCondition::Always,
    );
    register_text_input_clipboard(&mut bindings);
    register(
        &mut bindings,
        "s",
        ShortcutModifiers::primary(),
        NativeCommand::Save,
        NativeBindingCondition::Always,
    );
    register_direct_terminal_clipboard(&mut bindings, platform);
    bindings
}

fn register_text_input_clipboard(bindings: &mut BindingSet<NativeBindingCondition, NativeCommand>) {
    register(
        bindings,
        "c",
        ShortcutModifiers::primary(),
        NativeCommand::Copy,
        NativeBindingCondition::TextInput,
    );
    register(
        bindings,
        "v",
        ShortcutModifiers::primary(),
        NativeCommand::Paste,
        NativeBindingCondition::TextInput,
    );
    register(
        bindings,
        "c",
        ShortcutModifiers::primary().with_shift(),
        NativeCommand::Copy,
        NativeBindingCondition::TextInput,
    );
    register(
        bindings,
        "v",
        ShortcutModifiers::primary().with_shift(),
        NativeCommand::Paste,
        NativeBindingCondition::TextInput,
    );
}

fn register_direct_terminal_clipboard(
    bindings: &mut BindingSet<NativeBindingCondition, NativeCommand>,
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
        NativeCommand::Copy,
        NativeBindingCondition::DirectTerminal,
    );
    register(
        bindings,
        "v",
        modifiers,
        NativeCommand::Paste,
        NativeBindingCondition::DirectTerminal,
    );
}

fn register(
    bindings: &mut BindingSet<NativeBindingCondition, NativeCommand>,
    key: &str,
    modifiers: ShortcutModifiers,
    command: NativeCommand,
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

fn key_stroke(event: &KeyEvent, modifiers: ModifiersState) -> Option<KeyStroke> {
    let logical_key = match &event.logical_key {
        Key::Character(text) => LogicalKey::new(text.as_str()),
        Key::Named(key) => LogicalKey::new(format!("{key:?}")),
        Key::Dead(character) => {
            character.and_then(|character| LogicalKey::new(character.to_string()))
        }
        Key::Unidentified(_) => None,
    }?;
    let physical_key = match event.physical_key {
        PhysicalKey::Code(code) => ShortcutPhysicalKey::new(format!("{code:?}")),
        PhysicalKey::Unidentified(_) => None,
    };
    Some(KeyStroke::new(
        logical_key,
        physical_key,
        shortcut_modifiers(modifiers),
    ))
}

fn shortcut_modifiers(modifiers: ModifiersState) -> Modifiers {
    let mut shortcut = Modifiers::none();
    if modifiers.control_key() {
        shortcut = shortcut.with_control();
    }
    if modifiers.shift_key() {
        shortcut = shortcut.with_shift();
    }
    if modifiers.alt_key() {
        shortcut = shortcut.with_alt();
    }
    if modifiers.super_key() {
        shortcut = shortcut.with_meta();
    }
    shortcut
}

#[cfg(test)]
#[path = "keybindings_tests.rs"]
mod tests;
