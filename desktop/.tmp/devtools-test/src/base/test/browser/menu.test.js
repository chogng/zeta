import assert from "node:assert/strict";
import test from "node:test";
import { JSDOM } from "jsdom";
test("Menu renders submenu actions with the shared SVG indicator", async () => {
    const dom = new JSDOM("<!doctype html><body></body>");
    Object.defineProperty(globalThis, "window", {
        configurable: true,
        value: dom.window,
    });
    const [{ SubmenuAction }, { Menu }] = await Promise.all([
        import("../../common/actions.js"),
        import("../../browser/ui/menu/menu.js"),
    ]);
    const menu = new Menu({
        actions: [new SubmenuAction("test.submenu", "More", [])],
        ownerDocument: dom.window.document,
    });
    dom.window.document.body.append(menu.element);
    const menuItem = menu.element.querySelector('[role="menuitem"]');
    assert.ok(menuItem instanceof dom.window.HTMLButtonElement);
    assert.equal(menuItem.getAttribute("aria-haspopup"), "menu");
    assert.equal(menuItem.getAttribute("aria-expanded"), "false");
    const indicator = menuItem.querySelector(".zeta-submenu-indicator");
    assert.ok(indicator instanceof dom.window.HTMLSpanElement);
    const icon = indicator.querySelector(":scope > svg.zeta-icon");
    assert.ok(icon instanceof dom.window.SVGElement);
    assert.equal(icon.getAttribute("aria-hidden"), "true");
    assert.equal(indicator.textContent, "");
    menu.dispose();
    dom.window.close();
    Reflect.deleteProperty(globalThis, "window");
});
