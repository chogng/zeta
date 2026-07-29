import assert from "node:assert/strict";
import test from "node:test";
import { JSDOM } from "jsdom";

const browserEnvironment = new JSDOM("<!doctype html><body></body>", {
  pretendToBeVisual: true,
});
Object.defineProperty(browserEnvironment.window.Element.prototype, "scrollTo", {
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

const { DisposableStore, toDisposable } = await import("../src/zeta/base/common/lifecycle.js");
const { KeybindingLabelStyle, getKeybindingLabel } = await import("../src/zeta/base/common/keybindingLabels.js");
const { resolveKeybinding } = await import("../src/zeta/base/common/keybindings.js");
const { LxIcon } = await import("../src/zeta/base/common/lxicons.js");
const { OperatingSystem } = await import("../src/zeta/base/common/platform.js");
const { MenuId } = await import("../src/zeta/platform/actions/common/actions.js");
const { MenuService } = await import("../src/zeta/platform/actions/common/menuService.js");
const { ContextKeyService } = await import("../src/zeta/platform/contextkey/common/contextkey.js");
const { ServiceCollection } = await import("../src/zeta/platform/instantiation/common/instantiation.js");
const { KeybindingResolver } = await import("../src/zeta/platform/keybinding/common/keybindingResolver.js");
const { KeybindingsRegistry } = await import("../src/zeta/platform/keybinding/common/keybindingsRegistry.js");
const { ISettingsService } = await import("../src/zeta/workbench/services/preferences/common/settings.js");
const { SettingsService } = await import("../src/zeta/workbench/services/preferences/common/settingsService.js");
const { SettingsEditorContribution } = await import("../src/zeta/workbench/contrib/preferences/browser/settingsEditor.contribution.js");
const { CommandService } = await import("../src/zeta/workbench/services/commands/common/commandService.js");
const { BrowserKeyboardLayoutService } = await import("../src/zeta/workbench/services/keybinding/browser/keyboardLayoutService.js");
const { WorkbenchKeybindingService } = await import("../src/zeta/workbench/services/keybinding/browser/keybindingService.js");
const { OpenSettingsCommandId } = await import("../src/zeta/workbench/contrib/preferences/browser/preferences.contribution.js");

test("Settings overlay opens, closes, and restores focus", () => {
  using disposables = new DisposableStore();
  const ownerDocument = browserEnvironment.window.document;
  ownerDocument.body.replaceChildren();
  const root = ownerDocument.createElement("div");
  const trigger = ownerDocument.createElement("button");
  trigger.textContent = "Open";
  root.append(trigger);
  ownerDocument.body.append(root);
  trigger.focus();

  const settings = disposables.add(new SettingsService());
  disposables.add(new SettingsEditorContribution({
    container: root,
    settingsService: settings,
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
      "User",
      "Appearance",
      "Editor",
      "Agents",
      "Models",
      "Git",
      "Worktrees",
      "Plugins",
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

  navigationItems[5].click();
  assert.equal(settings.activeSectionId, "models");
  assert.equal(navigationItems[0].hasAttribute("aria-current"), false);
  assert.equal(navigationItems[5].getAttribute("aria-current"), "page");
  assert.equal(root.querySelector(".zeta-settings-page h3")?.textContent, "Models");
  assert.equal(
    root.querySelector(".zeta-settings-page")?.getAttribute("data-active-settings-section"),
    "models",
  );

  surface.dispatchEvent(new browserEnvironment.window.KeyboardEvent(
    "keydown",
    { bubbles: true, key: "Escape" },
  ));
  assert.equal(settings.isOpen, false);
  assert.equal(host.hidden, true);
  assert.equal(ownerDocument.activeElement, trigger);
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

test("Zeta Settings titlebar action opens the window Settings service", async () => {
  using disposables = new DisposableStore();
  let opens = 0;
  const settings = {
    onDidChangeVisibility: () => toDisposable(() => {}),
    onDidChangeActiveSection: () => toDisposable(() => {}),
    get isOpen() { return opens > 0; },
    activeSectionId: "general",
    open() { opens += 1; },
    close() {},
  };
  const services = new ServiceCollection();
  services.set(ISettingsService, settings);
  const commands = disposables.add(new CommandService(services));
  const contextKeys = disposables.add(new ContextKeyService());
  const menus = new MenuService(commands, contextKeys);
  const action = menus.getMenuActions(MenuId.TitleBar)
    .flatMap(([, actions]) => actions)
    .find((candidate) => candidate.id === OpenSettingsCommandId);

  assert.ok(action);
  assert.equal(action.label, "Zeta Settings");
  assert.equal(action.icon, LxIcon.gear);

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
  assert.equal(opens, 1);

  await commands.executeCommand(OpenSettingsCommandId);
  assert.equal(opens, 2);
});
