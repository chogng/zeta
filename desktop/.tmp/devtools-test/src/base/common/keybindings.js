import { operatingSystem, OperatingSystem, } from "./platform.js";
export var KeybindingChordKind;
(function (KeybindingChordKind) {
    KeybindingChordKind["Logical"] = "logical";
    KeybindingChordKind["Physical"] = "physical";
})(KeybindingChordKind || (KeybindingChordKind = {}));
/** Creates a layout-aware chord without exposing representation details. */
export function logicalKey(key, modifiers = {}) {
    const normalizedKey = normalizeLogicalKey(key);
    if (!normalizedKey)
        throw new TypeError("Logical key must not be empty");
    return {
        kind: KeybindingChordKind.Logical,
        key: normalizedKey,
        modifiers: { ...modifiers },
    };
}
/** Creates a physical chord from a browser `KeyboardEvent.code` value. */
export function physicalKey(code, modifiers = {}) {
    const normalizedCode = code.trim();
    if (!normalizedCode)
        throw new TypeError("Physical key code must not be empty");
    return {
        kind: KeybindingChordKind.Physical,
        code: normalizedCode,
        modifiers: { ...modifiers },
    };
}
/** One command shortcut consisting of one or more ordered chords. */
export class Keybinding {
    chords;
    constructor(chords) {
        if (chords.length === 0) {
            throw new TypeError("A keybinding requires at least one chord");
        }
        this.chords = [...chords];
    }
    static single(chord) {
        return new Keybinding([chord]);
    }
    static chord(first, second, ...remaining) {
        return new Keybinding([first, second, ...remaining]);
    }
}
/** A keybinding ready for matching and presentation on one host OS. */
export class ResolvedKeybinding {
    chords;
    operatingSystem;
    constructor(chords, operatingSystem) {
        this.chords = chords;
        this.operatingSystem = operatingSystem;
    }
}
export function resolveKeybinding(keybinding, targetOperatingSystem = operatingSystem, physicalKeyLabels) {
    return new ResolvedKeybinding(keybinding.chords.map((chord) => resolveChord(chord, targetOperatingSystem, physicalKeyLabels)), targetOperatingSystem);
}
export function matchesResolvedChord(chord, event) {
    const keyMatches = chord.kind === KeybindingChordKind.Physical
        ? chord.key === event.code
        : chord.key === normalizeLogicalKey(event.key);
    return keyMatches &&
        chord.ctrlKey === event.ctrlKey &&
        chord.shiftKey === event.shiftKey &&
        chord.altKey === event.altKey &&
        chord.metaKey === event.metaKey;
}
function resolveChord(chord, targetOperatingSystem, physicalKeyLabels) {
    const modifiers = chord.modifiers;
    const primaryKey = Boolean(modifiers.primaryKey);
    const primaryIsMeta = targetOperatingSystem === OperatingSystem.Macintosh;
    return {
        kind: chord.kind,
        key: chord.kind === KeybindingChordKind.Physical
            ? chord.code
            : chord.key,
        label: chord.kind === KeybindingChordKind.Physical
            ? physicalKeyLabels?.get(chord.code)
            : undefined,
        ctrlKey: Boolean(modifiers.ctrlKey) ||
            (primaryKey && !primaryIsMeta),
        shiftKey: Boolean(modifiers.shiftKey),
        altKey: Boolean(modifiers.altKey),
        metaKey: Boolean(modifiers.metaKey) ||
            (primaryKey && primaryIsMeta),
    };
}
function normalizeLogicalKey(key) {
    const trimmed = key.trim();
    if (key === " " || trimmed.toLocaleLowerCase("en-US") === "space") {
        return " ";
    }
    return trimmed.toLocaleLowerCase("en-US");
}
