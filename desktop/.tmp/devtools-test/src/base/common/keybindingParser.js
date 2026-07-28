import { Keybinding, logicalKey, physicalKey, } from "./keybindings.js";
const modifierNames = new Map([
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
/**
 * Parses a user keybinding such as `ctrl+k ctrl+c` or `shift+[KeyP]`.
 *
 * Logical keys use `KeyboardEvent.key`; bracketed keys use the physical
 * `KeyboardEvent.code`. Invalid or ambiguous input returns `undefined`.
 */
export function parseKeybinding(value) {
    const chordTexts = value.trim().split(/\s+/).filter(Boolean);
    if (chordTexts.length === 0)
        return undefined;
    const chords = [];
    for (const chordText of chordTexts) {
        const chord = parseChord(chordText);
        if (!chord)
            return undefined;
        chords.push(chord);
    }
    return new Keybinding(chords);
}
function parseChord(value) {
    const tokens = value.split("+").map((token) => token.trim());
    if (tokens.some((token) => token.length === 0))
        return undefined;
    const modifiers = {
        primaryKey: false,
        ctrlKey: false,
        shiftKey: false,
        altKey: false,
        metaKey: false,
    };
    let key;
    for (const token of tokens) {
        const modifier = modifierNames.get(token.toLocaleLowerCase("en-US"));
        if (modifier) {
            if (modifiers[modifier])
                return undefined;
            modifiers[modifier] = true;
            continue;
        }
        if (key !== undefined)
            return undefined;
        key = token;
    }
    if (!key)
        return undefined;
    if (modifiers.primaryKey && (modifiers.ctrlKey || modifiers.metaKey)) {
        return undefined;
    }
    const chordModifiers = modifiers.primaryKey
        ? {
            primaryKey: true,
            shiftKey: modifiers.shiftKey,
            altKey: modifiers.altKey,
        }
        : {
            ctrlKey: modifiers.ctrlKey,
            shiftKey: modifiers.shiftKey,
            altKey: modifiers.altKey,
            metaKey: modifiers.metaKey,
        };
    const physicalMatch = /^\[([^\]]+)\]$/.exec(key);
    return physicalMatch
        ? physicalKey(physicalMatch[1], chordModifiers)
        : logicalKey(key, chordModifiers);
}
