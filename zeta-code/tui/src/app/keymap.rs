use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyEventKind;
use crossterm::event::KeyModifiers;
use std::time::Duration;
use std::time::Instant;
use zeta_keybinding::BindingPriority;
use zeta_keybinding::BindingSet;
use zeta_keybinding::BindingSource;
use zeta_keybinding::Chord;
use zeta_keybinding::HostPlatform;
use zeta_keybinding::KeyIdentity;
use zeta_keybinding::KeySequence;
use zeta_keybinding::KeyStroke;
use zeta_keybinding::KeybindingResolver;
use zeta_keybinding::LogicalKey;
use zeta_keybinding::Modifiers;
use zeta_keybinding::ResolveResult;
use zeta_keybinding::ShortcutModifiers;
use zeta_keybinding::format_key_sequence;
use zeta_keybinding::parse_key_sequence;

const KEY_CHORD_TIMEOUT: Duration = Duration::from_secs(1);

/// Cross-component actions owned by the Zeta Code TUI root.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum AppKeymapAction {
    CycleApprovalMode,
    RootEscape,
    ReadClipboardImage,
    InterruptOrQuit,
    CopyLastResponse,
    Suspend,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum AppKeymapCondition {
    Always,
    AcceptsInput,
    EmptyComposer,
    PressWithInput,
    PressWithInputWithoutSelection,
}

/// State needed to decide a root binding without exposing component internals to the resolver.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct AppKeymapContext {
    pub(super) accepts_input: bool,
    pub(super) has_selection: bool,
    pub(super) composer_empty: bool,
    pub(super) is_press: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct AppKeybindingSpec {
    keybinding: &'static str,
    action: AppKeymapAction,
    condition: AppKeymapCondition,
    help_label: &'static str,
    help_description: &'static str,
}

const APP_KEYBINDINGS: &[AppKeybindingSpec] = &[
    AppKeybindingSpec {
        keybinding: "shift+tab",
        action: AppKeymapAction::CycleApprovalMode,
        condition: AppKeymapCondition::PressWithInputWithoutSelection,
        help_label: "Shift-Tab",
        help_description: "cycle approval mode for the next turn",
    },
    AppKeybindingSpec {
        keybinding: "escape",
        action: AppKeymapAction::RootEscape,
        condition: AppKeymapCondition::PressWithInput,
        help_label: "Esc Esc",
        help_description: "open rewind checkpoints from the root view",
    },
    AppKeybindingSpec {
        keybinding: "ctrl+v",
        action: AppKeymapAction::ReadClipboardImage,
        condition: AppKeymapCondition::AcceptsInput,
        help_label: "Ctrl-V",
        help_description: "attach an image from the system clipboard",
    },
    AppKeybindingSpec {
        keybinding: "ctrl+c",
        action: AppKeymapAction::InterruptOrQuit,
        condition: AppKeymapCondition::Always,
        help_label: "Ctrl-C",
        help_description: "interrupt an active turn or exit while idle",
    },
    AppKeybindingSpec {
        keybinding: "ctrl+d",
        action: AppKeymapAction::InterruptOrQuit,
        condition: AppKeymapCondition::EmptyComposer,
        help_label: "Ctrl-D",
        help_description: "interrupt or exit when the composer is empty",
    },
    AppKeybindingSpec {
        keybinding: "ctrl+o",
        action: AppKeymapAction::CopyLastResponse,
        condition: AppKeymapCondition::Always,
        help_label: "Ctrl-O",
        help_description: "copy the latest Zeta response",
    },
    AppKeybindingSpec {
        keybinding: "ctrl+z",
        action: AppKeymapAction::Suspend,
        condition: AppKeymapCondition::Always,
        help_label: "Ctrl-Z",
        help_description: "suspend Zeta on Unix and restore it after fg",
    },
];

#[derive(Clone, Debug, Eq, PartialEq)]
struct PendingChord {
    events: Vec<KeyStroke>,
    sequence: KeySequence,
    context: AppKeymapContext,
    started_at: Instant,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum AppChordMatch {
    PassThrough,
    Pending,
    Command(AppKeymapAction),
    Consumed,
}

/// Application-level shortcuts that sit above individual TUI components.
#[derive(Debug)]
pub(super) struct AppKeymap {
    single_bindings: BindingSet<AppKeymapCondition, AppKeymapAction>,
    chord_bindings: BindingSet<AppKeymapCondition, AppKeymapAction>,
    platform: HostPlatform,
    pending: Option<PendingChord>,
}

impl Default for AppKeymap {
    fn default() -> Self {
        Self::from_specs(APP_KEYBINDINGS)
    }
}

impl AppKeymap {
    fn from_specs(specs: &[AppKeybindingSpec]) -> Self {
        let parsed = specs
            .iter()
            .map(|binding| {
                (
                    *binding,
                    parse_key_sequence(binding.keybinding)
                        .expect("fixed TUI binding must use portable keybinding syntax"),
                )
            })
            .collect::<Vec<_>>();
        validate_specs(&parsed);
        let mut single_bindings = BindingSet::default();
        let mut chord_bindings = BindingSet::default();
        for (binding, keybinding) in parsed {
            register(
                &mut single_bindings,
                &mut chord_bindings,
                binding,
                keybinding,
            );
        }
        Self {
            single_bindings,
            chord_bindings,
            platform: HostPlatform::current(),
            pending: None,
        }
    }

    pub(super) fn resolve_single(
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

    pub(super) fn route_chord(
        &mut self,
        key: &KeyEvent,
        context: AppKeymapContext,
        now: Instant,
    ) -> AppChordMatch {
        self.expire(context, now);
        if self.pending.is_some() && key.kind != KeyEventKind::Press {
            return AppChordMatch::Consumed;
        }
        if key.kind != KeyEventKind::Press {
            return AppChordMatch::PassThrough;
        }
        if self.pending.is_some() && key.code == KeyCode::Esc && key.modifiers == KeyModifiers::NONE
        {
            self.cancel_chord();
            return AppChordMatch::Consumed;
        }
        let Some(normalized) = normalized_key(key) else {
            self.pending = None;
            return AppChordMatch::PassThrough;
        };
        let had_pending = self.pending.is_some();
        let mut events = self
            .pending
            .as_ref()
            .map(|pending| pending.events.clone())
            .unwrap_or_default();
        let mut chords = self
            .pending
            .as_ref()
            .map(|pending| pending.sequence.chords().to_vec())
            .unwrap_or_default();
        events.push(normalized.stroke.clone());
        chords.push(normalized.chord.clone());
        if let Some(result) = self.resolve_chord_prefix(events, chords, context, now) {
            return result;
        }
        self.pending = None;
        if had_pending
            && let Some(result) = self.resolve_chord_prefix(
                vec![normalized.stroke],
                vec![normalized.chord],
                context,
                now,
            )
        {
            return result;
        }
        AppChordMatch::PassThrough
    }

    pub(super) fn expire(&mut self, context: AppKeymapContext, now: Instant) -> bool {
        if self.pending.as_ref().is_some_and(|pending| {
            pending.context != context
                || now.saturating_duration_since(pending.started_at) >= KEY_CHORD_TIMEOUT
        }) {
            self.pending = None;
            return true;
        }
        false
    }

    fn cancel_chord(&mut self) -> bool {
        self.pending.take().is_some()
    }

    pub(super) fn pending_chord_label(&self) -> Option<String> {
        self.pending
            .as_ref()
            .map(|pending| format_key_sequence(&pending.sequence, self.platform))
    }

    fn resolve_chord_prefix(
        &mut self,
        events: Vec<KeyStroke>,
        chords: Vec<Chord>,
        context: AppKeymapContext,
        now: Instant,
    ) -> Option<AppChordMatch> {
        let resolver = KeybindingResolver::new(&self.chord_bindings, self.platform);
        match resolver.resolve(&context, &events, condition_matches) {
            ResolveResult::NoMatch => None,
            ResolveResult::PendingChord { .. } => {
                self.pending = Some(PendingChord {
                    events,
                    sequence: KeySequence::new(chords)
                        .expect("a pending TUI chord must contain one to four keys"),
                    context,
                    started_at: now,
                });
                Some(AppChordMatch::Pending)
            }
            ResolveResult::Command { command, .. } => {
                self.pending = None;
                Some(AppChordMatch::Command(command))
            }
            ResolveResult::Blocked { .. } => {
                self.pending = None;
                Some(AppChordMatch::Consumed)
            }
        }
    }
}

pub(super) fn app_keybinding_help_items() -> impl Iterator<Item = (&'static str, &'static str)> {
    APP_KEYBINDINGS
        .iter()
        .map(|binding| (binding.help_label, binding.help_description))
}

fn register(
    single_bindings: &mut BindingSet<AppKeymapCondition, AppKeymapAction>,
    chord_bindings: &mut BindingSet<AppKeymapCondition, AppKeymapAction>,
    binding: AppKeybindingSpec,
    keybinding: KeySequence,
) {
    let bindings = if keybinding.chords().len() == 1 {
        single_bindings
    } else {
        chord_bindings
    };
    bindings.register_command(
        keybinding,
        binding.action,
        binding.condition,
        BindingSource::Builtin,
        BindingPriority::NORMAL,
    );
}

fn validate_specs(specs: &[(AppKeybindingSpec, KeySequence)]) {
    for (binding, sequence) in specs {
        if sequence.chords().len() == 1 {
            continue;
        }
        assert!(
            !sequence.chords().iter().any(is_plain_escape),
            "fixed TUI chord `{}` cannot use plain Escape because Escape cancels pending chords",
            binding.keybinding,
        );
        let prefix = &sequence.chords()[0];
        assert!(
            is_safe_app_chord_prefix(prefix),
            "fixed TUI chord `{}` must use Control, Alt, Meta, primary, or a non-character prefix",
            binding.keybinding,
        );
        assert!(
            !specs.iter().any(|(_, candidate)| {
                candidate.chords().len() == 1 && candidate.chords().first() == Some(prefix)
            }),
            "fixed TUI chord `{}` shadows an application-level single-key binding",
            binding.keybinding,
        );
    }
}

fn is_plain_escape(chord: &Chord) -> bool {
    chord.modifiers() == ShortcutModifiers::none()
        && matches!(
            chord.key(),
            KeyIdentity::Logical(key) if key.as_str() == "escape"
        )
}

fn is_safe_app_chord_prefix(chord: &Chord) -> bool {
    let modifiers = chord.modifiers();
    let character_key = matches!(
        chord.key(),
        KeyIdentity::Logical(key) if key.as_str().chars().count() == 1
    );
    if !character_key {
        return true;
    }
    let control_or_primary = modifiers.uses_control() || modifiers.uses_primary();
    let alt = modifiers.uses_alt();
    let meta = modifiers.uses_meta();
    (control_or_primary || alt || meta) && !(control_or_primary && alt)
}

fn condition_matches(condition: &AppKeymapCondition, context: &AppKeymapContext) -> bool {
    match condition {
        AppKeymapCondition::Always => true,
        AppKeymapCondition::AcceptsInput => context.accepts_input,
        AppKeymapCondition::EmptyComposer => context.composer_empty,
        AppKeymapCondition::PressWithInput => context.is_press && context.accepts_input,
        AppKeymapCondition::PressWithInputWithoutSelection => {
            context.is_press && context.accepts_input && !context.has_selection
        }
    }
}

struct NormalizedKey {
    stroke: KeyStroke,
    chord: Chord,
}

fn normalized_key(key: &KeyEvent) -> Option<NormalizedKey> {
    if key.modifiers.contains(KeyModifiers::HYPER) {
        return None;
    }
    let logical_key_name = logical_key_name(key.code)?;
    let logical_key = LogicalKey::new(logical_key_name.clone())?;
    let mut modifiers = Modifiers::none();
    let mut shortcut_modifiers = ShortcutModifiers::none();
    if key.modifiers.contains(KeyModifiers::CONTROL) {
        modifiers = modifiers.with_control();
        shortcut_modifiers = shortcut_modifiers.with_control();
    }
    if key.modifiers.contains(KeyModifiers::SHIFT) || key.code == KeyCode::BackTab {
        modifiers = modifiers.with_shift();
        shortcut_modifiers = shortcut_modifiers.with_shift();
    }
    if key.modifiers.contains(KeyModifiers::ALT) {
        modifiers = modifiers.with_alt();
        shortcut_modifiers = shortcut_modifiers.with_alt();
    }
    if key
        .modifiers
        .intersects(KeyModifiers::SUPER | KeyModifiers::META)
    {
        modifiers = modifiers.with_meta();
        shortcut_modifiers = shortcut_modifiers.with_meta();
    }
    Some(NormalizedKey {
        stroke: KeyStroke::new(logical_key, None, modifiers),
        chord: Chord::logical(logical_key_name, shortcut_modifiers)?,
    })
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
