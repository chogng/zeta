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
const { lxiconsLibrary } = await import("../src/zeta/base/common/lxiconsLibrary.js");
const { OperatingSystem } = await import("../src/zeta/base/common/platform.js");
const { MenuId } = await import("../src/zeta/platform/actions/common/actions.js");
const { MenuService } = await import("../src/zeta/platform/actions/common/menuService.js");
const { ContextKeyService } = await import("../src/zeta/platform/contextkey/common/contextkey.js");
const { ServiceCollection } = await import("../src/zeta/platform/instantiation/common/instantiation.js");
const { KeybindingResolver } = await import("../src/zeta/platform/keybinding/common/keybindingResolver.js");
const { KeybindingsRegistry } = await import("../src/zeta/platform/keybinding/common/keybindingsRegistry.js");
const { createColorTheme, darkColorTheme, lightColorTheme } = await import("../src/zeta/platform/theme/common/colorTheme.js");
const { ThemeService } = await import("../src/zeta/platform/theme/common/themeService.js");
const { parseUserColorTheme } = await import("../src/zeta/platform/theme/common/userColorTheme.js");
const { ColorScheme } = await import("../src/zeta/platform/theme/common/theme.js");
const { ISettingsService } = await import("../src/zeta/workbench/services/preferences/common/settings.js");
const { SettingsService } = await import("../src/zeta/workbench/services/preferences/common/settingsService.js");
const { WorkbenchConfiguration } = await import("../src/zeta/workbench/common/configuration.js");
const { WorkbenchThemesRegistry } = await import("../src/zeta/workbench/common/theme.js");
const { UnavailableUserThemeService } = await import("../src/zeta/workbench/common/userThemes.js");
const { WorkbenchConfigurationService } = await import("../src/zeta/workbench/services/configuration/browser/configurationService.js");
const { SettingsEditorContribution } = await import("../src/zeta/workbench/contrib/preferences/browser/settingsEditor.contribution.js");
const { CommandService } = await import("../src/zeta/workbench/services/commands/common/commandService.js");
const { BrowserKeyboardLayoutService } = await import("../src/zeta/workbench/services/keybinding/browser/keyboardLayoutService.js");
const { WorkbenchKeybindingService } = await import("../src/zeta/workbench/services/keybinding/browser/keybindingService.js");
const { OpenSettingsCommandId } = await import("../src/zeta/workbench/contrib/preferences/browser/preferences.contribution.js");

const acceptingDialogService = {
  showMessage: async () => {},
  confirm: async () => true,
};

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
  const configuration = disposables.add(new WorkbenchConfigurationService());
  disposables.add(new SettingsEditorContribution({
    configurationService: configuration,
    container: root,
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

  navigationItems[6].click();
  assert.equal(settings.activeSectionId, "models");
  assert.equal(navigationItems[0].hasAttribute("aria-current"), false);
  assert.equal(navigationItems[6].getAttribute("aria-current"), "page");
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

test("Appearance settings persist and dynamically render registered theme preferences", async () => {
  using disposables = new DisposableStore();
  using themeRegistration = WorkbenchThemesRegistry.registerColorTheme(createColorTheme({
    id: "zeta-test-aurora",
    label: "Zeta Test Aurora",
    colorScheme: ColorScheme.Dark,
  }));
  const ownerDocument = browserEnvironment.window.document;
  ownerDocument.body.replaceChildren();
  const root = ownerDocument.createElement("div");
  ownerDocument.body.append(root);
  const settings = disposables.add(new SettingsService());
  const configuration = disposables.add(new WorkbenchConfigurationService());
  disposables.add(new SettingsEditorContribution({
    configurationService: configuration,
    container: root,
    dialogService: acceptingDialogService,
    settingsService: settings,
    themeService: disposables.add(new ThemeService(darkColorTheme)),
    userThemeService: UnavailableUserThemeService,
  }));

  settings.open("appearance");
  const options = [...root.querySelectorAll<HTMLInputElement>("[data-theme-preference] input")];
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

  root.querySelector<HTMLInputElement>("[value='zeta-dark']")?.click();
  await Promise.resolve();
  assert.equal(configuration.getValue(WorkbenchConfiguration.colorTheme), "zeta-dark");

  root.querySelector<HTMLInputElement>("[value='zeta-test-aurora']")?.click();
  await Promise.resolve();
  assert.equal(configuration.getValue(WorkbenchConfiguration.colorTheme), "zeta-test-aurora");

  root.querySelector<HTMLInputElement>("[value='system']")?.click();
  await Promise.resolve();
  assert.equal(configuration.getValue(WorkbenchConfiguration.colorTheme), "system");
});

test("Appearance edits the resolved Light JSON, previews it, and saves a new theme", async () => {
  using disposables = new DisposableStore();
  const ownerDocument = browserEnvironment.window.document;
  ownerDocument.body.replaceChildren();
  const root = ownerDocument.createElement("div");
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
  assert.equal(opens, 1);

  await commands.executeCommand(OpenSettingsCommandId);
  assert.equal(opens, 2);
});
