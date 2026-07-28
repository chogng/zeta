import {
  Keybinding,
  type KeybindingChord,
  logicalKey,
  physicalKey,
} from "./keybindings.js";

const modifierNames = new Map<string, ModifierName>([
  ["ctrl", "ctrlKey"],
  ["control", "ctrlKey"],
  ["shift", "shiftKey"],
  ["alt", "altKey"],
  ["option", "altKey"],
  ["meta", "metaKey"],
  ["cmd", "metaKey"],
  ["command", "metaKey"],
  ["win", "metaKey"],
  ["windows", "metaKey"],
  ["primary", "primaryKey"],
]);

type ModifierName =
  | "primaryKey"
  | "ctrlKey"
  | "shiftKey"
  | "altKey"
  | "metaKey";

/**
 * Parses a user keybinding such as `ctrl+k ctrl+c` or `shift+[KeyP]`.
 *
 * Logical keys use `KeyboardEvent.key`; bracketed keys use the physical
 * `KeyboardEvent.code`. Invalid or ambiguous input returns `undefined`.
 */
export function parseKeybinding(value: string): Keybinding | undefined {
  const chordTexts = value.trim().split(/\s+/).filter(Boolean);
  if (chordTexts.length === 0) return undefined;

  const chords: KeybindingChord[] = [];
  for (const chordText of chordTexts) {
    const chord = parseChord(chordText);
    if (!chord) return undefined;
    chords.push(chord);
  }
  return new Keybinding(chords);
}

function parseChord(value: string): KeybindingChord | undefined {
  const tokens = value.split("+").map((token) => token.trim());
  if (tokens.some((token) => token.length === 0)) return undefined;

  const modifiers: Record<ModifierName, boolean> = {
    primaryKey: false,
    ctrlKey: false,
    shiftKey: false,
    altKey: false,
    metaKey: false,
  };
  let key: string | undefined;

  for (const token of tokens) {
    const modifier = modifierNames.get(token.toLocaleLowerCase("en-US"));
    if (modifier) {
      if (modifiers[modifier]) return undefined;
      modifiers[modifier] = true;
      continue;
    }
    if (key !== undefined) return undefined;
    key = token;
  }

  if (!key) return undefined;
  if (modifiers.primaryKey && (modifiers.ctrlKey || modifiers.metaKey)) {
    return undefined;
  }

  const chordModifiers = modifiers.primaryKey
    ? {
      primaryKey: true,
      shiftKey: modifiers.shiftKey,
      altKey: modifiers.altKey,
    } as const
    : {
      ctrlKey: modifiers.ctrlKey,
      shiftKey: modifiers.shiftKey,
      altKey: modifiers.altKey,
      metaKey: modifiers.metaKey,
    } as const;
  const physicalMatch = /^\[([^\]]+)\]$/.exec(key);
  return physicalMatch
    ? physicalKey(physicalMatch[1], chordModifiers)
    : logicalKey(key, chordModifiers);
}
