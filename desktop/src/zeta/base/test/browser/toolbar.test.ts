import assert from "node:assert/strict";
import test from "node:test";
import { JSDOM } from "jsdom";
import type { IActionContextMenuOptions, IContextMenuProvider } from "../../browser/contextmenu.js";
import { ToolBar } from "../../browser/ui/toolbar/toolbar.js";
import type { IAction } from "../../common/actions.js";
import { Separator } from "../../common/actions.js";

test("ToolBar renders primary actions and trails More Actions", () => {
  const dom = new JSDOM("<!doctype html><body></body>");
  const contextMenuProvider = new TestContextMenuProvider();
  const toolbar = new ToolBar({
    contextMenuProvider,
    ownerDocument: dom.window.document,
    ariaLabel: "Test actions",
  });
  toolbar.setActions(
    [action("primary")],
    [action("secondary")],
  );
  dom.window.document.body.append(toolbar.element);

  const buttons = toolbar.element.querySelectorAll("button");
  assert.equal(toolbar.element.getAttribute("role"), "toolbar");
  assert.equal(toolbar.element.getAttribute("aria-label"), "Test actions");
  assert.equal(toolbar.element.classList.contains("zeta-toolbar-default"), true);
  assert.equal(buttons.length, 2);
  assert.equal(buttons[0]?.textContent, "primary");
  assert.equal(buttons[1]?.title, "More Actions");
  assert.equal(buttons[1]?.getAttribute("aria-haspopup"), "menu");
  assert.equal(toolbar.element.textContent?.includes("secondary"), false);

  buttons[1]?.click();
  assert.deepEqual(
    contextMenuProvider.lastOptions?.actions.map(({ id }) => id),
    ["secondary"],
  );
  assert.equal(buttons[1]?.getAttribute("aria-expanded"), "true");
  contextMenuProvider.lastOptions?.onHide?.(false);
  assert.equal(buttons[1]?.getAttribute("aria-expanded"), "false");

  toolbar.dispose();
  dom.window.close();
});

test("ToolBar exposes component-owned foreground presentations", () => {
  const dom = new JSDOM("<!doctype html><body></body>");
  const toolbar = new ToolBar({
    contextMenuProvider: new TestContextMenuProvider(),
    ownerDocument: dom.window.document,
    presentation: "inherit-foreground",
  });

  assert.equal(toolbar.element.classList.contains("zeta-toolbar-inherit-foreground"), true);

  toolbar.dispose();
  dom.window.close();
});

test("ToolBar highlights checked actions only when requested", () => {
  const dom = new JSDOM("<!doctype html><body></body>");
  const toolbar = new ToolBar({
    contextMenuProvider: new TestContextMenuProvider(),
    ownerDocument: dom.window.document,
    highlightToggledItems: true,
  });
  toolbar.setActions([{ ...action("checked"), checked: true }]);

  const button = toolbar.element.querySelector("button");
  assert.equal(toolbar.element.classList.contains("highlight-toggled"), true);
  assert.equal(button?.classList.contains("checked"), true);
  assert.equal(button?.getAttribute("aria-pressed"), "true");

  toolbar.dispose();
  dom.window.close();
});

test("ToolBar omits More Actions when secondary actions are empty", () => {
  const dom = new JSDOM("<!doctype html><body></body>");
  const toolbar = new ToolBar({
    contextMenuProvider: new TestContextMenuProvider(),
    ownerDocument: dom.window.document,
  });
  toolbar.setActions(
    [new Separator(), action("primary"), new Separator()],
    [new Separator()],
  );

  const buttons = toolbar.element.querySelectorAll("button");
  assert.equal(buttons.length, 1);
  assert.equal(buttons[0]?.textContent, "primary");

  toolbar.dispose();
  dom.window.close();
});

test("ToolBar can place More Actions before a named primary action", () => {
  const dom = new JSDOM("<!doctype html><body></body>");
  const toolbar = new ToolBar({
    contextMenuProvider: new TestContextMenuProvider(),
    ownerDocument: dom.window.document,
    moreActionsPlacement: { beforeActionId: "maximize" },
  });
  toolbar.setActions(
    [action("kill"), action("maximize"), action("close")],
    [action("clear")],
  );

  assert.deepEqual(
    [...toolbar.element.querySelectorAll<HTMLElement>("[data-action-id]")]
      .map((item) => item.dataset.actionId),
    ["kill", "zeta.toolbar.moreActions", "maximize", "close"],
  );

  toolbar.dispose();
  dom.window.close();
});

class TestContextMenuProvider implements IContextMenuProvider {
  lastOptions: IActionContextMenuOptions | undefined;

  showContextMenu(options: IActionContextMenuOptions): void {
    this.lastOptions = options;
  }
}

function action(id: string): IAction {
  return {
    id,
    label: id,
    tooltip: id,
    enabled: true,
    run() {},
  };
}
