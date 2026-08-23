import assert from "node:assert/strict";
import test from "node:test";
import { JSDOM } from "jsdom";
import type { CodeIndexConfigurationSnapshot, SemanticCodeIndexSelection } from "../../../../../platform/codeIndex/common/codeIndexService.js";
import { h } from "../../../../../base/browser/dom.js";
import type { IAction } from "../../../../../base/common/actions.js";

const browserEnvironment = new JSDOM("<!doctype html><body></body>", {
	pretendToBeVisual: true,
});
Object.defineProperty(browserEnvironment.window.Element.prototype, "scrollTo", {
	configurable: true,
	value() {},
});
Object.defineProperty(browserEnvironment.window.Element.prototype, "scrollIntoView", {
	configurable: true,
	value() {},
});
for (const [name, value] of Object.entries({
	window: browserEnvironment.window,
	document: browserEnvironment.window.document,
	Node: browserEnvironment.window.Node,
	Element: browserEnvironment.window.Element,
	HTMLElement: browserEnvironment.window.HTMLElement,
	Event: browserEnvironment.window.Event,
	MouseEvent: browserEnvironment.window.MouseEvent,
	KeyboardEvent: browserEnvironment.window.KeyboardEvent,
	navigator: browserEnvironment.window.navigator,
})) {
	Object.defineProperty(globalThis, name, {
		configurable: true,
		value,
	});
}

const { DisposableStore, toDisposable } = await import("../../../../../base/common/lifecycle.js");
const { Emitter } = await import("../../../../../base/common/event.js");
const { KeybindingLabelStyle, getKeybindingLabel } = await import("../../../../../base/common/keybindingLabels.js");
const { resolveKeybinding } = await import("../../../../../base/common/keybindings.js");
const { lxiconsLibrary } = await import("../../../../../base/common/lxiconsLibrary.js");
const { OperatingSystem } = await import("../../../../../base/common/platform.js");
const { URI } = await import("../../../../../base/common/uri.js");
const { MenuId } = await import("../../../../../platform/actions/common/actions.js");
const { HoverConfiguration } = await import("../../../../../platform/hover/common/hoverService.js");
const { MenuService } = await import("../../../../../platform/actions/common/menuService.js");
const { ContextKeyService } = await import("../../../../../platform/contextkey/common/contextkey.js");
const { BrowserContextViewService } = await import("../../../../../platform/contextview/browser/contextViewService.js");
const { ServiceCollection } = await import("../../../../../platform/instantiation/common/instantiation.js");
const { KeybindingResolver } = await import("../../../../../platform/keybinding/common/keybindingResolver.js");
const { KeybindingsRegistry } = await import("../../../../../platform/keybinding/common/keybindingsRegistry.js");
const { createColorTheme, darkColorTheme, lightColorTheme } = await import("../../../../../platform/theme/common/colorTheme.js");
const { ThemeService } = await import("../../../../../platform/theme/common/themeService.js");
const { parseUserColorTheme } = await import("../../../../../platform/theme/common/userColorTheme.js");
const { ColorScheme } = await import("../../../../../platform/theme/common/theme.js");
const { ISettingsService } = await import("../../../../../workbench/services/preferences/common/settings.js");
const { SettingsService } = await import("../../../../../workbench/services/preferences/common/settingsService.js");
const { IPreferencesService } = await import("../../../../../workbench/services/preferences/common/preferences.js");
const { PreferencesService } = await import("../../../../../workbench/services/preferences/common/preferencesService.js");
const { WorkbenchConfiguration } = await import("../../../../../workbench/common/configuration.js");
const { CodeEditorConfiguration } = await import("../../../../../workbench/contrib/codeEditor/common/editorConfiguration.js");
const { EditorIndentationKind } = await import("../../../../../editor/common/editorIndentation.js");
const { EditorLineWrapping } = await import("../../../../../editor/browser/viewModel/visualLineProjection.js");
const { WorkbenchThemesRegistry } = await import("../../../../../workbench/common/theme.js");
const { UnavailableUserThemeService } = await import("../../../../../workbench/common/userThemes.js");
const { WorkbenchConfigurationService } = await import("../../../../../workbench/services/configuration/browser/configurationService.js");
const { WorkspaceContextService } = await import("../../../../../workbench/services/workspaces/browser/workspaceContextService.js");
const { SettingsEditorContribution } = await import("../../../../../workbench/contrib/preferences/browser/settingsEditor.contribution.js");
const { SettingsLayout } = await import("../../../../../workbench/contrib/preferences/browser/settingsLayout.js");
const { SettingsTree } = await import("../../../../../workbench/contrib/preferences/browser/settingsTree.js");
const { SettingsTreeModel } = await import("../../../../../workbench/contrib/preferences/browser/settingsTreeModels.js");
const { CommandService } = await import("../../../../../workbench/services/commands/common/commandService.js");
const { BrowserKeyboardLayoutService } = await import("../../../../../workbench/services/keybinding/browser/keyboardLayoutService.js");
const { WorkbenchKeybindingService } = await import("../../../../../workbench/services/keybinding/browser/keybindingService.js");
await import("../../../../../workbench/contrib/preferences/browser/preferences.contribution.js");
const { OpenSettingsCommandId } = await import("../../../../../workbench/contrib/preferences/common/preferences.js");

const acceptingDialogService = {
	showMessage: async () => {},
	confirm: async () => true,
};

function settingValue(root: HTMLElement, key: string): string | undefined {
	return root.querySelector<HTMLElement>(`[data-configuration-key="${key}"] .zeta-dropdown-label`)?.textContent ?? undefined;
}

function chooseSettingOption(root: HTMLElement, key: string, label: string): void {
	const control = root.querySelector<HTMLElement>(`[data-configuration-key="${key}"]`);
	assert.ok(control);
	const button = control.querySelector<HTMLButtonElement>(".zeta-select-box-button");
	assert.ok(button);
	button.click();
	const option = [...browserEnvironment.window.document.querySelectorAll<HTMLElement>(".zeta-select-box-option")]
		.find(element => element.querySelector(".zeta-select-box-option-label")?.textContent === label);
	assert.ok(option);
	option.click();
}

test("Settings tree renders validated group and item identities", () => {
	using disposables = new DisposableStore();
	const ownerDocument = browserEnvironment.window.document;
	const model = disposables.add(new SettingsTreeModel<string>());
	model.setChildren([{
		element: { kind: "group", id: "appearance.colors", title: "Colors", description: "Choose the active color scheme." },
		children: [
			{ element: { kind: "item", id: "appearance.colors.theme", title: "Theme", description: "Choose a theme.", value: "Theme" } },
			{ element: { kind: "item", id: "appearance.colors.font", title: "Font family", description: "Choose a UI font.", value: "Font" } },
		],
	}]);
	const disposedItems: string[] = [];
	const renderer = disposables.add(new SettingsTree(ownerDocument.body, {
		model,
		rootClassName: "test-settings-tree",
		groupClassName: "test-settings-group",
		groupDescriptionClassName: "test-settings-group-description",
		itemsClassName: "test-settings-items",
		renderItem: (item) => {
			const element = h(ownerDocument, "article");
			element.textContent = item.value;
			return element;
		},
		updateItem: (item, element) => { element.textContent = item.value; },
		disposeItem: (item) => disposedItems.push(item.id),
	}));

	assert.equal(renderer.element.classList.contains("zeta-settings-tree"), true);
	assert.equal(renderer.element.querySelector<HTMLElement>("[data-settings-tree-group-id]")?.dataset.settingsTreeGroupId, "appearance.colors");
	assert.equal(renderer.element.querySelector<HTMLElement>("[data-settings-tree-item-id]")?.dataset.settingsTreeItemId, "appearance.colors.theme");
	assert.equal(renderer.element.querySelector<HTMLElement>("[data-settings-item-id]")?.dataset.settingsItemId, "appearance.colors.theme");
	assert.equal(renderer.element.querySelector<HTMLElement>("[data-settings-item-kind]")?.dataset.settingsItemKind, "information");
	assert.equal(renderer.element.querySelector("article")?.textContent, "Theme");
	assert.equal(renderer.element.querySelectorAll("article").length, 2);
	assert.equal(model.getNode("appearance.colors.theme")?.id, "appearance.colors.theme");
	assert.equal(model.getParent("appearance.colors.theme")?.id, "appearance.colors");
	assert.equal(model.getNode("appearance.colors")?.collapsible, false);
	const themeElement = renderer.getItemElement("appearance.colors.theme");
	themeElement!.tabIndex = 0;
	themeElement!.focus();
	model.setQuery("font family");
	assert.deepEqual(model.visibleItems.map((item) => item.id), ["appearance.colors.font"]);
	assert.equal(model.countVisibleItems("appearance.colors"), 1);
	assert.equal(renderer.element.querySelector("article")?.textContent, "Font");
	model.setQuery("theme");
	assert.equal(renderer.getItemElement("appearance.colors.theme"), themeElement);
	themeElement!.focus();
	model.setQuery("");
	assert.equal(renderer.element.querySelectorAll("article").length, 2);
	assert.equal(ownerDocument.activeElement, themeElement);
	model.setNodeChildren("appearance.colors", [{
		element: { kind: "item", id: "appearance.colors.theme", title: "Theme", description: "Choose a theme.", value: "Updated Theme" },
	}]);
	assert.equal(renderer.getItemElement("appearance.colors.theme"), themeElement);
	assert.equal(themeElement?.textContent, "Updated Theme");
	assert.equal(ownerDocument.activeElement, themeElement);
	assert.deepEqual(disposedItems, ["appearance.colors.font"]);
	assert.throws(() => model.setChildren([
		{ element: { kind: "item", id: "duplicate", title: "One", description: "", value: "one" } },
		{ element: { kind: "item", id: "duplicate", title: "Two", description: "", value: "two" } },
	]), /Duplicate tree node ID/);
	assert.throws(() => model.setChildren([{
		element: { kind: "group", id: "empty-title", title: "", description: "" },
	}]), /must have a title/);
	assert.throws(() => model.setChildren([{
		element: { kind: "item", id: "parent-item", title: "Parent item", description: "", value: "parent" },
		children: [{ element: { kind: "item", id: "child-item", title: "Child item", description: "", value: "child" } }],
	}]), /must not have children/);
});

test('Settings layout projects stable groups and searchable item IDs', () => {
	const layout = new SettingsLayout('editor', [{
		id: 'display',
		title: 'Display',
		description: 'Editor presentation.',
		settings: [
			{ id: 'editor.minimap.enabled', title: 'Minimap', description: 'Show the minimap.' },
			{ id: 'editor.lineNumbers', title: 'Line numbers', description: 'Show line numbers.', keywords: ['gutter'] },
		],
	}]);

	assert.equal(layout.nodes[0]?.element.id, 'editor.group.display');
	assert.deepEqual(layout.nodes[0]?.children?.map(node => node.element.id), [
		'editor.minimap.enabled',
		'editor.lineNumbers',
	]);
	assert.deepEqual(layout.nodes[0]?.children?.[1]?.element.keywords, ['editor.lineNumbers', 'gutter']);
	assert.throws(() => new SettingsLayout('editor', [{
		id: 'display',
		title: 'Display',
		description: '',
		settings: [
			{ id: 'editor.minimap.enabled', title: 'Minimap', description: '' },
			{ id: 'editor.minimap.enabled', title: 'Duplicate', description: '' },
		],
	}]), /Duplicate Settings item ID/);
	assert.throws(() => new SettingsLayout('', []), /section ID must not be empty/);
});

test("Settings overlay opens, closes, and restores focus", () => {
	using disposables = new DisposableStore();
	const ownerDocument = browserEnvironment.window.document;
	ownerDocument.body.replaceChildren();
	const root = h(ownerDocument, "div");
	const trigger = h(ownerDocument, "button");
	trigger.textContent = "Open";
	root.append(trigger);
	ownerDocument.body.append(root);
	trigger.focus();

	const settings = disposables.add(new SettingsService());
	const configuration = disposables.add(new WorkbenchConfigurationService());
	disposables.add(new SettingsEditorContribution({
		configurationService: configuration,
		container: root,
		contextViewProvider: disposables.add(new BrowserContextViewService(root)),
		dialogService: acceptingDialogService,
		settingsService: settings,
		themeService: disposables.add(new ThemeService(darkColorTheme)),
		userThemeService: UnavailableUserThemeService,
	}));
	settings.open();

	const host = root.querySelector<HTMLElement>(".zeta-modal-editor-host");
	const surface = root.querySelector<HTMLElement>(".zeta-modal-editor");
	assert.ok(host);
	assert.ok(surface);
	assert.equal(settings.isOpen, true);
	assert.equal(host.hidden, false);
	assert.equal(surface.getAttribute("role"), "dialog");
	assert.equal(surface.getAttribute("aria-modal"), "true");
	assert.ok(surface.querySelector(":scope > .zeta-modal-editor-header"));
	assert.ok(surface.querySelector(":scope > .zeta-modal-editor-content > .zeta-settings-editor"));
	assert.ok(root.querySelector("[data-settings-container]"));
	assert.equal(root.querySelectorAll(".zeta-settings-layout > * > .zeta-scrollable-element").length, 2);
	const search = root.querySelector<HTMLInputElement>(".zeta-settings-search input");
	assert.ok(search);
	assert.equal(search.type, "search");
	assert.equal(search.placeholder, "Search settings");
	assert.equal(ownerDocument.activeElement, search);
	const navigationItems = [...root.querySelectorAll<HTMLButtonElement>("[data-settings-section-id]")];
	assert.deepEqual(
		navigationItems.map((item) => item.textContent),
		[
			"General",
			"Chat",
			"User",
			"Workspace Trust",
			"Appearance",
			"Editor",
			"Languages",
			"Display Language",
			"Agents",
			"Models",
			"Git",
			"Worktrees",
			"Marketplace",
			"Plugins",
			"Connectors",
			"Rules",
			"Skills & Subagents",
			"Tools & MCPs",
			"Hooks",
			"Browser",
			"Tabs",
			"Indexing",
			"Experimental",
			"Documentation",
		],
	);
	assert.equal(navigationItems[0].getAttribute("aria-current"), "page");
	assert.equal(root.querySelector(".zeta-settings-page h3")?.textContent, "General");
	assert.equal(root.querySelectorAll(".zeta-general-setting").length, 10);

	search.value = "model";
	search.dispatchEvent(new browserEnvironment.window.Event("input", { bubbles: true }));
	assert.deepEqual(
		navigationItems.filter((item) => !item.parentElement?.hidden).map((item) => item.textContent),
		["Models", "Tools & MCPs"],
	);
	search.dispatchEvent(new browserEnvironment.window.KeyboardEvent("keydown", {
		bubbles: true,
		cancelable: true,
		key: "Escape",
	}));
	assert.equal(settings.isOpen, true);
	assert.equal(search.value, "");

	navigationItems[9].click();
	assert.equal(settings.activeSectionId, "models");
	assert.equal(navigationItems[0].hasAttribute("aria-current"), false);
	assert.equal(navigationItems[9].getAttribute("aria-current"), "page");
	assert.equal(root.querySelector(".zeta-settings-page h3")?.textContent, "Models");
	assert.equal(
		root.querySelector(".zeta-settings-page")?.getAttribute("data-active-settings-section"),
		"models",
	);
	assert.ok(root.querySelector(".zeta-model-settings"));

	surface.dispatchEvent(new browserEnvironment.window.KeyboardEvent(
		"keydown",
		{ bubbles: true, key: "Escape" },
	));
	assert.equal(settings.isOpen, false);
	assert.equal(host.hidden, true);
	assert.equal(ownerDocument.activeElement, trigger);
});

test("Workspace Trust settings add, list, and revoke durable folder decisions", async () => {
	using disposables = new DisposableStore();
	const ownerDocument = browserEnvironment.window.document;
	ownerDocument.body.replaceChildren();
	const root = h(ownerDocument, "div");
	ownerDocument.body.append(root);
	const settings = disposables.add(new SettingsService());
	const configuration = disposables.add(new WorkbenchConfigurationService());
	let revision = 7;
	let entries = [
		{ workspace: "sha256:trusted", root: "/workspaces/trusted" },
	];
	const mutations: string[] = [];
	const workspaceTrustService = {
		list: async () => ({ revision, entries }),
		read: async () => "restricted" as const,
		set: async (folder: string, setting: "restricted" | "trusted", expectedRevision: number) => {
			mutations.push(`set:${folder}:${setting}:${expectedRevision}`);
			revision += 1;
			if (setting === "trusted") entries = [...entries, { workspace: "sha256:new", root: folder }];
			return { revision, generation: revision, disposition: "updated" as const };
		},
		forget: async (workspace: string, expectedRevision: number) => {
			mutations.push(`forget:${workspace}:${expectedRevision}`);
			revision += 1;
			entries = entries.filter(entry => entry.workspace !== workspace);
			return { revision, generation: revision, disposition: "updated" as const };
		},
	};
	const workspaceOpenService = {
		canOpenFolder: true,
		canOpenWorkspace: true,
		openFolder: async () => {},
		openWorkspace: async () => {},
		pickFolder: async () => "/workspaces/new",
	};
	disposables.add(new SettingsEditorContribution({
		configurationService: configuration,
		container: root,
		contextViewProvider: disposables.add(new BrowserContextViewService(root)),
		dialogService: acceptingDialogService,
		settingsService: settings,
		themeService: disposables.add(new ThemeService(darkColorTheme)),
		userThemeService: UnavailableUserThemeService,
		workspaceTrustService,
		workspaceOpenService,
	}));

	settings.open("workspace-trust");
	await new Promise(resolve => globalThis.setTimeout(resolve, 0));
	assert.deepEqual(
		[...root.querySelectorAll<HTMLElement>('[data-workspace-trust-list="trusted"] .zeta-workspace-trust-entry h5')].map(element => element.textContent),
		["/workspaces/trusted"],
	);
	assert.ok(root.querySelector('[data-settings-item-id="workspaceTrust.entries.sha256:trusted"][data-settings-item-kind="resource"]'));
	assert.equal(root.querySelector<HTMLButtonElement>(".zeta-workspace-trust-toolbar button")?.textContent, "Add Folder…");
	root.querySelector<HTMLButtonElement>(".zeta-workspace-trust-toolbar button")?.click();
	await new Promise(resolve => globalThis.setTimeout(resolve, 0));
	await new Promise(resolve => globalThis.setTimeout(resolve, 0));
	assert.deepEqual(mutations, ["set:/workspaces/new:trusted:7"]);

	const trustedEntry = [...root.querySelectorAll<HTMLElement>(".zeta-workspace-trust-entry")]
		.find(element => element.querySelector("h5")?.textContent === "/workspaces/trusted");
	assert.ok(trustedEntry);
	trustedEntry.querySelector<HTMLButtonElement>("button")?.click();
	await new Promise(resolve => globalThis.setTimeout(resolve, 0));
	await new Promise(resolve => globalThis.setTimeout(resolve, 0));
	assert.deepEqual(mutations, [
		"set:/workspaces/new:trusted:7",
		"forget:sha256:trusted:8",
	]);
	assert.equal(root.querySelector("h5")?.textContent, "/workspaces/new");
});

test("Workspace Trust settings does not render restricted legacy decisions in the trusted-folder list", async () => {
	using disposables = new DisposableStore();
	const ownerDocument = browserEnvironment.window.document;
	ownerDocument.body.replaceChildren();
	const root = h(ownerDocument, "div");
	ownerDocument.body.append(root);
	const settings = disposables.add(new SettingsService());
	const configuration = disposables.add(new WorkbenchConfigurationService());
	const workspaceTrustService = {
		list: async () => ({
			revision: 1,
			entries: [],
		}),
		read: async () => "restricted" as const,
		set: async () => ({ revision: 2, generation: 2, disposition: "updated" as const }),
		forget: async () => ({ revision: 2, generation: 2, disposition: "updated" as const }),
	};
	disposables.add(new SettingsEditorContribution({
		configurationService: configuration,
		container: root,
		contextViewProvider: disposables.add(new BrowserContextViewService(root)),
		dialogService: acceptingDialogService,
		settingsService: settings,
		themeService: disposables.add(new ThemeService(darkColorTheme)),
		userThemeService: UnavailableUserThemeService,
		workspaceTrustService,
		workspaceOpenService: {
			canOpenFolder: true,
			canOpenWorkspace: true,
			openFolder: async () => {},
			openWorkspace: async () => {},
			pickFolder: async () => undefined,
		},
	}));

	settings.open("workspace-trust");
	await new Promise(resolve => globalThis.setTimeout(resolve, 0));
	assert.equal(root.querySelector('[data-workspace-trust-list="trusted"] .zeta-workspace-trust-empty')?.textContent, "You haven't trusted any folders or workspace files yet. Use Add Folder… to trust a folder.");
	assert.equal(root.querySelector(".zeta-workspace-trust-entry"), null);
});

test("Workspace Trust settings exposes and updates the current Restricted workspace", async () => {
	using disposables = new DisposableStore();
	using workspaceContext = new WorkspaceContextService({ id: "window", uri: URI.file("/workspaces/current") });
	const ownerDocument = browserEnvironment.window.document;
	ownerDocument.body.replaceChildren();
	const root = h(ownerDocument, "div");
	ownerDocument.body.append(root);
	const settings = disposables.add(new SettingsService());
	const configuration = disposables.add(new WorkbenchConfigurationService());
	let revision = 1;
	let state: "restricted" | "trusted" = "restricted";
	const workspaceTrustService = {
		list: async () => ({ revision, entries: state === "trusted" ? [{ workspace: "sha256:current", root: "/workspaces/current" }] : [] }),
		read: async (folder: string) => {
			assert.equal(folder, "/workspaces/current");
			return state;
		},
		set: async (folder: string, nextState: "restricted" | "trusted", expectedRevision: number) => {
			assert.equal(folder, "/workspaces/current");
			assert.equal(expectedRevision, revision);
			state = nextState;
			revision += 1;
			return { revision, generation: revision, disposition: "updated" as const };
		},
		forget: async () => ({ revision, generation: revision, disposition: "updated" as const }),
	};
	disposables.add(new SettingsEditorContribution({
		configurationService: configuration,
		container: root,
		contextViewProvider: disposables.add(new BrowserContextViewService(root)),
		dialogService: acceptingDialogService,
		settingsService: settings,
		themeService: disposables.add(new ThemeService(darkColorTheme)),
		userThemeService: UnavailableUserThemeService,
		workspaceTrustService,
		workspaceContextService: workspaceContext,
		workspaceOpenService: {
			canOpenFolder: true,
			canOpenWorkspace: true,
			openFolder: async () => {},
			openWorkspace: async () => {},
			pickFolder: async () => undefined,
		},
	}));

	settings.open("workspace-trust");
	await new Promise(resolve => globalThis.setTimeout(resolve, 0));
	assert.equal(root.querySelector(".zeta-workspace-trust-current .zeta-workspace-trust-status")?.textContent, "Restricted");
	assert.ok(root.querySelector('[data-settings-item-id="workspaceTrust.current"][data-settings-item-kind="resource"]'));
	const trustCurrent = root.querySelector<HTMLButtonElement>(".zeta-workspace-trust-current-actions button");
	assert.equal(trustCurrent?.textContent, "Trust This Folder");
	trustCurrent?.click();
	await new Promise(resolve => globalThis.setTimeout(resolve, 0));
	await new Promise(resolve => globalThis.setTimeout(resolve, 0));
	assert.equal(state, "trusted");
	assert.equal(root.querySelector(".zeta-workspace-trust-current .zeta-workspace-trust-status")?.textContent, "Trusted");
});

test("General settings persist shared interaction preferences", async () => {
	using disposables = new DisposableStore();
	const ownerDocument = browserEnvironment.window.document;
	ownerDocument.body.replaceChildren();
	const root = h(ownerDocument, "div");
	ownerDocument.body.append(root);
	const settings = disposables.add(new SettingsService());
	const configuration = disposables.add(new WorkbenchConfigurationService());
	const modeSwitches: string[] = [];
	const copiedSettingIds: string[] = [];
	let keyboardShortcutsOpens = 0;
	let settingActions: readonly IAction[] = [];
	let hideSettingActions = (): void => {};
	disposables.add(new SettingsEditorContribution({
		clipboardService: { writeText: async value => { copiedSettingIds.push(value); } },
		configurationService: configuration,
		container: root,
		contextMenuProvider: {
			showContextMenu: options => {
				if (!("actions" in options)) throw new Error("Expected direct setting actions");
				settingActions = options.actions;
				hideSettingActions = () => options.onHide?.(false);
			},
		},
		contextViewProvider: disposables.add(new BrowserContextViewService(root)),
		dialogService: acceptingDialogService,
		settingsService: settings,
		preferencesService: {
			openSettings: sectionId => settings.open(sectionId),
			openKeybindings: async () => { keyboardShortcutsOpens += 1; },
		},
		themeService: disposables.add(new ThemeService(darkColorTheme)),
		userThemeService: UnavailableUserThemeService,
		workbenchModeService: {
			currentModeId: "code",
			availableModes: [
				{ id: "code", label: "Code" },
				{ id: "academic", label: "Academic" },
			],
			switchMode: async modeId => { modeSwitches.push(modeId); },
			resetMode: async () => { modeSwitches.push("reset"); },
		},
	}));

	settings.open("general");
	const hoverDelay = root.querySelector<HTMLInputElement>('[data-configuration-key="workbench.hover.delay"]')!;
	assert.equal(root.querySelector('[data-configuration-key="workbench.defaultProfile"]'), null);
	assert.equal(settingValue(root, "workbench.mode"), "Code");
	assert.equal(settingValue(root, "workbench.reduceMotion"), "Auto");
	assert.equal(hoverDelay.value, "500");
	chooseSettingOption(root, "workbench.reduceMotion", "On");
	await new Promise(resolve => globalThis.setTimeout(resolve, 0));
	hoverDelay.value = "250";
	hoverDelay.dispatchEvent(new browserEnvironment.window.Event("change", { bubbles: true }));
	await new Promise(resolve => globalThis.setTimeout(resolve, 0));
	chooseSettingOption(root, "workbench.mode", "Academic");
	await new Promise(resolve => globalThis.setTimeout(resolve, 0));

	assert.equal(configuration.getValue(WorkbenchConfiguration.reduceMotion), "on");
	assert.equal(configuration.getValue(HoverConfiguration.delay), 250);
	assert.deepEqual(modeSwitches, ["academic"]);
	const keyboardShortcutsButton = root.querySelector<HTMLButtonElement>('[data-setting-action-id="workbench.keyboardShortcuts"]');
	assert.ok(keyboardShortcutsButton);
	assert.equal(keyboardShortcutsButton.textContent, 'Open Keyboard Shortcuts');
	keyboardShortcutsButton.click();
	await Promise.resolve();
	assert.equal(keyboardShortcutsOpens, 1);
	assert.equal(root.querySelectorAll(".zeta-general-setting > .zeta-setting-item-actions").length, 9);
	const hoverDelayRow = hoverDelay.closest<HTMLElement>(".zeta-general-setting");
	assert.ok(hoverDelayRow);
	assert.equal(hoverDelayRow.dataset.settingsItemId, "workbench.hover.delay");
	assert.equal(hoverDelayRow.dataset.settingsItemKind, "setting");
	const hoverDelayActions = hoverDelayRow.querySelector<HTMLButtonElement>(".zeta-setting-item-actions-trigger");
	assert.equal(hoverDelayActions?.getAttribute("aria-label"), "More actions for Hover delay");
	hoverDelayActions?.click();
	assert.deepEqual(settingActions.map(action => ({ label: action.label, enabled: action.enabled })), [
		{ label: "Reset Setting", enabled: true },
		{ label: "Copy Setting ID", enabled: true },
	]);
	settingActions[1]?.run();
	await Promise.resolve();
	assert.deepEqual(copiedSettingIds, ["workbench.hover.delay"]);
	hideSettingActions();
	settingActions[0]?.run();
	await new Promise(resolve => globalThis.setTimeout(resolve, 0));
	assert.equal(configuration.getValue(HoverConfiguration.delay), HoverConfiguration.delay.defaultValue);
});

test("Settings domains without writable services render honest capability overviews", () => {
	using disposables = new DisposableStore();
	const ownerDocument = browserEnvironment.window.document;
	ownerDocument.body.replaceChildren();
	const root = h(ownerDocument, "div");
	ownerDocument.body.append(root);
	const settings = disposables.add(new SettingsService());
	disposables.add(new SettingsEditorContribution({
		configurationService: disposables.add(new WorkbenchConfigurationService()),
		container: root,
		contextViewProvider: disposables.add(new BrowserContextViewService(root)),
		dialogService: acceptingDialogService,
		settingsService: settings,
		themeService: disposables.add(new ThemeService(darkColorTheme)),
		userThemeService: UnavailableUserThemeService,
	}));
	const overviewSections = ["chat", "user", "agents", "git", "worktrees", "rules", "skills-and-subagents", "tools-and-mcps", "hooks", "browser", "tabs", "experimental", "documentation"];
	for (const sectionId of overviewSections) {
		settings.open(sectionId);
		assert.ok(root.querySelectorAll(".zeta-settings-overview-item").length > 0, sectionId);
		assert.equal(root.querySelectorAll(".zeta-settings-overview-item:not([data-settings-item-id])").length, 0, sectionId);
		assert.equal(root.querySelectorAll('.zeta-settings-overview-item[data-settings-item-kind="information"]').length, root.querySelectorAll(".zeta-settings-overview-item").length, sectionId);
	}
	settings.open("chat");
	assert.ok(root.querySelector('[data-settings-item-id="chat.group.conversations.item.untitled"]'));
	[...root.querySelectorAll<HTMLButtonElement>(".zeta-settings-overview-action")].find(button => button.textContent === "Open Models")?.click();
	assert.equal(settings.activeSectionId, "models");
});

test("Models settings show catalog metadata and persist picker visibility", async () => {
	using disposables = new DisposableStore();
	const ownerDocument = browserEnvironment.window.document;
	ownerDocument.body.replaceChildren();
	const root = h(ownerDocument, "div");
	ownerDocument.body.append(root);
	const settings = disposables.add(new SettingsService());
	const changes = disposables.add(new Emitter<void>());
	const hidden = new Set<string>();
	const copiedSettingIds: string[] = [];
	let settingActions: readonly IAction[] = [];
	let hideSettingActions = (): void => {};
	let refreshes = 0;
	let catalog = [
		{
			model: { provider: "openai", model: "gpt-5.6-sol" },
			displayName: "GPT-5.6 Sol",
			access: "subscription" as const,
			outputTransport: "nativeStreaming" as const,
		},
		{
			model: { provider: "anthropic", model: "claude-opus-5" },
			displayName: "Claude Opus 5",
			access: "apiKey" as const,
			outputTransport: "nativeStreaming" as const,
		},
		{
			model: { provider: "openai", model: "gpt-5.6" },
			displayName: "GPT-5.6",
			access: "apiKey" as const,
			outputTransport: "nativeStreaming" as const,
		},
	];
	disposables.add(new SettingsEditorContribution({
		clipboardService: { writeText: async value => { copiedSettingIds.push(value); } },
		configurationService: disposables.add(new WorkbenchConfigurationService()),
		container: root,
		contextMenuProvider: {
			showContextMenu: options => {
				if (!("actions" in options)) throw new Error("Expected direct setting actions");
				settingActions = options.actions;
				hideSettingActions = () => options.onHide?.(false);
			},
		},
		contextViewProvider: disposables.add(new BrowserContextViewService(root)),
		dialogService: acceptingDialogService,
		settingsService: settings,
		themeService: disposables.add(new ThemeService(darkColorTheme)),
		userThemeService: UnavailableUserThemeService,
		modelCatalog: {
			onDidChangeModels: changes.event,
			listModelCatalog: async () => catalog,
			refreshModels: async () => { refreshes += 1; return catalog; },
			isModelVisible: model => !hidden.has(`${model.provider}/${model.model}`),
			setModelVisible: async (model, visible) => {
				const identity = `${model.provider}/${model.model}`;
				if (visible) hidden.delete(identity);
				else hidden.add(identity);
				changes.fire();
			},
		},
	}));

	settings.open("models");
	await new Promise(resolve => globalThis.setTimeout(resolve, 0));
	assert.deepEqual([...root.querySelectorAll(".zeta-model-settings-copy h4")].map(element => element.textContent), ["GPT-5.6 Sol", "Claude Opus 5", "GPT-5.6"]);
	assert.equal(root.querySelector(".zeta-model-settings-access-badge")?.textContent, "ChatGPT subscription");
	assert.equal(root.querySelector(".zeta-model-settings-status")?.textContent, "3 models");
	assert.equal(root.querySelectorAll(".zeta-model-settings-row > .zeta-setting-item-actions").length, 3);
	assert.equal(root.querySelectorAll('.zeta-model-settings-row[data-settings-item-kind="setting"]').length, 3);
	root.querySelector<HTMLButtonElement>('.zeta-model-settings-row[data-settings-item-id="openai/gpt-5.6-sol"] .zeta-setting-item-actions-trigger')?.click();
	assert.deepEqual(settingActions.map(action => ({ label: action.label, enabled: action.enabled })), [
		{ label: "Reset Setting", enabled: false },
		{ label: "Copy Setting ID", enabled: true },
	]);
	settingActions[1]?.run();
	await Promise.resolve();
	assert.deepEqual(copiedSettingIds, ["openai/gpt-5.6-sol"]);
	hideSettingActions();
	const firstRow = root.querySelector<HTMLElement>('.zeta-model-settings-row[data-settings-item-id="openai/gpt-5.6-sol"]')!;
	const firstToggle = firstRow.querySelector<HTMLInputElement>('input[role="switch"]')!;
	firstToggle.click();
	await new Promise(resolve => globalThis.setTimeout(resolve, 0));
	assert.deepEqual([...hidden], ["openai/gpt-5.6-sol"]);
	assert.equal(root.querySelector('.zeta-model-settings-row[data-settings-item-id="openai/gpt-5.6-sol"]'), firstRow);
	assert.equal(firstRow.querySelector('input[role="switch"]'), firstToggle);
	root.querySelector<HTMLButtonElement>('.zeta-model-settings-row[data-settings-item-id="openai/gpt-5.6-sol"] .zeta-setting-item-actions-trigger')?.click();
	assert.equal(settingActions[0]?.enabled, true);
	settingActions[0]?.run();
	await new Promise(resolve => globalThis.setTimeout(resolve, 0));
	assert.deepEqual([...hidden], []);

	const search = root.querySelector<HTMLInputElement>(".zeta-model-settings-search input")!;
	const claudeRow = root.querySelector<HTMLElement>('.zeta-model-settings-row[data-settings-item-id="anthropic/claude-opus-5"]')!;
	search.value = "anthropic";
	search.dispatchEvent(new browserEnvironment.window.Event("input", { bubbles: true }));
	assert.deepEqual([...root.querySelectorAll(".zeta-model-settings-copy h4")].map(element => element.textContent), ["Claude Opus 5"]);
	catalog = [...catalog, {
		model: { provider: "google", model: "gemini-3.6-flash" },
		displayName: "Gemini 3.6 Flash",
		access: "apiKey",
		outputTransport: "nativeStreaming",
	}];
	root.querySelector<HTMLButtonElement>(".zeta-model-settings-refresh")?.click();
	await new Promise(resolve => globalThis.setTimeout(resolve, 0));
	assert.equal(refreshes, 1);
	assert.equal(root.querySelector('.zeta-model-settings-row[data-settings-item-id="anthropic/claude-opus-5"]'), claudeRow);
	search.value = "";
	search.dispatchEvent(new browserEnvironment.window.Event("input", { bubbles: true }));
	assert.deepEqual([...root.querySelectorAll(".zeta-model-settings-copy h4")].map(element => element.textContent), ["GPT-5.6 Sol", "Claude Opus 5", "GPT-5.6", "Gemini 3.6 Flash"]);
});

test("Models settings keep ChatGPT and Kimi subscription accounts provider-scoped", async () => {
	using disposables = new DisposableStore();
	const ownerDocument = browserEnvironment.window.document;
	ownerDocument.body.replaceChildren();
	const root = h(ownerDocument, "div");
	ownerDocument.body.append(root);
	const settings = disposables.add(new SettingsService());
	const accountChanges = disposables.add(new Emitter<any>());
	const loginCompletions = disposables.add(new Emitter<any>());
	const actions: string[] = [];
	const readyAccount = {
		provider: "kimi",
		accountId: "current",
		email: undefined,
		displayName: "Kimi Code",
		organization: undefined,
		plan: "subscription",
		status: "ready" as const,
		credentialRevision: 1n,
	};
	const readyChatGptAccount = {
		provider: "openai-chatgpt",
		accountId: "chatgpt-account",
		email: "person@example.com",
		displayName: "ChatGPT",
		organization: undefined,
		plan: "plus",
		status: "ready" as const,
		credentialRevision: 1n,
	};
	disposables.add(new SettingsEditorContribution({
		accountService: {
			onDidChangeAccounts: accountChanges.event,
			onDidCompleteLogin: loginCompletions.event,
			read: async () => ({ revision: 0n, accounts: [] }),
			startLogin: async method => {
				actions.push(`start:${method.type}`);
				return method.type === "kimiDeviceCode"
					? { type: "deviceCode", loginId: "login-kimi", verificationUrl: "https://auth.kimi.com/device", userCode: "KIMI-CODE" }
					: { type: "deviceCode", loginId: "login-chatgpt", verificationUrl: "https://auth.openai.com/codex/device", userCode: "OPENAI-CODE" };
			},
			cancelLogin: async loginId => { actions.push(`cancel:${loginId}`); },
			logout: async provider => {
				actions.push(`logout:${provider}`);
			},
		},
		configurationService: disposables.add(new WorkbenchConfigurationService()),
		container: root,
		contextViewProvider: disposables.add(new BrowserContextViewService(root)),
		dialogService: acceptingDialogService,
		settingsService: settings,
		themeService: disposables.add(new ThemeService(darkColorTheme)),
		userThemeService: UnavailableUserThemeService,
		modelCatalog: {
			onDidChangeModels: () => toDisposable(() => {}),
			listModelCatalog: async () => [],
			refreshModels: async () => [],
			isModelVisible: () => true,
			setModelVisible: async () => {},
		},
	}));

	settings.open("models");
	await new Promise(resolve => globalThis.setTimeout(resolve, 0));
	assert.deepEqual(
		[...root.querySelectorAll(".zeta-model-settings-account h3")].map(element => element.textContent),
		["ChatGPT subscription", "Kimi subscription"],
	);
	const chatgptCard = root.querySelector<HTMLElement>('.zeta-model-settings-account[data-provider="openai-chatgpt"]')!;
	const chatgptButton = chatgptCard.querySelector<HTMLButtonElement>(".zeta-model-settings-account-action")!;
	chatgptButton.click();
	await new Promise(resolve => globalThis.setTimeout(resolve, 0));
	assert.equal(chatgptCard.querySelector(".zeta-model-settings-account-challenge")?.textContent, "Code: OPENAI-CODE");
	loginCompletions.fire({ loginId: "login-chatgpt", status: { type: "succeeded" }, account: { revision: 1n, accounts: [readyChatGptAccount] } });
	assert.equal(chatgptCard.querySelector(".zeta-model-settings-account-copy p")?.textContent, "Signed in as ChatGPT · plus");
	chatgptButton.click();
	await new Promise(resolve => globalThis.setTimeout(resolve, 0));

	const kimiCard = root.querySelector<HTMLElement>('.zeta-model-settings-account[data-provider="kimi"]')!;
	const button = kimiCard.querySelector<HTMLButtonElement>(".zeta-model-settings-account-action")!;
	button.click();
	await new Promise(resolve => globalThis.setTimeout(resolve, 0));
	assert.equal(kimiCard.querySelector(".zeta-model-settings-account-challenge")?.textContent, "Code: KIMI-CODE");
	assert.equal(button.textContent, "Cancel");
	loginCompletions.fire({ loginId: "login-kimi", status: { type: "succeeded" }, account: { revision: 1n, accounts: [readyAccount] } });
	assert.equal(kimiCard.querySelector(".zeta-model-settings-account-copy p")?.textContent, "Signed in as Kimi Code · subscription");
	assert.equal(button.textContent, "Sign out");
	button.click();
	await new Promise(resolve => globalThis.setTimeout(resolve, 0));
	assert.deepEqual(actions, ["start:openAiChatGptDeviceCode", "logout:openai-chatgpt", "start:kimiDeviceCode", "logout:kimi"]);
	assert.equal(button.textContent, "Sign in");
});

test("Editor settings render supported controls and persist typed preferences", async () => {
	using disposables = new DisposableStore();
	const ownerDocument = browserEnvironment.window.document;
	ownerDocument.body.replaceChildren();
	const root = h(ownerDocument, "div");
	ownerDocument.body.append(root);
	const settings = disposables.add(new SettingsService());
	const configuration = disposables.add(new WorkbenchConfigurationService());
	disposables.add(new SettingsEditorContribution({
		configurationService: configuration,
		container: root,
		contextViewProvider: disposables.add(new BrowserContextViewService(root)),
		dialogService: acceptingDialogService,
		settingsService: settings,
		themeService: disposables.add(new ThemeService(darkColorTheme)),
		userThemeService: UnavailableUserThemeService,
	}));

	settings.open("editor");
	assert.deepEqual([...root.querySelectorAll(".zeta-editor-settings-group h4")].map(element => element.textContent), ["Editor selection", "Typography", "Display", "Editing", "Code intelligence", "Find and replace", "Workspace search", "Diff editor", "Files"]);
	assert.equal(root.querySelectorAll(".zeta-editor-setting").length, 40);
	assert.equal(root.querySelectorAll(".zeta-editor-setting > .zeta-setting-item-actions").length, 38);
	assert.equal(root.querySelectorAll(".zeta-editor-informational-setting > .zeta-setting-item-actions").length, 0);
	assert.equal(root.querySelectorAll('.zeta-editor-setting[data-settings-item-kind="setting"]').length, 38);
	assert.equal(root.querySelectorAll('.zeta-editor-informational-setting[data-settings-item-kind="information"]').length, 2);
	const fontFamily = root.querySelector<HTMLInputElement>('[data-configuration-key="editor.fontFamily"]')!;
	const fontSize = root.querySelector<HTMLInputElement>('[data-configuration-key="editor.fontSize"]')!;
	const wordWrap = root.querySelector<HTMLElement>('[data-configuration-key="editor.wordWrap"]')!;
	const minimap = root.querySelector<HTMLInputElement>('[data-configuration-key="editor.minimap.enabled"]')!;
	const minimapRow = minimap.closest<HTMLElement>('.zeta-editor-setting')!;
	const lineNumbers = root.querySelector<HTMLInputElement>('[data-configuration-key="editor.lineNumbers"]')!;
	const lineNumbersRow = lineNumbers.closest<HTMLElement>('.zeta-editor-setting')!;
	const indentation = root.querySelector<HTMLElement>('[data-configuration-key="editor.indentation"]')!;
	const tabSize = root.querySelector<HTMLInputElement>('[data-configuration-key="editor.tabSize"]')!;
	assert.equal(wordWrap.querySelector<HTMLButtonElement>(".zeta-select-box-button")?.getAttribute("role"), "combobox");
	assert.equal(settingValue(root, "editor.wordWrap"), "Off");
	assert.equal(settingValue(root, "workbench.editor.defaultNewDocumentEditor"), "Default");
	const wordWrapButton = wordWrap.querySelector<HTMLButtonElement>(".zeta-select-box-button")!;
	wordWrap.closest<HTMLElement>(".zeta-editor-setting")!.querySelector<HTMLElement>(".zeta-editor-setting-copy")!.click();
	assert.equal(wordWrapButton.getAttribute("aria-expanded"), "false");
	assert.equal(fontFamily.value, "");
	assert.equal(fontFamily.placeholder, "Default monospace");
	assert.equal(fontSize.value, "13");
	assert.equal(fontSize.type, "number");
	assert.equal(fontSize.closest(".zeta-input-box")?.classList.contains("zeta-input-box-field"), true);
	assert.equal(root.querySelector<HTMLInputElement>('[data-configuration-key="editor.lineHeight"]')?.value, "20");
	assert.equal(root.querySelector<HTMLInputElement>('[data-configuration-key="editor.lineNumbers"]')?.checked, true);
	assert.equal(root.querySelector<HTMLInputElement>('[data-configuration-key="editor.formatOnSave"]')?.checked, false);
	assert.equal(root.querySelector<HTMLInputElement>('[data-configuration-key="editor.inlayHints.enabled"]')?.checked, true);
	assert.equal(root.querySelector<HTMLInputElement>('[data-configuration-key="editor.find.loop"]')?.checked, true);
	assert.equal(root.querySelector<HTMLInputElement>('[data-configuration-key="search.smartCase"]')?.checked, true);
	assert.equal(root.querySelector<HTMLInputElement>('[data-configuration-key="search.maxResults"]')?.value, "2000");
	assert.equal(root.querySelector<HTMLInputElement>('[data-configuration-key="diffEditor.showInlineChanges"]')?.checked, true);
	assert.equal(minimap.checked, true);
	assert.ok(minimap.closest<HTMLElement>(".zeta-editor-setting")?.querySelector(".zeta-editor-toggle-control.zeta-toggle-content-before-control"));
	assert.equal(settingValue(root, "editor.indentation"), "Spaces");
	assert.equal(tabSize.value, "4");

	chooseSettingOption(root, "editor.wordWrap", "On");
	await new Promise(resolve => globalThis.setTimeout(resolve, 0));
	chooseSettingOption(root, "editor.indentation", "Tabs");
	await new Promise(resolve => globalThis.setTimeout(resolve, 0));
	tabSize.value = "2";
	tabSize.dispatchEvent(new browserEnvironment.window.Event("change", { bubbles: true }));
	await new Promise(resolve => globalThis.setTimeout(resolve, 0));
	minimap.click();
	assert.equal(minimap.disabled, true);
	assert.equal(lineNumbers.disabled, false);
	assert.equal(root.querySelector('[data-settings-item-id="editor.minimap.enabled"]'), minimapRow);
	assert.equal(root.querySelector('[data-settings-item-id="editor.lineNumbers"]'), lineNumbersRow);
	await new Promise(resolve => globalThis.setTimeout(resolve, 0));

	assert.equal(configuration.getValue(CodeEditorConfiguration.wordWrap), EditorLineWrapping.On);
	assert.equal(configuration.getValue(CodeEditorConfiguration.indentationKind), EditorIndentationKind.Tabs);
	assert.equal(configuration.getValue(CodeEditorConfiguration.tabSize), 2);
	assert.equal(configuration.getValue(CodeEditorConfiguration.minimapEnabled), false);
	assert.equal(minimap.disabled, false);
	assert.equal(root.querySelector('[data-settings-item-id="editor.minimap.enabled"]'), minimapRow);
	assert.equal(root.querySelector(".zeta-editor-settings-status")?.textContent, "Setting saved.");
});

test("Connector settings project catalog state and invoke typed connect and disconnect actions", async () => {
	using disposables = new DisposableStore();
	const ownerDocument = browserEnvironment.window.document;
	ownerDocument.body.replaceChildren();
	const root = h(ownerDocument, "div");
	ownerDocument.body.append(root);
	const settings = disposables.add(new SettingsService());
	const configuration = disposables.add(new WorkbenchConfigurationService());
	const mutations: string[] = [];
	const disconnected = { id: "github", displayName: "GitHub", description: "Connect GitHub.", connectionGeneration: 0, state: { status: "disconnected" as const }, oauthMethods: [], canConnectApiToken: true, canConnectOAuth: false, canDisconnect: false, canRefreshOAuth: false, canRevokeOAuth: false };
	const connected = { id: "slack", displayName: "Slack", description: "Connect Slack.", connectionGeneration: 2, state: { status: "connected" as const, account: { id: "team", displayName: "Zeta Team" } }, oauthMethods: [], canConnectApiToken: false, canConnectOAuth: false, canDisconnect: true, canRefreshOAuth: false, canRevokeOAuth: false };
	const connectorService = {
		onDidChange: () => toDisposable(() => {}),
		list: async () => ({ generation: 7, connectors: [disconnected, connected] }),
		connectApiToken: async (connector: { id: string }, generation: number, input: { accountId: string; accountDisplayName: string; token: string }) => {
			mutations.push(`connect:${connector.id}:${generation}:${input.accountId}:${input.accountDisplayName}:${input.token}`);
		},
		connectOAuth: async () => {},
		disconnect: async (connector: { id: string }, generation: number) => {
			mutations.push(`disconnect:${connector.id}:${generation}`);
		},
		refreshOAuth: async () => {},
		revokeOAuth: async () => {},
	};
	disposables.add(new SettingsEditorContribution({
		configurationService: configuration,
		connectorService,
		container: root,
		contextViewProvider: disposables.add(new BrowserContextViewService(root)),
		dialogService: acceptingDialogService,
		settingsService: settings,
		themeService: disposables.add(new ThemeService(darkColorTheme)),
		userThemeService: UnavailableUserThemeService,
	}));

	settings.open("connectors");
	await new Promise((resolve) => globalThis.setTimeout(resolve, 0));
	assert.deepEqual([...root.querySelectorAll(".zeta-integration-heading h4")].map(element => element.textContent), ["GitHub", "Slack"]);
	assert.ok(root.querySelector('[data-settings-item-id="connectors.github"][data-settings-item-kind="resource"]'));
	assert.ok(root.querySelector('[data-settings-item-id="connectors.slack"][data-settings-item-kind="resource"]'));
	const inputs = root.querySelectorAll<HTMLInputElement>(".zeta-connector-connect-form input");
	inputs[0]!.value = "octocat";
	inputs[1]!.value = "Octocat";
	inputs[2]!.value = "secret-token";
	root.querySelector<HTMLFormElement>(".zeta-connector-connect-form")?.dispatchEvent(new browserEnvironment.window.SubmitEvent("submit", { bubbles: true, cancelable: true }));
	await new Promise((resolve) => globalThis.setTimeout(resolve, 0));
	assert.equal(inputs[2]!.value, "");
	root.querySelector<HTMLButtonElement>(".zeta-integration-card > .is-danger")?.click();
	await new Promise((resolve) => globalThis.setTimeout(resolve, 0));
	assert.deepEqual(mutations, [
		"connect:github:7:octocat:Octocat:secret-token",
		"disconnect:slack:7",
	]);
});

test("Marketplace settings discover and install through the generic service", async () => {
	using disposables = new DisposableStore();
	const ownerDocument = browserEnvironment.window.document;
	ownerDocument.body.replaceChildren();
	const root = h(ownerDocument, "div");
	ownerDocument.body.append(root);
	const settings = disposables.add(new SettingsService());
	const configuration = disposables.add(new WorkbenchConfigurationService());
	const installs: string[] = [];
	const searches: Array<{ query: string; packageType?: string; limit?: number }> = [];
	let installed: readonly any[] = [];
	let cachedBrowse: any;
	const packageValue = { id: "marketplace/docs-mcp", version: "0.3.2", packageType: "mcp", displayName: "Docs MCP", description: "Search product documentation." };
	const detailsValue = { package: { id: packageValue.id, version: packageValue.version, digest: `sha256:${"a".repeat(64)}` }, packageType: "mcp", displayName: packageValue.displayName, description: packageValue.description, license: "MIT", source: "thirdParty" as const, upstream: { registry: "officialMcp" as const, name: "ac.tandem/docs-mcp", version: "0.3.2", recordUrl: "https://registry.modelcontextprotocol.io/v0.1/servers/ac.tandem%2Fdocs-mcp/versions/0.3.2", repositoryUrl: "https://github.com/frumu-ai/tandem" }, capabilities: [{ kind: "mcp" as const, id: "docs-mcp", contractVersion: "1", permissions: ["tandem.ac"], authenticationProvider: null }] };
	const loadBrowse = async (query: string, packageType?: string, limit?: number) => {
		searches.push({ query, packageType, limit });
		cachedBrowse = { query, packageType, limit, packages: [{ summary: packageValue, details: detailsValue }], installed };
		return cachedBrowse;
	};
	const marketplaceService = {
		onDidChangeInstalled: () => ({ dispose() {}, [Symbol.dispose]() {} }),
		cachedBrowse: (query: string, packageType?: string, limit?: number) => cachedBrowse?.query === query && cachedBrowse?.packageType === packageType && cachedBrowse?.limit === limit ? cachedBrowse : undefined,
		browse: loadBrowse,
		refreshBrowse: loadBrowse,
		search: async () => [packageValue],
		get: async () => detailsValue,
		download: async () => ({ id: "art", package: { id: packageValue.id, version: packageValue.version, digest: `sha256:${"a".repeat(64)}` } }),
		install: async (id: string, version?: string) => {
			installs.push(`${id}@${version}`);
			const value = { installationId: "ins", package: { id, version: version!, digest: `sha256:${"a".repeat(64)}` }, state: "installed" as const, capabilities: [] };
			installed = [value];
			cachedBrowse = undefined;
			return value;
		},
		update: async () => { throw new Error("unused"); },
		uninstall: async () => {},
		listInstalled: async () => installed,
		acquireCapability: async () => { throw new Error("unused"); },
		releaseCapability: async () => {},
		openResource: async () => { throw new Error("unused"); },
	};
	disposables.add(new SettingsEditorContribution({
		configurationService: configuration,
		container: root,
		contextViewProvider: disposables.add(new BrowserContextViewService(root)),
		dialogService: acceptingDialogService,
		marketplaceService,
		settingsService: settings,
		themeService: disposables.add(new ThemeService(darkColorTheme)),
		userThemeService: UnavailableUserThemeService,
	}));

	settings.open("marketplace");
	await new Promise((resolve) => globalThis.setTimeout(resolve, 0));
	await new Promise((resolve) => globalThis.setTimeout(resolve, 0));
	assert.equal(root.querySelector(".zeta-package-marketplace-toolbar > button")?.textContent, "Browse Marketplace");
	assert.equal(root.querySelector(".zeta-package-marketplace-filters [role='tablist']")?.getAttribute("aria-label"), "Marketplace package types");
	assert.deepEqual([...root.querySelectorAll(".zeta-package-marketplace-filters [role='tab']")].map(tab => tab.textContent), ["All", "Plugins", "MCPs", "Skills", "Languages", "Localization", "Themes"]);
	assert.equal(root.querySelector(".zeta-package-marketplace-filters .zeta-tab.checked")?.textContent, "All");
	assert.equal(root.querySelector(".zeta-package-marketplace-card h4")?.textContent, "Docs MCP");
	assert.ok(root.querySelector('[data-settings-item-id="marketplace.marketplace/docs-mcp@0.3.2"][data-settings-item-kind="resource"]'));
	assert.match(root.textContent ?? "", /mcp: docs-mcp/);
	assert.match(root.textContent ?? "", /Listed in the official MCP Registry · ac\.tandem\/docs-mcp@0\.3\.2/);
	settings.open("general");
	settings.open("marketplace");
	assert.equal(root.querySelector(".zeta-package-marketplace-card h4")?.textContent, "Docs MCP");
	assert.deepEqual(searches, [{ query: "", packageType: undefined, limit: 100 }]);
	root.querySelectorAll<HTMLButtonElement>(".zeta-package-marketplace-filters [role='tab']")[3]?.click();
	await new Promise((resolve) => globalThis.setTimeout(resolve, 0));
	await new Promise((resolve) => globalThis.setTimeout(resolve, 0));
	assert.deepEqual(searches, [
		{ query: "", packageType: undefined, limit: 100 },
		{ query: "", packageType: "skill", limit: 100 },
	]);
	assert.equal(root.querySelector(".zeta-package-marketplace-filters .zeta-tab.checked")?.textContent, "Skills");
	root.querySelector<HTMLButtonElement>(".zeta-package-marketplace-actions button")?.click();
	await new Promise((resolve) => globalThis.setTimeout(resolve, 0));
	assert.deepEqual(installs, ["marketplace/docs-mcp@0.3.2"]);
});

test("Plugin settings project layered authority and send exact-package commands", async () => {
	using disposables = new DisposableStore();
	const ownerDocument = browserEnvironment.window.document;
	ownerDocument.body.replaceChildren();
	const root = h(ownerDocument, "div");
	ownerDocument.body.append(root);
	const settings = disposables.add(new SettingsService());
	const configuration = disposables.add(new WorkbenchConfigurationService());
	const mutations: string[] = [];
	const plugin = { id: "acme/github", version: "1.0.0", digest: `sha256:${"a".repeat(64)}`, enabled: false, granted: false, effective: false, revoked: false };
	const pluginService = {
		onDidChange: () => toDisposable(() => {}),
		list: async () => ({ revision: 7, activationGeneration: 3, packages: [plugin] }),
		enable: async (target: typeof plugin, revision: number) => { mutations.push(`enable:${target.id}:${target.digest}:${revision}`); },
		disable: async () => {},
		grant: async (target: typeof plugin, revision: number) => { mutations.push(`grant:${target.id}:${target.digest}:${revision}`); },
		revokeGrant: async () => {},
		uninstall: async (target: typeof plugin, revision: number) => { mutations.push(`uninstall:${target.id}:${target.digest}:${revision}`); },
	};
	disposables.add(new SettingsEditorContribution({
		configurationService: configuration,
		container: root,
		contextViewProvider: disposables.add(new BrowserContextViewService(root)),
		dialogService: acceptingDialogService,
		pluginService,
		settingsService: settings,
		themeService: disposables.add(new ThemeService(darkColorTheme)),
		userThemeService: UnavailableUserThemeService,
	}));

	settings.open("plugins");
	await new Promise((resolve) => globalThis.setTimeout(resolve, 0));
	assert.equal(root.querySelector(".zeta-integration-heading h4")?.textContent, "acme/github · 1.0.0");
	assert.ok(root.querySelector('[data-settings-item-id="plugins.acme/github@1.0.0"][data-settings-item-kind="resource"]'));
	const buttons = [...root.querySelectorAll<HTMLButtonElement>(".zeta-integration-card > .zeta-theme-action")];
	assert.deepEqual(buttons.map(button => button.textContent), ["Grant", "Enable", "Remove legacy installation"]);
	buttons[0]!.click();
	await new Promise((resolve) => globalThis.setTimeout(resolve, 0));
	assert.deepEqual(mutations, [`grant:acme/github:${plugin.digest}:7`]);
});

test("Plugin settings direct package discovery to the generic Marketplace", async () => {
	using disposables = new DisposableStore();
	const ownerDocument = browserEnvironment.window.document;
	ownerDocument.body.replaceChildren();
	const root = h(ownerDocument, "div");
	ownerDocument.body.append(root);
	const settings = disposables.add(new SettingsService());
	const configuration = disposables.add(new WorkbenchConfigurationService());
	const pluginService = {
		onDidChange: () => toDisposable(() => {}),
		list: async () => ({ revision: 4, activationGeneration: 0, packages: [] }),
		enable: async () => {},
		disable: async () => {},
		grant: async () => {},
		revokeGrant: async () => {},
		uninstall: async () => {},
	};
	disposables.add(new SettingsEditorContribution({
		configurationService: configuration,
		container: root,
		contextViewProvider: disposables.add(new BrowserContextViewService(root)),
		dialogService: acceptingDialogService,
		pluginService,
		settingsService: settings,
		themeService: disposables.add(new ThemeService(darkColorTheme)),
		userThemeService: UnavailableUserThemeService,
	}));

	settings.open("plugins");
	await new Promise((resolve) => globalThis.setTimeout(resolve, 0));
	assert.match(root.textContent ?? "", /No legacy plugins are installed/);
	assert.match(root.textContent ?? "", /Discover new packages in Marketplace/);
	assert.equal(root.querySelector(".zeta-marketplace-results"), null);
});

test("Language settings reuse Marketplace discovery with a language package filter", async () => {
	using disposables = new DisposableStore();
	const ownerDocument = browserEnvironment.window.document;
	ownerDocument.body.replaceChildren();
	const root = h(ownerDocument, "div");
	ownerDocument.body.append(root);
	const settings = disposables.add(new SettingsService());
	const configuration = disposables.add(new WorkbenchConfigurationService());
	const searches: Array<{ query: string; packageType?: string; limit?: number }> = [];
	let cachedBrowse: any;
	const loadBrowse = async (query: string, packageType?: string, limit?: number) => {
		searches.push({ query, packageType, limit });
		cachedBrowse = { query, packageType, limit, packages: [], installed: [] };
		return cachedBrowse;
	};
	const marketplaceService = {
		onDidChangeInstalled: () => ({ dispose() {}, [Symbol.dispose]() {} }),
		cachedBrowse: (query: string, packageType?: string, limit?: number) => cachedBrowse?.query === query && cachedBrowse?.packageType === packageType && cachedBrowse?.limit === limit ? cachedBrowse : undefined,
		browse: loadBrowse,
		refreshBrowse: loadBrowse,
		search: async () => [],
		get: async () => { throw new Error("unexpected get"); },
		download: async () => { throw new Error("unexpected download"); },
		install: async () => { throw new Error("unexpected install"); },
		update: async () => { throw new Error("unexpected update"); },
		uninstall: async () => { throw new Error("unexpected uninstall"); },
		listInstalled: async () => [],
		acquireCapability: async () => { throw new Error("unexpected acquire"); },
		releaseCapability: async () => {},
		openResource: async () => { throw new Error("unexpected resource"); },
	};
	disposables.add(new SettingsEditorContribution({
		configurationService: configuration,
		container: root,
		contextViewProvider: disposables.add(new BrowserContextViewService(root)),
		dialogService: acceptingDialogService,
		marketplaceService,
		settingsService: settings,
		themeService: disposables.add(new ThemeService(darkColorTheme)),
		userThemeService: UnavailableUserThemeService,
	}));

	settings.open("languages");
	await new Promise((resolve) => globalThis.setTimeout(resolve, 0));
	assert.deepEqual(searches, [{ query: "", packageType: "language", limit: 100 }]);
	assert.match(root.textContent ?? "", /No matching packages/);
	assert.equal(root.querySelector<HTMLInputElement>('input[type="search"]')?.placeholder, "Search settings");
	assert.equal(root.querySelector<HTMLInputElement>('.zeta-package-marketplace input[type="search"]')?.placeholder, "Search language extensions");
});

test("Display Language exposes its configuration ID while language packs keep resource IDs", async () => {
	using disposables = new DisposableStore();
	const ownerDocument = browserEnvironment.window.document;
	ownerDocument.body.replaceChildren();
	const root = h(ownerDocument, "div");
	ownerDocument.body.append(root);
	const settings = disposables.add(new SettingsService());
	const copiedSettingIds: string[] = [];
	const selectedLocales: string[] = [];
	let currentLocale = "zh-CN";
	let settingActions: readonly IAction[] = [];
	let hideSettingActions = (): void => {};
	disposables.add(new SettingsEditorContribution({
		clipboardService: { writeText: async value => { copiedSettingIds.push(value); } },
		configurationService: disposables.add(new WorkbenchConfigurationService()),
		container: root,
		contextMenuProvider: {
			showContextMenu: options => {
				if (!("actions" in options)) throw new Error("Expected direct setting actions");
				settingActions = options.actions;
				hideSettingActions = () => options.onHide?.(false);
			},
		},
		contextViewProvider: disposables.add(new BrowserContextViewService(root)),
		dialogService: acceptingDialogService,
		languagePackService: {
			onDidChange: () => toDisposable(() => {}),
			whenReady: Promise.resolve(),
			catalogs: [],
			availableLocales: [
				{ locale: "en", languageName: "English", localizedLanguageName: "English", source: "builtin" },
				{ locale: "zh-CN", languageName: "Chinese", localizedLanguageName: "简体中文", source: "marketplace" },
			],
			installedPackages: [],
			search: async () => [{ id: "zeta.language.fr", version: "1.2.0", displayName: "Français", description: "French", installed: false }],
			install: async () => {},
			refresh: async () => {},
		},
		localeService: {
			get locale() { return currentLocale; },
			onDidChangeLocale: () => toDisposable(() => {}),
			whenReady: Promise.resolve(),
			setLocale: async locale => {
				currentLocale = locale;
				selectedLocales.push(locale);
			},
		},
		localizationService: {
			onDidChange: () => toDisposable(() => {}),
			whenReady: Promise.resolve(),
			translate: (_bundle, _key, fallback) => fallback,
		},
		settingsService: settings,
		themeService: disposables.add(new ThemeService(darkColorTheme)),
		userThemeService: UnavailableUserThemeService,
	}));

	settings.open("localization");
	await new Promise(resolve => globalThis.setTimeout(resolve, 0));
	await new Promise(resolve => globalThis.setTimeout(resolve, 0));
	const localeItem = root.querySelector<HTMLElement>('[data-settings-item-id="workbench.locale"]');
	assert.equal(localeItem?.dataset.settingsItemKind, "setting");
	assert.ok(root.querySelector('[data-settings-item-id="languagePacks.zeta.language.fr@1.2.0"][data-settings-item-kind="resource"]'));
	localeItem?.querySelector<HTMLButtonElement>(".zeta-setting-item-actions-trigger")?.click();
	assert.equal(settingActions[0]?.enabled, true);
	settingActions[1]?.run();
	await Promise.resolve();
	assert.deepEqual(copiedSettingIds, ["workbench.locale"]);
	hideSettingActions();
	localeItem?.querySelector<HTMLButtonElement>(".zeta-setting-item-actions-trigger")?.click();
	settingActions[0]?.run();
	await Promise.resolve();
	assert.deepEqual(selectedLocales, ["en"]);
});

test("Appearance settings persist and dynamically render registered theme preferences", async () => {
	using disposables = new DisposableStore();
	const ownerDocument = browserEnvironment.window.document;
	ownerDocument.body.replaceChildren();
	const root = h(ownerDocument, "div");
	ownerDocument.body.append(root);
	const settings = disposables.add(new SettingsService());
	const configuration = disposables.add(new WorkbenchConfigurationService());
	const copiedSettingIds: string[] = [];
	let settingActions: readonly IAction[] = [];
	let hideSettingActions = (): void => {};
	disposables.add(new SettingsEditorContribution({
		clipboardService: { writeText: async value => { copiedSettingIds.push(value); } },
		configurationService: configuration,
		container: root,
		contextMenuProvider: {
			showContextMenu: options => {
				if (!("actions" in options)) throw new Error("Expected direct setting actions");
				settingActions = options.actions;
				hideSettingActions = () => options.onHide?.(false);
			},
		},
		contextViewProvider: disposables.add(new BrowserContextViewService(root)),
		dialogService: acceptingDialogService,
		settingsService: settings,
		themeService: disposables.add(new ThemeService(darkColorTheme)),
		userThemeService: UnavailableUserThemeService,
	}));

	settings.open("appearance");
	const appearanceItem = root.querySelector<HTMLElement>('[data-settings-item-id="workbench.colorTheme"]');
	assert.equal(appearanceItem?.dataset.settingsItemKind, "setting");
	appearanceItem?.querySelector<HTMLButtonElement>(".zeta-setting-item-actions-trigger")?.click();
	assert.deepEqual(settingActions.map(action => ({ label: action.label, enabled: action.enabled })), [
		{ label: "Reset Setting", enabled: false },
		{ label: "Copy Setting ID", enabled: true },
	]);
	settingActions[1]?.run();
	await Promise.resolve();
	assert.deepEqual(copiedSettingIds, ["workbench.colorTheme"]);
	hideSettingActions();
	let options = [...root.querySelectorAll<HTMLInputElement>("[data-theme-preference] input")];
	assert.deepEqual(options.map((option) => option.value), ["system", "zeta-light", "zeta-dark"]);
	using themeRegistration = WorkbenchThemesRegistry.registerColorTheme(createColorTheme({
		id: "zeta-test-aurora",
		label: "Zeta Test Aurora",
		colorScheme: ColorScheme.Dark,
	}));
	options = [...root.querySelectorAll<HTMLInputElement>("[data-theme-preference] input")];
	assert.deepEqual(options.map((option) => option.value), [
		"system",
		"zeta-light",
		"zeta-dark",
		"zeta-test-aurora",
	]);
	assert.equal(options[0]?.checked, true);

	options[1]?.click();
	await Promise.resolve();
	assert.equal(configuration.getValue(WorkbenchConfiguration.colorTheme), "zeta-light");
	assert.equal(root.querySelector<HTMLInputElement>("[value='zeta-light']")?.checked, true);
	root.querySelector<HTMLButtonElement>('[data-settings-item-id="workbench.colorTheme"] .zeta-setting-item-actions-trigger')?.click();
	assert.equal(settingActions[0]?.enabled, true);
	settingActions[0]?.run();
	await new Promise(resolve => globalThis.setTimeout(resolve, 0));
	assert.equal(configuration.getValue(WorkbenchConfiguration.colorTheme), WorkbenchConfiguration.colorTheme.defaultValue);
	hideSettingActions();

	root.querySelector<HTMLInputElement>("[value='zeta-dark']")?.click();
	await Promise.resolve();
	assert.equal(configuration.getValue(WorkbenchConfiguration.colorTheme), "zeta-dark");

	root.querySelector<HTMLInputElement>("[value='zeta-test-aurora']")?.click();
	await Promise.resolve();
	assert.equal(configuration.getValue(WorkbenchConfiguration.colorTheme), "zeta-test-aurora");

	root.querySelector<HTMLInputElement>("[value='system']")?.click();
	await Promise.resolve();
	assert.equal(configuration.getValue(WorkbenchConfiguration.colorTheme), "system");
	themeRegistration.dispose();
	assert.equal(root.querySelector<HTMLInputElement>("[value='zeta-test-aurora']"), null);
});

test("Appearance edits the resolved Light JSON, previews it, and saves a new theme", async () => {
	using disposables = new DisposableStore();
	const ownerDocument = browserEnvironment.window.document;
	ownerDocument.body.replaceChildren();
	const root = h(ownerDocument, "div");
	ownerDocument.body.append(root);
	const settings = disposables.add(new SettingsService());
	const configuration = disposables.add(new WorkbenchConfigurationService());
	const themeService = disposables.add(new ThemeService(lightColorTheme));
	const savedSources = new Map<string, { file: string; source: string }>();
	const themeRegistrations = new Map<string, { dispose(): void }>();
	const userThemes = {
		available: true,
		directory: "C:\\themes",
		issues: [],
		sourceFor: (themeId: string) => {
			const saved = savedSources.get(themeId);
			return saved ? { id: themeId, file: saved.file } : undefined;
		},
		getSource: (themeId: string) => savedSources.get(themeId)?.source,
		delete: async (themeId: string) => {
			const theme = WorkbenchThemesRegistry.getColorTheme(themeId);
			const saved = savedSources.get(themeId);
			if (!theme || !saved) throw new Error("Theme is not loaded");
			themeRegistrations.get(themeId)?.dispose();
			themeRegistrations.delete(themeId);
			savedSources.delete(themeId);
			return { colorScheme: theme.colorScheme, file: saved.file };
		},
		reload: async () => {},
		save: async () => {
			throw new Error("Unexpected direct save");
		},
		saveAs: async (source: string) => {
			const theme = parseUserColorTheme(source);
			const file = `${theme.id}.json`;
			savedSources.set(theme.id, { file, source });
			const registration = disposables.add(WorkbenchThemesRegistry.registerColorTheme(theme));
			themeRegistrations.set(theme.id, registration);
			return { file, theme };
		},
	};
	disposables.add(new SettingsEditorContribution({
		configurationService: configuration,
		container: root,
		contextViewProvider: disposables.add(new BrowserContextViewService(root)),
		dialogService: acceptingDialogService,
		settingsService: settings,
		themeService,
		userThemeService: userThemes,
	}));

	settings.open("appearance");
	root.querySelector<HTMLButtonElement>(".zeta-theme-customization button")?.click();
	const editor = root.querySelector<HTMLTextAreaElement>(".zeta-theme-json-editor");
	assert.ok(editor);
	const draft = JSON.parse(editor.value) as { id: string; label: string; colorScheme: string; colors: Record<string, string> };
	assert.equal(draft.colorScheme, "light");
	assert.equal(draft.colors["editor.background"], lightColorTheme.getColorCss("editor.background"));
	draft.id = "test-settings-light";
	draft.label = "Test Settings Light";
	draft.colors["editor.background"] = "#f0f1f2";
	editor.value = JSON.stringify(draft, null, 2);
	editor.dispatchEvent(new browserEnvironment.window.Event("input", { bubbles: true }));
	assert.equal(themeService.getColorTheme().getColorCss("editor.background"), "#f0f1f2");

	root.querySelector<HTMLButtonElement>(".zeta-theme-json-actions button")?.click();
	await new Promise((resolve) => globalThis.setTimeout(resolve, 0));
	assert.equal(configuration.getValue(WorkbenchConfiguration.colorTheme), "test-settings-light");
	assert.equal(root.querySelector(".zeta-theme-setting-status")?.textContent, "Saved Test Settings Light to test-settings-light.json.");
	assert.equal(root.querySelector<HTMLTextAreaElement>(".zeta-theme-json-editor"), null);

	root.querySelector<HTMLButtonElement>(".zeta-theme-customization button")?.click();
	const deleteButton = [...root.querySelectorAll<HTMLButtonElement>(".zeta-theme-json-actions button")]
		.find((button) => button.textContent === "Delete");
	assert.ok(deleteButton);
	deleteButton.click();
	await new Promise((resolve) => globalThis.setTimeout(resolve, 0));
	assert.equal(configuration.getValue(WorkbenchConfiguration.colorTheme), "zeta-light");
	assert.equal(WorkbenchThemesRegistry.getColorTheme("test-settings-light"), undefined);
	assert.equal(savedSources.has("test-settings-light"), false);
	assert.equal(root.querySelector(".zeta-theme-setting-status")?.textContent, "Deleted Test Settings Light (test-settings-light.json) and switched to Zeta Light.");
});

test("Settings service retains the active section across visibility changes", () => {
	using settings = new SettingsService();
	const selected: string[] = [];
	settings.onDidChangeActiveSection((sectionId) => selected.push(sectionId));

	settings.open("editor");
	settings.close();
	settings.open();

	assert.equal(settings.activeSectionId, "editor");
	assert.deepEqual(selected, ["editor"]);
	assert.throws(() => settings.open(""), /must not be empty/);
});

test('Preferences service opens Settings without owning its editor state', () => {
	using disposables = new DisposableStore();
	const settings = disposables.add(new SettingsService());
	const preferences = new PreferencesService(settings);

	preferences.openSettings('editor');
	assert.equal(settings.isOpen, true);
	assert.equal(settings.activeSectionId, 'editor');
});

test("Indexing settings save Tool Search and semantic model consent configuration", async () => {
	using disposables = new DisposableStore();
	const ownerDocument = browserEnvironment.window.document;
	ownerDocument.body.replaceChildren();
	const root = h(ownerDocument, "div");
	ownerDocument.body.append(root);
	const settings = disposables.add(new SettingsService());
	const configuration = disposables.add(new WorkbenchConfigurationService());
	const copiedSettingIds: string[] = [];
	let settingActions: readonly IAction[] = [];
	let hideSettingActions = (): void => {};
	const config = {
		revision: 4,
		generation: 4,
		providers: {
			ollama: {
				provider: "ollama",
				baseUrl: "http://localhost:11434/v1",
				maxOutputTokens: null,
				modelContext: {},
			},
		},
		semanticCodeIndex: {
			automaticContext: "off",
			selection: {
				type: "remote",
				models: {
					embeddingModel: { provider: "ollama", model: "nomic-embed-text" },
					rerankModel: null,
				},
			},
			activeWorkspaceAuthorized: false,
		},
	} as const satisfies CodeIndexConfigurationSnapshot;
	const toolSearchConfig = {
		revision: 4,
		mode: "hybridEmbedding" as const,
		embeddingModel: { provider: "ollama", model: "nomic-embed-text" },
		embeddingStatus: {
			type: "unavailable" as const,
			model: { provider: "ollama", model: "nomic-embed-text" },
			reason: "connection refused",
		},
	};
	const configured: Array<{ mode: string; embeddingModel?: { provider: string; model: string }; revision: number }> = [];
	const configuredProviders: Array<{ provider: string; baseUrl: string | null; revision: number }> = [];
	const configuredSemantic: Array<{ selection: SemanticCodeIndexSelection; automaticContext: string; revision: number }> = [];
	let authorizations = 0;
	disposables.add(new SettingsEditorContribution({
		clipboardService: { writeText: async value => { copiedSettingIds.push(value); } },
		configurationService: configuration,
		container: root,
		contextMenuProvider: {
			showContextMenu: options => {
				if (!("actions" in options)) throw new Error("Expected direct setting actions");
				settingActions = options.actions;
				hideSettingActions = () => options.onHide?.(false);
			},
		},
		contextViewProvider: disposables.add(new BrowserContextViewService(root)),
		dialogService: acceptingDialogService,
		settingsService: settings,
		themeService: disposables.add(new ThemeService(darkColorTheme)),
		userThemeService: UnavailableUserThemeService,
		codeIndexService: {
			readConfig: async () => config,
			configureProvider: async (next, revision) => {
				configuredProviders.push({ provider: next.provider, baseUrl: next.baseUrl ?? null, revision });
				return { revision: 4, generation: 4, disposition: "updated" };
			},
			configure: async (selection, automaticContext, revision) => {
				configuredSemantic.push({ selection, automaticContext, revision });
				return { revision: 4, generation: 4, disposition: "updated" };
			},
			authorize: async () => {
				authorizations += 1;
				return { revision: 4, generation: 4, disposition: "updated" };
			},
			revoke: async () => ({ revision: 4, generation: 4, disposition: "updated" }),
			status: () => Promise.reject(new Error("Code index runtime is not exercised by this test.")),
			cancel: () => Promise.reject(new Error("Code index runtime is not exercised by this test.")),
			retry: () => Promise.reject(new Error("Code index runtime is not exercised by this test.")),
		},
		toolSearchService: {
			readConfig: async () => toolSearchConfig,
			configure: async (next, revision) => { configured.push({ ...next, revision }); },
		},
	}));

	settings.open("indexing");
	await new Promise((resolve) => globalThis.setTimeout(resolve, 0));

	assert.deepEqual(
		[...root.querySelectorAll(".zeta-indexing-setting legend")].map((legend) => legend.textContent),
		["Agent tool search", "Semantic code search"],
	);
	const indexingItems = [...root.querySelectorAll<HTMLElement>('.zeta-indexing-setting-item[data-settings-item-kind="setting"]')];
	assert.deepEqual(indexingItems.map(item => item.dataset.settingsItemId), ["toolSearch.configuration", "codeIndex.semanticCodeIndex"]);
	for (const item of indexingItems) {
		item.querySelector<HTMLButtonElement>(".zeta-setting-item-actions-trigger")?.click();
		settingActions[1]?.run();
		await Promise.resolve();
		hideSettingActions();
	}
	assert.deepEqual(copiedSettingIds, ["toolSearch.configuration", "codeIndex.semanticCodeIndex"]);
	assert.match(root.textContent ?? "", /Embedding search is unavailable: connection refused/);
	const toolGroup = root.querySelectorAll<HTMLFieldSetElement>(".zeta-indexing-setting")[0];
	assert.ok(toolGroup);
	const input = toolGroup.querySelector<HTMLInputElement>(".zeta-settings-text-input");
	assert.equal(input?.value, "ollama/nomic-embed-text");
	input!.value = "ollama/mxbai-embed-large";
	[...toolGroup.querySelectorAll<HTMLButtonElement>("button")]
		.find((button) => button.textContent === "Save tool search")
		?.click();
	await new Promise((resolve) => globalThis.setTimeout(resolve, 0));

	assert.deepEqual(configured, [{
		mode: "hybridEmbedding",
		embeddingModel: { provider: "ollama", model: "mxbai-embed-large" },
		revision: 4,
	}]);

	let semanticGroup = root.querySelectorAll<HTMLFieldSetElement>(".zeta-indexing-setting")[1];
	assert.ok(semanticGroup);
	const provider = semanticGroup.querySelector<HTMLInputElement>('[aria-label="Semantic model provider"]');
	const endpoint = semanticGroup.querySelector<HTMLInputElement>('[aria-label="Semantic model endpoint URL"]');
	assert.ok(provider);
	assert.ok(endpoint);
	provider.value = "openai-compatible";
	endpoint.value = "https://models.example.test/v1";
	[...semanticGroup.querySelectorAll<HTMLButtonElement>("button")]
		.find((button) => button.textContent === "Save endpoint")
		?.click();
	await new Promise((resolve) => globalThis.setTimeout(resolve, 0));
	assert.deepEqual(configuredProviders, [{
		provider: "openai-compatible",
		baseUrl: "https://models.example.test/v1",
		revision: 4,
	}]);

	semanticGroup = root.querySelectorAll<HTMLFieldSetElement>(".zeta-indexing-setting")[1]!;
	const semanticEmbedding = semanticGroup.querySelector<HTMLInputElement>('[aria-label="Embedding model"]');
	assert.ok(semanticEmbedding);
	semanticEmbedding.value = "ollama/mxbai-embed-large";
	[...semanticGroup.querySelectorAll<HTMLButtonElement>("button")]
		.find((button) => button.textContent === "Save model selection")
		?.click();
	await new Promise((resolve) => globalThis.setTimeout(resolve, 0));
	assert.deepEqual(configuredSemantic, [{
		selection: {
			type: "remote",
			models: {
				embeddingModel: { provider: "ollama", model: "mxbai-embed-large" },
				rerankModel: null,
			},
		},
		automaticContext: "off",
		revision: 4,
	}]);

	semanticGroup = root.querySelectorAll<HTMLFieldSetElement>(".zeta-indexing-setting")[1]!;
	[...semanticGroup.querySelectorAll<HTMLButtonElement>("button")]
		.find((button) => button.textContent === "Authorize active workspace")
		?.click();
	await new Promise((resolve) => globalThis.setTimeout(resolve, 0));
	assert.equal(authorizations, 1);

	root.querySelector<HTMLButtonElement>('[data-settings-item-id="toolSearch.configuration"] .zeta-setting-item-actions-trigger')?.click();
	assert.equal(settingActions[0]?.enabled, true);
	settingActions[0]?.run();
	await new Promise(resolve => globalThis.setTimeout(resolve, 0));
	assert.deepEqual(configured.at(-1), { mode: "lexical", revision: 4 });
	hideSettingActions();
	root.querySelector<HTMLButtonElement>('[data-settings-item-id="codeIndex.semanticCodeIndex"] .zeta-setting-item-actions-trigger')?.click();
	assert.equal(settingActions[0]?.enabled, true);
	settingActions[0]?.run();
	await new Promise(resolve => globalThis.setTimeout(resolve, 0));
	assert.deepEqual(configuredSemantic.at(-1), { selection: { type: "disabled" }, automaticContext: "off", revision: 4 });
});

test("Zeta Settings titlebar action opens the window Preferences service", async () => {
	using disposables = new DisposableStore();
	const settings = disposables.add(new SettingsService());
	const preferences = new PreferencesService(settings);
	const services = new ServiceCollection();
	services.set(IPreferencesService, preferences);
	const commands = disposables.add(new CommandService(services));
	const contextKeys = disposables.add(new ContextKeyService());
	const menus = new MenuService(commands, contextKeys);
	const action = menus.getMenuActions(MenuId.TitleBar)
		.flatMap(([, actions]) => actions)
		.find((candidate) => candidate.id === OpenSettingsCommandId);

	assert.ok(action);
	assert.equal(action.label, "Zeta Settings");
	assert.equal(action.icon, lxiconsLibrary.gear);

	const bindingFor = (operatingSystem: (typeof OperatingSystem)[keyof typeof OperatingSystem]) =>
		new KeybindingResolver({
			registry: KeybindingsRegistry,
			resolveKeybinding: (keybinding) =>
				resolveKeybinding(keybinding, operatingSystem),
		}).lookupKeybinding(OpenSettingsCommandId, contextKeys);
	const windowsBinding = bindingFor(OperatingSystem.Windows);
	const macBinding = bindingFor(OperatingSystem.Macintosh);
	assert.ok(windowsBinding);
	assert.ok(macBinding);
	assert.equal(
		getKeybindingLabel(
			windowsBinding,
			KeybindingLabelStyle.UserSettings,
		),
		"ctrl+,",
	);
	assert.equal(
		getKeybindingLabel(macBinding, KeybindingLabelStyle.UserSettings),
		"cmd+,",
	);

	const keyboardLayout = disposables.add(
		new BrowserKeyboardLayoutService({
			navigator: browserEnvironment.window.navigator,
			operatingSystem: OperatingSystem.Windows,
		}),
	);
	disposables.add(new WorkbenchKeybindingService({
		ownerDocument: browserEnvironment.window.document,
		commandService: commands,
		contextKeyService: contextKeys,
		keyboardLayoutService: keyboardLayout,
	}));
	const shortcut = new browserEnvironment.window.KeyboardEvent(
		"keydown",
		{
			bubbles: true,
			cancelable: true,
			code: "Comma",
			ctrlKey: true,
			key: ",",
		},
	);
	browserEnvironment.window.document.body.dispatchEvent(shortcut);
	assert.equal(shortcut.defaultPrevented, true);
	assert.equal(settings.isOpen, true);

	settings.close();
	await commands.executeCommand(OpenSettingsCommandId);
	assert.equal(settings.isOpen, true);
});
