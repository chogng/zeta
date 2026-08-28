import assert from "node:assert/strict";
import test from "node:test";
import { JSDOM } from "jsdom";
import type { IAction } from "../../../../../base/common/actions.js";
import { Event } from "../../../../../base/common/event.js";
import type { IContextMenuService } from "../../../../../platform/contextview/browser/contextView.js";
import type { ITerminalInstance } from "../../../../../workbench/services/terminal/common/terminal.js";

const browserEnvironment = new JSDOM("<!doctype html><body></body>");
for (const [name, value] of Object.entries({
	window: browserEnvironment.window,
	document: browserEnvironment.window.document,
	Node: browserEnvironment.window.Node,
	Element: browserEnvironment.window.Element,
	HTMLElement: browserEnvironment.window.HTMLElement,
	Event: browserEnvironment.window.Event,
	MouseEvent: browserEnvironment.window.MouseEvent,
	navigator: browserEnvironment.window.navigator,
})) {
	Object.defineProperty(globalThis, name, {
		configurable: true,
		value,
	});
}

const [
	{ ContextKeyService },
	{ MenuService },
	{ ServiceContainer },
	{ CommandService },
	{ TerminalTitleActions },
] = await Promise.all([
	import("../../../../../platform/contextkey/common/contextkey.js"),
	import("../../../../../platform/actions/common/menuService.js"),
	import("../../../../../platform/instantiation/common/instantiation.js"),
	import("../../../../../workbench/services/commands/common/commandService.js"),
	import("../../../../../workbench/contrib/terminal/browser/view/terminalTitleActions.js"),
]);

test.after(() => {
	browserEnvironment.window.close();
	for (const name of ["window", "document", "Node", "Element", "HTMLElement", "Event", "MouseEvent", "navigator"]) {
		Reflect.deleteProperty(globalThis, name);
	}
});

let shownProfileActions: readonly IAction[] = [];
let shownProfileAnchor: unknown;

const contextMenuService: IContextMenuService = {
	onDidShowContextMenu: Event.None,
	onDidHideContextMenu: Event.None,
	showContextMenu(options) {
		shownProfileActions = options.getActions?.() ?? [];
		shownProfileAnchor = options.getAnchor();
	},
	hideContextMenu() {},
};

test("Terminal profile menu launches the selected shell profile", async () => {
	const ownerDocument = browserEnvironment.window.document;
	ownerDocument.body.replaceChildren();
	shownProfileActions = [];
	shownProfileAnchor = undefined;
	const createdProfiles: Array<string | undefined> = [];
	let focusCount = 0;
	let clearCount = 0;
	using contextKeyService = new ContextKeyService();
	const commandService = new CommandService(new ServiceContainer());
	const menuService = new MenuService(commandService, contextKeyService);
	using titleActions = new TerminalTitleActions(ownerDocument.body, {
		menuService,
		contextMenuService,
		contextKeyService,
		createTerminal: (profileId) => {
			createdProfiles.push(profileId);
		},
		focusActive: () => focusCount++,
		relaunchActive() {},
		killActive() {},
		clearActive: () => clearCount++,
	});
	const commandPromptProfile = { profileId: "cmd", title: "Command Prompt", isDefault: true };
	const powerShellProfile = { profileId: "pwsh", title: "PowerShell", isDefault: false };
	const unavailableProfile = titleActions.element.querySelector<HTMLButtonElement>(".zeta-dropdown-with-primary-dropdown > .zeta-button");
	assert.ok(unavailableProfile);
	assert.equal(unavailableProfile.disabled, true);
	titleActions.setProfiles([commandPromptProfile, powerShellProfile]);
	ownerDocument.body.append(titleActions.element);

	const toolbar = titleActions.element;
	assert.equal(toolbar.getAttribute("role"), "toolbar");
	assert.equal(toolbar.classList.contains("highlight-toggled"), true);
	const splitNewTerminal = toolbar.querySelector<HTMLElement>(".zeta-dropdown-with-primary-action-view-item");
	const profile = splitNewTerminal?.querySelector<HTMLButtonElement>(".zeta-dropdown-with-primary-dropdown > .zeta-button");
	assert.ok(splitNewTerminal);
	assert.ok(profile);
	assert.equal(profile.disabled, false);
	assert.equal(profile.querySelector(".zeta-button-label")?.textContent, "Select Terminal Profile");
	assert.equal(profile.getAttribute("aria-label"), "Select Terminal Profile");
	assert.ok(profile.querySelector("svg.zeta-icon"));
	assert.equal(toolbar.querySelectorAll("[data-action-id='zeta.terminal.new']").length, 1);
	assert.equal(toolbar.querySelector("[data-action-id='zeta.terminal.newWithProfile']"), null);
	const newTerminal = [...toolbar.querySelectorAll("button")].find((button) => button.textContent === "New Terminal");
	assert.ok(newTerminal);
	assert.equal([...toolbar.querySelectorAll("button")].some((button) => button.textContent === "Close Panel"), false);
	assert.equal([...toolbar.querySelectorAll("button")].some((button) => button.textContent === "Maximize Panel"), false);
	const currentNewTerminal = [...toolbar.querySelectorAll("button")].find((button) => button.textContent === "New Terminal");
	assert.ok(currentNewTerminal);
	currentNewTerminal.click();
	await Promise.resolve();
	assert.deepEqual(createdProfiles, [undefined]);
	titleActions.setCreating(true);
	assert.equal([...toolbar.querySelectorAll("button")].find((button) => button.textContent === "New Terminal")?.disabled, true);
	titleActions.setCreating(false);
	const activeInstance = {
		id: "terminal-1",
		title: "Backend shell",
		profile: commandPromptProfile,
		state: "running",
	} as ITerminalInstance;
	titleActions.setActiveInstance(activeInstance, "title");
	const activeTerminal = toolbar.querySelector<HTMLButtonElement>(".zeta-terminal-active-action .zeta-action-label");
	assert.ok(activeTerminal);
	assert.equal(activeTerminal.querySelector(".zeta-action-label-text")?.textContent, "Backend shell");
	assert.ok(activeTerminal.querySelector(".zeta-action-label-icon > svg.zeta-icon"));
	assert.equal(activeTerminal.classList.contains("zeta-button"), false);
	assert.equal(activeTerminal.querySelector(".zeta-button-label"), null);
	assert.equal(activeTerminal.querySelector(".zeta-icon-label"), null);
	assert.equal(activeTerminal.getAttribute("aria-label"), "Active terminal: Backend shell (Command Prompt)");
	activeTerminal.click();
	await Promise.resolve();
	assert.equal(focusCount, 1);
	assert.equal(toolbar.textContent?.includes("Kill Terminal"), true);
	assert.equal(toolbar.textContent?.includes("Relaunch Terminal"), false);
	const killTerminal = [...toolbar.querySelectorAll("[data-action-id]")].find((item) => item.getAttribute("data-action-id") === "zeta.terminal.kill");
	const moreActions = toolbar.querySelector<HTMLElement>("[data-action-id='zeta.toolbar.moreActions']");
	assert.ok(killTerminal);
	assert.ok(moreActions);
	assert.equal(killTerminal.compareDocumentPosition(moreActions) & browserEnvironment.window.Node.DOCUMENT_POSITION_FOLLOWING, browserEnvironment.window.Node.DOCUMENT_POSITION_FOLLOWING);
	assert.equal(toolbar.querySelector("[data-action-id='workbench.action.toggleMaximizedPanel']"), null);
	moreActions.querySelector("button")?.click();
	const clearTerminal = shownProfileActions.find((action) => action.id === "zeta.terminal.clear");
	assert.ok(clearTerminal);
	assert.equal(shownProfileActions.some((action) => action.id.startsWith("zeta.compositeBar.open.")), false);
	await clearTerminal.run();
	assert.equal(clearCount, 1);
	titleActions.setActiveInstance(activeInstance, "list");
	assert.equal(toolbar.querySelector(".zeta-terminal-active-action"), null);
	assert.equal(toolbar.textContent?.includes("Kill Terminal"), true);
	titleActions.setActiveInstance({
		id: "terminal-1",
		title: "Backend shell",
		profile: commandPromptProfile,
		state: "exited",
	} as ITerminalInstance, "list");
	assert.equal(toolbar.textContent?.includes("Relaunch Terminal"), true);
	titleActions.setActiveInstance(activeInstance, "title");
	assert.doesNotThrow(() => titleActions.setActiveInstance(undefined, "title"));
	assert.equal(toolbar.querySelector(".zeta-terminal-active-action"), null);
	assert.equal(toolbar.textContent?.includes("Kill Terminal"), false);
	titleActions.setActiveInstance(activeInstance, "list");

	const currentProfile = toolbar.querySelector<HTMLButtonElement>(".zeta-dropdown-with-primary-dropdown > .zeta-button");
	assert.ok(currentProfile);
	currentProfile.click();
	assert.equal(currentProfile.getAttribute("aria-haspopup"), "menu");
	assert.equal(currentProfile.getAttribute("aria-expanded"), "true");
	assert.equal(shownProfileAnchor, currentProfile);
	assert.equal(ownerDocument.querySelector(".zeta-quick-pick"), null);
	const commandPrompt = shownProfileActions.find((action) => action.label.includes("Command Prompt"));
	const powerShell = shownProfileActions.find((action) => action.label.includes("PowerShell"));
	assert.ok(commandPrompt);
	assert.ok(powerShell);
	assert.match(commandPrompt.label, /Default/);
	assert.equal(commandPrompt.checked, true);
	assert.equal(powerShell.checked, false);
	await powerShell.run();
	await Promise.resolve();
	assert.deepEqual(createdProfiles, [undefined, "pwsh"]);
});
