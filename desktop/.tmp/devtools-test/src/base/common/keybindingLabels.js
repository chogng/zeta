import { KeybindingChordKind, } from "./keybindings.js";
import { OperatingSystem } from "./platform.js";
export var KeybindingLabelStyle;
(function (KeybindingLabelStyle) {
    KeybindingLabelStyle["UI"] = "ui";
    KeybindingLabelStyle["Aria"] = "aria";
    KeybindingLabelStyle["UserSettings"] = "userSettings";
})(KeybindingLabelStyle || (KeybindingLabelStyle = {}));
/** Returns one visual and accessible label for each chord. */
export function getKeybindingLabelParts(keybinding) {
    return keybinding.chords.map((chord) => ({
        label: formatChord(chord, keybinding.operatingSystem, KeybindingLabelStyle.UI),
        ariaLabel: formatChord(chord, keybinding.operatingSystem, KeybindingLabelStyle.Aria),
    }));
}
/** Formats a resolved keybinding for UI, accessibility, or settings. */
export function getKeybindingLabel(keybinding, style = KeybindingLabelStyle.UI) {
    const separator = style === KeybindingLabelStyle.Aria ? ", " : " ";
    return keybinding.chords
        .map((chord) => formatChord(chord, keybinding.operatingSystem, style))
        .join(separator);
}
function formatChord(chord, operatingSystem, style) {
    if (style === KeybindingLabelStyle.UserSettings) {
        return formatUserSettingsChord(chord, operatingSystem);
    }
    const labels = style === KeybindingLabelStyle.Aria
        ? ariaModifierLabels(operatingSystem)
        : uiModifierLabels(operatingSystem);
    const parts = [];
    if (chord.ctrlKey)
        parts.push(labels.ctrl);
    if (chord.shiftKey)
        parts.push(labels.shift);
    if (chord.altKey)
        parts.push(labels.alt);
    if (chord.metaKey)
        parts.push(labels.meta);
    parts.push(displayKey(chord));
    return parts.join(labels.separator);
}
function formatUserSettingsChord(chord, operatingSystem) {
    const parts = [];
    if (chord.ctrlKey)
        parts.push("ctrl");
    if (chord.shiftKey)
        parts.push("shift");
    if (chord.altKey)
        parts.push("alt");
    if (chord.metaKey) {
        parts.push(operatingSystem === OperatingSystem.Macintosh
            ? "cmd"
            : operatingSystem === OperatingSystem.Windows
                ? "win"
                : "meta");
    }
    parts.push(chord.kind === KeybindingChordKind.Physical
        ? `[${chord.key}]`
        : chord.key);
    return parts.join("+");
}
function uiModifierLabels(operatingSystem) {
    if (operatingSystem === OperatingSystem.Macintosh) {
        return {
            ctrl: "⌃",
            shift: "⇧",
            alt: "⌥",
            meta: "⌘",
            separator: "",
        };
    }
    return {
        ctrl: "Ctrl",
        shift: "Shift",
        alt: "Alt",
        meta: operatingSystem === OperatingSystem.Windows ? "Windows" : "Super",
        separator: "+",
    };
}
function ariaModifierLabels(operatingSystem) {
    return {
        ctrl: "Control",
        shift: "Shift",
        alt: operatingSystem === OperatingSystem.Macintosh ? "Option" : "Alt",
        meta: operatingSystem === OperatingSystem.Macintosh
            ? "Command"
            : operatingSystem === OperatingSystem.Windows
                ? "Windows"
                : "Super",
        separator: "+",
    };
}
function displayKey(chord) {
    if (chord.label) {
        return chord.label.length === 1
            ? chord.label.toLocaleUpperCase("en-US")
            : chord.label;
    }
    if (chord.kind === KeybindingChordKind.Physical) {
        if (/^Key[A-Z]$/.test(chord.key))
            return chord.key.slice(3);
        if (/^Digit[0-9]$/.test(chord.key))
            return chord.key.slice(5);
    }
    const knownKeys = {
        arrowdown: "Down",
        arrowleft: "Left",
        arrowright: "Right",
        arrowup: "Up",
        escape: "Escape",
        enter: "Enter",
        backspace: "Backspace",
        delete: "Delete",
        pageup: "Page Up",
        pagedown: "Page Down",
        " ": "Space",
    };
    return knownKeys[chord.key.toLocaleLowerCase("en-US")] ??
        (chord.key.length === 1 ? chord.key.toLocaleUpperCase("en-US") : chord.key);
}
