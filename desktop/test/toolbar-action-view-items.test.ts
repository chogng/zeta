import assert from "node:assert/strict";
import test from "node:test";
import { JSDOM } from "jsdom";

test("toolbar submenu items retain toolbar button semantics", async () => {
  const dom = new JSDOM("<!doctype html><body></body>");
  Object.defineProperty(globalThis, "window", {
    configurable: true,
    value: dom.window,
  });
  const [
    { MenuId, SubmenuItemAction },
    { createMenuEntryActionViewItem },
  ] =
    await Promise.all([
      import("../src/zeta/platform/actions/common/actions.js"),
      import("../src/zeta/platform/actions/browser/menuEntryActionViewItem.js"),
    ]);
  let shownOptions:
    | import("../src/zeta/base/browser/contextmenu.js").IActionContextMenuOptions
    | undefined;
  const childAction = {
    id: "test.toolbar.child",
    label: "Child",
    tooltip: "Child",
    enabled: true,
    run() {},
  };
  const item = createMenuEntryActionViewItem(
    new SubmenuItemAction({
      title: "More",
      submenu: MenuId.for("test.toolbar.submenu"),
    }, [childAction]),
    {
      showContextMenu(options) {
        shownOptions = options;
      },
    },
  );
  assert.ok(item);
  const container = dom.window.document.createElement("div");
  dom.window.document.body.append(container);

  item.render(container);

  const button = container.querySelector("button");
  assert.ok(button instanceof dom.window.HTMLButtonElement);
  assert.equal(button.hasAttribute("role"), false);
  assert.equal(button.getAttribute("aria-haspopup"), "menu");
  assert.ok(button.querySelector(
    ".zeta-dropdown-menu-indicator > svg.zeta-icon",
  ));
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

test("workbench toolbar adapts manually supplied platform menu actions", async () => {
  const dom = new JSDOM("<!doctype html><body></body>");
  Object.defineProperty(globalThis, "window", {
    configurable: true,
    value: dom.window,
  });
  const [
    { MenuId, MenusRegistry },
    { MenuService },
    { CommandsRegistry },
    { ContextKeyService },
    { ServiceCollection },
    { CommandService },
    { DisposableStore },
    { WorkbenchToolBar },
  ] = await Promise.all([
    import("../src/zeta/platform/actions/common/actions.js"),
    import("../src/zeta/platform/actions/common/menuService.js"),
    import("../src/zeta/platform/commands/common/commands.js"),
    import("../src/zeta/platform/contextkey/common/contextkey.js"),
    import("../src/zeta/platform/instantiation/common/instantiation.js"),
    import("../src/zeta/workbench/services/commands/common/commandService.js"),
    import("../src/zeta/base/common/lifecycle.js"),
    import("../src/zeta/platform/actions/browser/toolbar.js"),
  ]);
  using registrations = new DisposableStore();
  const menuId = new MenuId("test.toolbar.workbench");
  const commandId = "test.toolbar.workbench.action";
  registrations.add(CommandsRegistry.register(commandId, () => undefined));
  registrations.add(MenusRegistry.appendMenuItem(menuId, {
    command: { id: commandId, title: "Workbench action" },
    group: "navigation",
  }));
  const commands = new CommandService(new ServiceCollection());
  const contexts = registrations.add(new ContextKeyService());
  const menus = new MenuService(commands, contexts);
  const action = menus.getMenuActions(menuId)[0]?.[1][0];
  assert.ok(action);
  const toolbar = new WorkbenchToolBar({ showContextMenu() {} }, dom.window.document);
  toolbar.setActions([action]);
  dom.window.document.body.append(toolbar.element);

  assert.equal(toolbar.element.querySelector(".zeta-menu-entry button")?.textContent, "Workbench action");

  toolbar.dispose();
  dom.window.close();
  Reflect.deleteProperty(globalThis, "window");
});

test("menu toolbar keeps navigation inline and moves other groups into More Actions", async () => {
  const dom = new JSDOM("<!doctype html><body></body>");
  Object.defineProperty(globalThis, "window", {
    configurable: true,
    value: dom.window,
  });
  const [
    { MenuId, MenusRegistry },
    { MenuService },
    { CommandsRegistry },
    { ContextKeyService },
    { ServiceCollection },
    { CommandService },
    { DisposableStore },
    { MenuWorkbenchToolBar },
  ] = await Promise.all([
    import("../src/zeta/platform/actions/common/actions.js"),
    import("../src/zeta/platform/actions/common/menuService.js"),
    import("../src/zeta/platform/commands/common/commands.js"),
    import("../src/zeta/platform/contextkey/common/contextkey.js"),
    import("../src/zeta/platform/instantiation/common/instantiation.js"),
    import("../src/zeta/workbench/services/commands/common/commandService.js"),
    import("../src/zeta/base/common/lifecycle.js"),
    import("../src/zeta/platform/actions/browser/toolbar.js"),
  ]);
  using registrations = new DisposableStore();
  const menuId = new MenuId("test.toolbar.groups");
  registrations.add(CommandsRegistry.register(
    "test.toolbar.navigation",
    () => undefined,
  ));
  registrations.add(CommandsRegistry.register(
    "test.toolbar.secondary",
    () => undefined,
  ));
  registrations.add(MenusRegistry.appendMenuItem(menuId, {
    command: {
      id: "test.toolbar.navigation",
      title: "Navigation",
    },
    group: "navigation",
  }));
  registrations.add(MenusRegistry.appendMenuItem(menuId, {
    command: {
      id: "test.toolbar.secondary",
      title: "Secondary",
    },
    group: "other",
  }));
  const commands = new CommandService(new ServiceCollection());
  const contexts = registrations.add(new ContextKeyService());
  const menus = new MenuService(commands, contexts);
  let shownOptions:
    | import("../src/zeta/base/browser/contextmenu.js").IActionContextMenuOptions
    | undefined;
  const toolbar = new MenuWorkbenchToolBar(
    menus,
    {
      showContextMenu(options) {
        shownOptions = options;
      },
    },
    menuId,
    dom.window.document,
  );
  dom.window.document.body.append(toolbar.element);

  const buttons = toolbar.element.querySelectorAll("button");
  assert.equal(buttons.length, 2);
  assert.equal(buttons[0]?.textContent, "Navigation");
  assert.equal(buttons[1]?.title, "More Actions");
  assert.equal(toolbar.element.textContent?.includes("Secondary"), false);
  buttons[1]?.click();
  assert.deepEqual(
    shownOptions?.actions.map(({ id }) => id),
    ["test.toolbar.secondary"],
  );
  assert.throws(
    () => toolbar.setActions([]),
    /actions are owned by its MenuId/,
  );

  toolbar.dispose();
  dom.window.close();
  Reflect.deleteProperty(globalThis, "window");
});

test("More Actions opens an anchored Menu with actionable list items", async () => {
  const dom = new JSDOM("<!doctype html><body><main></main></body>");
  Object.defineProperty(dom.window.HTMLElement.prototype, "scrollTo", {
    configurable: true,
    value: () => {},
  });
  for (const [name, value] of Object.entries({
    window: dom.window,
    Node: dom.window.Node,
    Element: dom.window.Element,
    HTMLElement: dom.window.HTMLElement,
  })) {
    Object.defineProperty(globalThis, name, {
      configurable: true,
      value,
    });
  }
  const [
    { Separator },
    { toDisposable },
    { ToolBar },
    { ContextKeyService },
    { MenuService },
    { ServiceCollection },
    { CommandService },
    { BrowserContextViewService },
    { BrowserContextMenuService },
  ] = await Promise.all([
    import("../src/zeta/base/common/actions.js"),
    import("../src/zeta/base/common/lifecycle.js"),
    import("../src/zeta/base/browser/ui/toolbar/toolbar.js"),
    import("../src/zeta/platform/contextkey/common/contextkey.js"),
    import("../src/zeta/platform/actions/common/menuService.js"),
    import("../src/zeta/platform/instantiation/common/instantiation.js"),
    import("../src/zeta/workbench/services/commands/common/commandService.js"),
    import("../src/zeta/platform/contextview/browser/contextViewService.js"),
    import("../src/zeta/platform/contextview/browser/contextMenuService.js"),
  ]);
  const host = dom.window.document.querySelector<HTMLElement>("main");
  assert.ok(host);
  using contexts = new ContextKeyService();
  const commands = new CommandService(new ServiceCollection());
  const menus = new MenuService(commands, contexts);
  using contextViews = new BrowserContextViewService(host);
  const noEvent = () => toDisposable(() => {});
  using contextMenus = new BrowserContextMenuService(
    menus,
    {
      inChordMode: false,
      onDidUpdateKeybindings: noEvent,
      resolveKeybinding() { throw new Error("Not used"); },
      resolveUserBinding() { return undefined; },
      lookupKeybindings() { return []; },
      lookupKeybinding() { return undefined; },
    },
    contextViews,
  );
  let cleared = 0;
  using toolbar = new ToolBar({
    contextMenuProvider: contextMenus,
    ownerDocument: dom.window.document,
    moreActionsPlacement: { beforeActionId: "maximize" },
  });
  toolbar.setActions(
    [testAction("kill"), testAction("maximize"), testAction("close")],
    [
      testAction("clear", () => cleared++),
      new Separator(),
      { ...testAction("output"), checked: true },
    ],
  );
  host.append(toolbar.element);

  const more = toolbar.element.querySelector<HTMLButtonElement>("[data-action-id='zeta.toolbar.moreActions'] button");
  assert.ok(more);
  more.click();

  const popup = host.querySelector<HTMLElement>(".zeta-context-view .zeta-menu");
  assert.ok(popup);
  assert.equal(popup.getAttribute("role"), "menu");
  assert.deepEqual(
    [...popup.querySelectorAll<HTMLElement>("[role='menuitem'], [role='menuitemcheckbox']")]
      .map((item) => item.textContent),
    ["clear", "output"],
  );
  assert.equal(popup.querySelectorAll(".zeta-action-view-item-separator").length, 1);
  assert.equal(popup.querySelector("[role='menuitemcheckbox']")?.getAttribute("aria-checked"), "true");
  popup.querySelector<HTMLButtonElement>("[data-action-id='clear'] button")?.click();
  await Promise.resolve();
  assert.equal(cleared, 1);
  assert.equal(more.getAttribute("aria-expanded"), "false");

  dom.window.close();
  for (const name of ["window", "Node", "Element", "HTMLElement"]) {
    Reflect.deleteProperty(globalThis, name);
  }
});

function testAction(id: string, run: () => unknown = () => undefined) {
  return {
    id,
    label: id,
    tooltip: id,
    enabled: true,
    run,
  };
}
