import { stopEvent } from "./dom.js";
/**
 * Stable physical-key view for shortcut and keybinding matching.
 *
 * Local widget navigation should normally read `KeyboardEvent.key` directly.
 * Construct this representation when an event crosses a component boundary or
 * must be compared with a reusable physical key chord.
 */
export class StandardKeyboardEvent {
    browserEvent;
    key;
    code;
    ctrlKey;
    shiftKey;
    altKey;
    metaKey;
    altGraphKey;
    isComposing;
    repeat;
    constructor(browserEvent) {
        this.browserEvent = browserEvent;
        this.key = browserEvent.key;
        this.code = browserEvent.code;
        this.ctrlKey = browserEvent.ctrlKey;
        this.shiftKey = browserEvent.shiftKey;
        this.altKey = browserEvent.altKey;
        this.metaKey = browserEvent.metaKey;
        this.altGraphKey = browserEvent.getModifierState?.("AltGraph") ?? false;
        this.isComposing = browserEvent.isComposing;
        this.repeat = browserEvent.repeat;
    }
    matches(chord) {
        return !this.isComposing &&
            !this.altGraphKey &&
            this.code === chord.code &&
            this.ctrlKey === Boolean(chord.ctrlKey) &&
            this.shiftKey === Boolean(chord.shiftKey) &&
            this.altKey === Boolean(chord.altKey) &&
            this.metaKey === Boolean(chord.metaKey);
    }
    stop(options) {
        stopEvent(this.browserEvent, options);
    }
}
export function hasModifierKeys(event) {
    return event.ctrlKey || event.shiftKey || event.altKey || event.metaKey;
}
export function isModifierKey(event) {
    return event.key === "Control" ||
        event.key === "Shift" ||
        event.key === "Alt" ||
        event.key === "Meta";
}
