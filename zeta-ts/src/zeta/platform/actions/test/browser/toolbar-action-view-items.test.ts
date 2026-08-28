import assert from "node:assert/strict";
import test from "node:test";
import { JSDOM } from "jsdom";
import { h } from "../../../../base/browser/dom.js";
import { Event } from "../../../../base/common/event.js";

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
			import("../../../../platform/actions/common/actions.js"),
			import("../../../../platform/actions/browser/menuEntryActionViewItem.js"),
		]);
	let shownOptions:
		| import("../../../../base/browser/contextmenu.js").IActionContextMenuOptions
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
	const container = h(dom.window.document, "div");
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

test("DropdownWithPrimaryActionViewItem presents one split toolbar item", async () => {
	const dom = new JSDOM("<!doctype html><body></body>");
	const [
		{ DropdownWithPrimaryActionViewItem },
		{ ToolBar },
	] = await Promise.all([
		import("../../../../platform/actions/browser/dropdownWithPrimaryActionViewItem.js"),
		import("../../../../base/browser/ui/toolbar/toolbar.js"),
	]);
	let shownOptions: import("../../../../base/browser/contextmenu.js").IActionContextMenuOptions | undefined;
	const contextMenuProvider = {
		showContextMenu(options: import("../../../../base/browser/contextmenu.js").IActionContextMenuOptions) {
			shownOptions = options;
		},
	};
	let primaryRuns = 0;
	const primaryAction = { ...testAction("new"), run: () => primaryRuns++ };
	const dropdownAction = testAction("select-profile");
	using toolbar = new ToolBar(dom.window.document.body, {
		contextMenuProvider,
		actionViewItemProvider: (item, options) => new DropdownWithPrimaryActionViewItem(item, dropdownAction, [testAction("cmd")], contextMenuProvider, options),
	});
	toolbar.setActions([primaryAction]);
	dom.window.document.body.append(toolbar.element);

	const splitItem = toolbar.element.querySelector<HTMLElement>(".zeta-dropdown-with-primary-action-view-item");
	const primaryButton = splitItem?.querySelector<HTMLButtonElement>(".zeta-dropdown-with-primary-primary > .zeta-button");
	const dropdownButton = splitItem?.querySelector<HTMLButtonElement>(".zeta-dropdown-with-primary-dropdown > .zeta-button");
	assert.ok(splitItem);
	assert.ok(primaryButton);
	assert.ok(dropdownButton);
	assert.equal(toolbar.element.querySelectorAll(":scope > .zeta-action-view-item").length, 1);
	assert.equal(primaryButton.tabIndex, 0);
	assert.equal(dropdownButton.tabIndex, -1);

	primaryButton.click();
	assert.equal(primaryRuns, 1);
	primaryButton.focus();
	primaryButton.dispatchEvent(new dom.window.KeyboardEvent("keydown", { bubbles: true, cancelable: true, key: "ArrowRight" }));
	assert.equal(dom.window.document.activeElement, dropdownButton);
	assert.equal(primaryButton.tabIndex, -1);
	assert.equal(dropdownButton.tabIndex, 0);
	dropdownButton.dispatchEvent(new dom.window.KeyboardEvent("keydown", { bubbles: true, cancelable: true, key: "ArrowLeft" }));
	assert.equal(dom.window.document.activeElement, primaryButton);

	dropdownButton.click();
	assert.equal(shownOptions?.anchor, dropdownButton);
	assert.deepEqual(shownOptions?.actions.map(({ id }) => id), ["cmd"]);
	assert.equal(dropdownButton.getAttribute("aria-expanded"), "true");
	assert.equal(splitItem.classList.contains("active"), true);
	shownOptions?.onHide?.(false);
	assert.equal(dropdownButton.getAttribute("aria-expanded"), "false");
	assert.equal(splitItem.classList.contains("active"), false);

	dom.window.close();
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
		{ ServiceContainer },
		{ CommandService },
		{ DisposableStore },
		{ WorkbenchToolBar },
	] = await Promise.all([
		import("../../../../platform/actions/common/actions.js"),
		import("../../../../platform/actions/common/menuService.js"),
		import("../../../../platform/commands/common/commands.js"),
		import("../../../../platform/contextkey/common/contextkey.js"),
		import("../../../../platform/instantiation/common/instantiation.js"),
		import("../../../../workbench/services/commands/common/commandService.js"),
		import("../../../../base/common/lifecycle.js"),
		import("../../../../platform/actions/browser/toolbar.js"),
	]);
	using registrations = new DisposableStore();
	const menuId = new MenuId("test.toolbar.workbench");
	const commandId = "test.toolbar.workbench.action";
	registrations.add(CommandsRegistry.register(commandId, () => undefined));
	registrations.add(MenusRegistry.appendMenuItem(menuId, {
		command: { id: commandId, title: "Workbench action" },
		group: "navigation",
	}));
	const commands = new CommandService(new ServiceContainer());
	const contexts = registrations.add(new ContextKeyService());
	const menus = new MenuService(commands, contexts);
	const action = menus.getMenuActions(menuId)[0]?.[1][0];
	assert.ok(action);
	const toolbar = new WorkbenchToolBar(dom.window.document.body, { showContextMenu() {} });
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
		{ ServiceContainer },
		{ CommandService },
		{ DisposableStore },
		{ MenuWorkbenchToolBar },
	] = await Promise.all([
		import("../../../../platform/actions/common/actions.js"),
		import("../../../../platform/actions/common/menuService.js"),
		import("../../../../platform/commands/common/commands.js"),
		import("../../../../platform/contextkey/common/contextkey.js"),
		import("../../../../platform/instantiation/common/instantiation.js"),
		import("../../../../workbench/services/commands/common/commandService.js"),
		import("../../../../base/common/lifecycle.js"),
		import("../../../../platform/actions/browser/toolbar.js"),
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
	const commands = new CommandService(new ServiceContainer());
	const contexts = registrations.add(new ContextKeyService());
	const menus = new MenuService(commands, contexts);
	let shownOptions:
		| import("../../../../base/browser/contextmenu.js").IActionContextMenuOptions
		| undefined;
	const toolbar = new MenuWorkbenchToolBar(
		dom.window.document.body,
		menus,
		{
			showContextMenu(options) {
				shownOptions = options;
			},
		},
		menuId,
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

test("menu toolbar projects empty state as a stable visual class", async () => {
	const dom = new JSDOM("<!doctype html><body></body>");
	Object.defineProperty(globalThis, "window", {
		configurable: true,
		value: dom.window,
	});
	const [
		{ MenuId, MenusRegistry },
		{ MenuService },
		{ CommandsRegistry },
		{ ContextKeyExpr, ContextKeyService },
		{ ServiceContainer },
		{ CommandService },
		{ DisposableStore },
		{ MenuWorkbenchToolBar },
	] = await Promise.all([
		import("../../../../platform/actions/common/actions.js"),
		import("../../../../platform/actions/common/menuService.js"),
		import("../../../../platform/commands/common/commands.js"),
		import("../../../../platform/contextkey/common/contextkey.js"),
		import("../../../../platform/instantiation/common/instantiation.js"),
		import("../../../../workbench/services/commands/common/commandService.js"),
		import("../../../../base/common/lifecycle.js"),
		import("../../../../platform/actions/browser/toolbar.js"),
	]);
	using registrations = new DisposableStore();
	const menuId = new MenuId("test.toolbar.empty-state");
	const actionId = "test.toolbar.visible-action";
	registrations.add(CommandsRegistry.register(actionId, () => undefined));
	registrations.add(MenusRegistry.appendMenuItem(menuId, {
		command: { id: actionId, title: "Visible action" },
		group: "navigation",
		when: ContextKeyExpr.has("test.toolbar.visible"),
	}));
	const contexts = registrations.add(new ContextKeyService());
	const toolbar = new MenuWorkbenchToolBar(
		dom.window.document.body,
		new MenuService(new CommandService(new ServiceContainer()), contexts),
		{ showContextMenu() {} },
		menuId,
	);
	dom.window.document.body.append(toolbar.element);

	assert.equal(toolbar.element.hidden, true);
	assert.equal(toolbar.element.classList.contains("empty"), true);
	contexts.setContext("test.toolbar.visible", true);
	assert.equal(toolbar.element.hidden, false);
	assert.equal(toolbar.element.classList.contains("empty"), false);
	contexts.setContext("test.toolbar.visible", false);
	assert.equal(toolbar.element.hidden, true);
	assert.equal(toolbar.element.classList.contains("empty"), true);

	toolbar.dispose();
	dom.window.close();
	Reflect.deleteProperty(globalThis, "window");
});

test("menu toolbar retains action slots for enablement and toggle changes", async () => {
	const dom = new JSDOM("<!doctype html><body></body>");
	Object.defineProperty(globalThis, "window", {
		configurable: true,
		value: dom.window,
	});
	const [
		{ MenuId, MenusRegistry },
		{ MenuService },
		{ CommandsRegistry },
		{ ContextKeyExpr, ContextKeyService },
		{ ServiceContainer },
		{ CommandService },
		{ DisposableStore },
		{ MenuWorkbenchToolBar },
	] = await Promise.all([
		import("../../../../platform/actions/common/actions.js"),
		import("../../../../platform/actions/common/menuService.js"),
		import("../../../../platform/commands/common/commands.js"),
		import("../../../../platform/contextkey/common/contextkey.js"),
		import("../../../../platform/instantiation/common/instantiation.js"),
		import("../../../../workbench/services/commands/common/commandService.js"),
		import("../../../../base/common/lifecycle.js"),
		import("../../../../platform/actions/browser/toolbar.js"),
	]);
	using registrations = new DisposableStore();
	const menuId = new MenuId("test.toolbar.retained-actions");
	const actionId = "test.toolbar.retained-action";
	registrations.add(CommandsRegistry.register(actionId, () => undefined));
	registrations.add(MenusRegistry.appendMenuItem(menuId, {
		command: {
			id: actionId,
			title: "Retained action",
			precondition: ContextKeyExpr.has("test.toolbar.ready"),
			toggled: ContextKeyExpr.has("test.toolbar.active"),
		},
		group: "navigation",
	}));
	const contexts = registrations.add(new ContextKeyService());
	const toolbar = new MenuWorkbenchToolBar(
		dom.window.document.body,
		new MenuService(new CommandService(new ServiceContainer()), contexts),
		{ showContextMenu() {} },
		menuId,
	);
	dom.window.document.body.append(toolbar.element);
	const slot = toolbar.element.querySelector<HTMLElement>(`[data-action-id='${actionId}']`);
	assert.ok(slot);

	contexts.setContext("test.toolbar.ready", true);
	assert.equal(toolbar.element.querySelector(`[data-action-id='${actionId}']`), slot);
	assert.equal(slot.querySelector("button")?.disabled, false);

	contexts.setContext("test.toolbar.active", true);
	assert.equal(toolbar.element.querySelector(`[data-action-id='${actionId}']`), slot);
	assert.equal(slot.querySelector("button")?.classList.contains("checked"), true);

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
	Object.defineProperty(dom.window.HTMLElement.prototype, "getClientRects", {
		configurable: true,
		value: () => [{ width: 24, height: 24 }],
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
		{ ToolBar },
		{ ContextKeyService },
		{ MenuService },
		{ ServiceContainer },
		{ CommandService },
		{ BrowserContextViewService },
		{ BrowserContextMenuService },
	] = await Promise.all([
		import("../../../../base/common/actions.js"),
		import("../../../../base/browser/ui/toolbar/toolbar.js"),
		import("../../../../platform/contextkey/common/contextkey.js"),
		import("../../../../platform/actions/common/menuService.js"),
		import("../../../../platform/instantiation/common/instantiation.js"),
		import("../../../../workbench/services/commands/common/commandService.js"),
		import("../../../../platform/contextview/browser/contextViewService.js"),
		import("../../../../platform/contextview/browser/contextMenuService.js"),
	]);
	const host = dom.window.document.querySelector<HTMLElement>("main");
	assert.ok(host);
	using contexts = new ContextKeyService();
	const commands = new CommandService(new ServiceContainer());
	const menus = new MenuService(commands, contexts);
	using contextViews = new BrowserContextViewService(host);
	using contextMenus = new BrowserContextMenuService(
		menus,
		{
			inChordMode: false,
			onDidUpdateKeybindings: Event.None,
			resolveKeybinding() { throw new Error("Not used"); },
			resolveUserBinding() { return undefined; },
			lookupKeybindings() { return []; },
			lookupKeybinding() { return undefined; },
		},
		contextViews,
	);
	const openingOrder: string[] = [];
	contextMenus.onDidShowContextMenu(() => openingOrder.push("show"));
	host.addEventListener("focusin", (event) => {
		if ((event.target as HTMLElement).closest(".zeta-menu")) openingOrder.push("focus");
	});
	let cleared = 0;
	using toolbar = new ToolBar(dom.window.document.body, {
		contextMenuProvider: contextMenus,
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
	assert.equal(popup.parentElement?.classList.contains("zeta-context-view-menu"), true);
	assert.deepEqual(openingOrder, ["show", "focus"]);
	assert.equal(popup.getAttribute("role"), "menu");
	assert.deepEqual(
		[...popup.querySelectorAll<HTMLElement>("[role='menuitem'], [role='menuitemcheckbox']")]
			.map((item) => item.textContent),
		["clear", "output"],
	);
	assert.equal(popup.querySelectorAll(".zeta-action-view-item-separator").length, 1);
	const popupItems = popup.querySelectorAll<HTMLElement>("[role='menuitem'], [role='menuitemcheckbox']");
	assert.equal(popupItems.length, 2);
	assert.equal([...popupItems].every((item) => item.querySelector(":scope > .zeta-menu-leading-slot") !== null), true);
	const checkedPopupItem = popup.querySelector<HTMLElement>("[role='menuitemcheckbox']");
	assert.equal(checkedPopupItem?.getAttribute("aria-checked"), "true");
	assert.equal(checkedPopupItem?.querySelector(":scope > .zeta-menu-leading-check") !== null, true);
	assert.equal(checkedPopupItem?.querySelector(":scope > .zeta-menu-leading-check > .zeta-icon") !== null, true);
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
