use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyModifiers;
use zeta_keybinding::BindingPriority;
use zeta_keybinding::BindingSet;
use zeta_keybinding::BindingSource;
use zeta_keybinding::Chord;
use zeta_keybinding::HostPlatform;
use zeta_keybinding::KeySequence;
use zeta_keybinding::KeyStroke;
use zeta_keybinding::KeybindingResolver;
use zeta_keybinding::LogicalKey;
use zeta_keybinding::Modifiers;
use zeta_keybinding::ResolveResult;
use zeta_keybinding::ShortcutModifiers;

/// Cross-component actions owned by the Zeta Code TUI root.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum GlobalKeymapAction {
    CycleApprovalMode,
    RootEscape,
    ReadClipboardImage,
    InterruptOrQuit,
    CopyLastResponse,
    Suspend,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum GlobalKeymapCondition {
    Always,
    AcceptsInput,
    EmptyComposer,
    PressWithInput,
    PressWithInputWithoutSelection,
}

/// State needed to decide a root binding without exposing component internals to the resolver.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct GlobalKeymapContext {
    pub(super) accepts_input: bool,
    pub(super) has_selection: bool,
    pub(super) composer_empty: bool,
    pub(super) is_press: bool,
}

/// Fixed root keymap for terminal-wide actions.
#[derive(Debug)]
pub(super) struct GlobalKeymap {
    bindings: BindingSet<GlobalKeymapCondition, GlobalKeymapAction>,
    platform: HostPlatform,
}

impl Default for GlobalKeymap {
    fn default() -> Self {
        let mut bindings = BindingSet::default();
        register(
            &mut bindings,
            "tab",
            ShortcutModifiers::none().with_shift(),
            GlobalKeymapAction::CycleApprovalMode,
            GlobalKeymapCondition::PressWithInputWithoutSelection,
        );
        register(
            &mut bindings,
            "escape",
            ShortcutModifiers::none(),
            GlobalKeymapAction::RootEscape,
            GlobalKeymapCondition::PressWithInput,
        );
        register(
            &mut bindings,
            "v",
            ShortcutModifiers::control(),
            GlobalKeymapAction::ReadClipboardImage,
            GlobalKeymapCondition::AcceptsInput,
        );
        register(
            &mut bindings,
            "c",
            ShortcutModifiers::control(),
            GlobalKeymapAction::InterruptOrQuit,
            GlobalKeymapCondition::Always,
        );
        register(
            &mut bindings,
            "d",
            ShortcutModifiers::control(),
            GlobalKeymapAction::InterruptOrQuit,
            GlobalKeymapCondition::EmptyComposer,
        );
        register(
            &mut bindings,
            "o",
            ShortcutModifiers::control(),
            GlobalKeymapAction::CopyLastResponse,
            GlobalKeymapCondition::Always,
        );
        register(
            &mut bindings,
            "z",
            ShortcutModifiers::control(),
            GlobalKeymapAction::Suspend,
            GlobalKeymapCondition::Always,
        );
        Self {
            bindings,
            platform: HostPlatform::current(),
        }
    }
}

impl GlobalKeymap {
    pub(super) fn resolve(
        &self,
        key: &KeyEvent,
        context: GlobalKeymapContext,
    ) -> Option<GlobalKeymapAction> {
        let event = key_stroke(key)?;
        let resolver = KeybindingResolver::new(&self.bindings, self.platform);
        match resolver.resolve(&context, &[event], condition_matches) {
            ResolveResult::Command { command, .. } => Some(command),
            ResolveResult::NoMatch
            | ResolveResult::PendingChord { .. }
            | ResolveResult::Blocked { .. } => None,
        }
    }
}

fn register(
    bindings: &mut BindingSet<GlobalKeymapCondition, GlobalKeymapAction>,
    key: &str,
    modifiers: ShortcutModifiers,
    action: GlobalKeymapAction,
    condition: GlobalKeymapCondition,
) {
    let chord = Chord::logical(key, modifiers).expect("fixed TUI binding must have a logical key");
    bindings.register_command(
        KeySequence::single(chord),
        action,
        condition,
        BindingSource::Builtin,
        BindingPriority::NORMAL,
    );
}

fn condition_matches(condition: &GlobalKeymapCondition, context: &GlobalKeymapContext) -> bool {
    match condition {
        GlobalKeymapCondition::Always => true,
        GlobalKeymapCondition::AcceptsInput => context.accepts_input,
        GlobalKeymapCondition::EmptyComposer => context.composer_empty,
        GlobalKeymapCondition::PressWithInput => context.is_press && context.accepts_input,
        GlobalKeymapCondition::PressWithInputWithoutSelection => {
            context.is_press && context.accepts_input && !context.has_selection
        }
    }
}

fn key_stroke(key: &KeyEvent) -> Option<KeyStroke> {
    if key.modifiers.contains(KeyModifiers::HYPER) {
        return None;
    }
    let logical_key = LogicalKey::new(logical_key_name(key.code)?)?;
    let mut modifiers = Modifiers::none();
    if key.modifiers.contains(KeyModifiers::CONTROL) {
        modifiers = modifiers.with_control();
    }
    if key.modifiers.contains(KeyModifiers::SHIFT) || key.code == KeyCode::BackTab {
        modifiers = modifiers.with_shift();
    }
    if key.modifiers.contains(KeyModifiers::ALT) {
        modifiers = modifiers.with_alt();
    }
    if key
        .modifiers
        .intersects(KeyModifiers::SUPER | KeyModifiers::META)
    {
        modifiers = modifiers.with_meta();
    }
    Some(KeyStroke::new(logical_key, None, modifiers))
}

fn logical_key_name(code: KeyCode) -> Option<String> {
    let name = match code {
        KeyCode::Backspace => "backspace",
        KeyCode::Enter => "enter",
        KeyCode::Left => "arrowleft",
        KeyCode::Right => "arrowright",
        KeyCode::Up => "arrowup",
        KeyCode::Down => "arrowdown",
        KeyCode::Home => "home",
        KeyCode::End => "end",
        KeyCode::PageUp => "pageup",
        KeyCode::PageDown => "pagedown",
        KeyCode::Tab | KeyCode::BackTab => "tab",
        KeyCode::Delete => "delete",
        KeyCode::Insert => "insert",
        KeyCode::Esc => "escape",
        KeyCode::Char(character) => return Some(character.to_string()),
        KeyCode::F(number) => return Some(format!("f{number}")),
        _ => return None,
    };
    Some(name.to_owned())
}

#[cfg(test)]
#[path = "keymap_tests.rs"]
mod tests;
