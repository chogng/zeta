import assert from "node:assert/strict";
import test from "node:test";
import { StandardKeyboardEvent, } from "../../browser/keyboardEvent.js";
function keyboardEvent(overrides = {}) {
    return {
        key: "p",
        code: "KeyP",
        ctrlKey: true,
        shiftKey: false,
        altKey: false,
        metaKey: false,
        repeat: false,
        isComposing: false,
        getModifierState: () => false,
        preventDefault: () => { },
        stopPropagation: () => { },
        stopImmediatePropagation: () => { },
        ...overrides,
    };
}
test("standard keyboard events match physical chords and modifiers", () => {
    const event = new StandardKeyboardEvent(keyboardEvent());
    assert.equal(event.matches({ code: "KeyP", ctrlKey: true }), true);
    assert.equal(event.matches({ code: "KeyP" }), false);
    assert.equal(event.matches({ code: "KeyO", ctrlKey: true }), false);
});
test("composing keyboard events do not trigger physical chords", () => {
    const event = new StandardKeyboardEvent(keyboardEvent({
        isComposing: true,
    }));
    assert.equal(event.matches({ code: "KeyP", ctrlKey: true }), false);
});
test("AltGraph does not masquerade as a Ctrl+Alt shortcut", () => {
    const event = new StandardKeyboardEvent(keyboardEvent({
        altKey: true,
        getModifierState: (key) => key === "AltGraph",
    }));
    assert.equal(event.matches({ code: "KeyP", ctrlKey: true, altKey: true }), false);
});
