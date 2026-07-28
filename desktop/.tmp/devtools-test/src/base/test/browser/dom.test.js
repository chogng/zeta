import assert from "node:assert/strict";
import test from "node:test";
import { addDisposableListener, stopEvent, } from "../../browser/dom.js";
test("disposable DOM listeners detach deterministically", () => {
    const target = new EventTarget();
    let calls = 0;
    const registration = addDisposableListener(target, "change", () => calls++);
    target.dispatchEvent(new Event("change"));
    registration.dispose();
    registration.dispose();
    target.dispatchEvent(new Event("change"));
    assert.equal(calls, 1);
});
test("stopEvent prevents native behavior and propagation", () => {
    const event = new Event("submit", {
        bubbles: true,
        cancelable: true,
    });
    let propagated = false;
    const target = new EventTarget();
    target.addEventListener("submit", (next) => stopEvent(next, { immediate: true }));
    target.addEventListener("submit", () => {
        propagated = true;
    });
    const accepted = target.dispatchEvent(event);
    assert.equal(accepted, false);
    assert.equal(event.defaultPrevented, true);
    assert.equal(propagated, false);
});
