import assert from "node:assert/strict";
import test from "node:test";
import { JSDOM } from "jsdom";
test("toolbar submenu items retain toolbar button semantics", async () => {
    const dom = new JSDOM("<!doctype html><body></body>");
    Object.defineProperty(globalThis, "window", {
        configurable: true,
        value: dom.window,
    });
    const [{ MenuId, SubmenuItemAction }, { createMenuEntryActionViewItem },] = await Promise.all([
        import("../src/platform/actions/common/actions.js"),
        import("../src/platform/actions/browser/menuEntryActionViewItem.js"),
    ]);
    let shownOptions;
    const childAction = {
        id: "test.toolbar.child",
        label: "Child",
        tooltip: "Child",
        enabled: true,
        run() { },
    };
    const item = createMenuEntryActionViewItem(new SubmenuItemAction({
        title: "More",
        submenu: MenuId.for("test.toolbar.submenu"),
    }, [childAction]), {
        showContextMenu(options) {
            shownOptions = options;
        },
    });
    assert.ok(item);
    const container = dom.window.document.createElement("div");
    dom.window.document.body.append(container);
    item.render(container);
    const button = container.querySelector("button");
    assert.ok(button instanceof dom.window.HTMLButtonElement);
    assert.equal(button.hasAttribute("role"), false);
    assert.equal(button.getAttribute("aria-haspopup"), "menu");
    assert.ok(button.querySelector(".zeta-dropdown-menu-indicator > svg.zeta-icon"));
    button.click();
    assert.ok(shownOptions);
    assert.equal(shownOptions.anchor, button);
    assert.deepEqual(shownOptions.actions, [childAction]);
    assert.equal(button.getAttribute("aria-expanded"), "true");
    shownOptions.onHide?.(false);
    assert.equal(button.getAttribute("aria-expanded"), "false");
    item.dispose();
    dom.window.close();
    Reflect.deleteProperty(globalThis, "window");
});
