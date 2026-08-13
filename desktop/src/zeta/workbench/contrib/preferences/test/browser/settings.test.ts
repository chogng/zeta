import assert from "node:assert/strict";
import test from "node:test";
import { JSDOM } from "jsdom";
import type { ConfigReadResult, SemanticCodeIndexSelectionDto } from "../../../../../../../generated/app-server/types.js";

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

const { DisposableStore, toDisposable } = await import("../../../../../base/common/lifecycle.js");
const { KeybindingLabelStyle, getKeybindingLabel } = await import("../../../../../base/common/keybindingLabels.js");
const { resolveKeybinding } = await import("../../../../../base/common/keybindings.js");
const { lxiconsLibrary } = await import("../../../../../base/common/lxiconsLibrary.js");
const { OperatingSystem } = await import("../../../../../base/common/platform.js");
const { MenuId } = await import("../../../../../platform/actions/common/actions.js");
const { MenuService } = await import("../../../../../platform/actions/common/menuService.js");
const { ContextKeyService } = await import("../../../../../platform/contextkey/common/contextkey.js");
const { ServiceCollection } = await import("../../../../../platform/instantiation/common/instantiation.js");
const { KeybindingResolver } = await import("../../../../../platform/keybinding/common/keybindingResolver.js");
const { KeybindingsRegistry } = await import("../../../../../platform/keybinding/common/keybindingsRegistry.js");
const { createColorTheme, darkColorTheme, lightColorTheme } = await import("../../../../../platform/theme/common/colorTheme.js");
const { ThemeService } = await import("../../../../../platform/theme/common/themeService.js");
const { parseUserColorTheme } = await import("../../../../../platform/theme/common/userColorTheme.js");
const { ColorScheme } = await import("../../../../../platform/theme/common/theme.js");
const { ISettingsService } = await import("../../../../../workbench/services/preferences/common/settings.js");
const { SettingsService } = await import("../../../../../workbench/services/preferences/common/settingsService.js");
const { WorkbenchConfiguration } = await import("../../../../../workbench/common/configuration.js");
const { WorkbenchThemesRegistry } = await import("../../../../../workbench/common/theme.js");
const { UnavailableUserThemeService } = await import("../../../../../workbench/common/userThemes.js");
const { WorkbenchConfigurationService } = await import("../../../../../workbench/services/configuration/browser/configurationService.js");
const { SettingsEditorContribution } = await import("../../../../../workbench/contrib/preferences/browser/settingsEditor.contribution.js");
const { CommandService } = await import("../../../../../workbench/services/commands/common/commandService.js");
const { BrowserKeyboardLayoutService } = await import("../../../../../workbench/services/keybinding/browser/keyboardLayoutService.js");
const { WorkbenchKeybindingService } = await import("../../../../../workbench/services/keybinding/browser/keybindingService.js");
const { OpenSettingsCommandId } = await import("../../../../../workbench/contrib/preferences/browser/preferences.contribution.js");

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
      "Languages",
      "Agents",
      "Models",
      "Git",
      "Worktrees",
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

  navigationItems[7].click();
  assert.equal(settings.activeSectionId, "models");
  assert.equal(navigationItems[0].hasAttribute("aria-current"), false);
  assert.equal(navigationItems[7].getAttribute("aria-current"), "page");
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

test("Connector settings project catalog state and invoke typed connect and disconnect actions", async () => {
  using disposables = new DisposableStore();
  const ownerDocument = browserEnvironment.window.document;
  ownerDocument.body.replaceChildren();
  const root = ownerDocument.createElement("div");
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
    dialogService: acceptingDialogService,
    settingsService: settings,
    themeService: disposables.add(new ThemeService(darkColorTheme)),
    userThemeService: UnavailableUserThemeService,
  }));

  settings.open("connectors");
  await new Promise((resolve) => globalThis.setTimeout(resolve, 0));
  assert.deepEqual([...root.querySelectorAll(".zeta-integration-heading h4")].map(element => element.textContent), ["GitHub", "Slack"]);
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

test("Plugin settings project layered authority and send exact-package commands", async () => {
  using disposables = new DisposableStore();
  const ownerDocument = browserEnvironment.window.document;
  ownerDocument.body.replaceChildren();
  const root = ownerDocument.createElement("div");
  ownerDocument.body.append(root);
  const settings = disposables.add(new SettingsService());
  const configuration = disposables.add(new WorkbenchConfigurationService());
  const mutations: string[] = [];
  const plugin = { id: "acme/github", version: "1.0.0", digest: `sha256:${"a".repeat(64)}`, enabled: false, granted: false, effective: false, revoked: false };
  const pluginService = {
    onDidChange: () => toDisposable(() => {}),
    list: async () => ({ revision: 7, activationGeneration: 3, packages: [plugin] }),
    listMarketplace: async () => [],
    install: async () => {},
    update: async () => {},
    rollback: async () => {},
    enable: async (target: typeof plugin, revision: number) => { mutations.push(`enable:${target.id}:${target.digest}:${revision}`); },
    disable: async () => {},
    grant: async (target: typeof plugin, revision: number) => { mutations.push(`grant:${target.id}:${target.digest}:${revision}`); },
    revokeGrant: async () => {},
    uninstall: async (target: typeof plugin, revision: number) => { mutations.push(`uninstall:${target.id}:${target.digest}:${revision}`); },
  };
  disposables.add(new SettingsEditorContribution({
    configurationService: configuration,
    container: root,
    dialogService: acceptingDialogService,
    pluginService,
    settingsService: settings,
    themeService: disposables.add(new ThemeService(darkColorTheme)),
    userThemeService: UnavailableUserThemeService,
  }));

  settings.open("plugins");
  await new Promise((resolve) => globalThis.setTimeout(resolve, 0));
  assert.equal(root.querySelector(".zeta-integration-heading h4")?.textContent, "acme/github · 1.0.0");
  const buttons = [...root.querySelectorAll<HTMLButtonElement>(".zeta-integration-card > .zeta-theme-action")];
  assert.deepEqual(buttons.map(button => button.textContent), ["Grant", "Enable", "Uninstall"]);
  buttons[0]!.click();
  await new Promise((resolve) => globalThis.setTimeout(resolve, 0));
  assert.deepEqual(mutations, [`grant:acme/github:${plugin.digest}:7`]);
});

test("Plugin settings browse signed Marketplace metadata before installation", async () => {
  using disposables = new DisposableStore();
  const ownerDocument = browserEnvironment.window.document;
  ownerDocument.body.replaceChildren();
  const root = ownerDocument.createElement("div");
  ownerDocument.body.append(root);
  const settings = disposables.add(new SettingsService());
  const configuration = disposables.add(new WorkbenchConfigurationService());
  const mutations: string[] = [];
  const review = {
    marketplaceId: "zeta",
    marketplaceMode: "remoteManaged" as const,
    marketplaceTrust: "productManaged" as const,
    marketplaceRevision: "sha256:catalog",
    id: "chogng/code-review",
    publisher: "chogng",
    version: "1.0.0",
    digest: `sha256:${"b".repeat(64)}`,
    displayName: "Code Review",
    description: "Review workspace changes before they ship.",
    license: "Apache-2.0",
    compatibilityZeta: ">=0.1.0",
    contributions: { skills: 1, mcpServers: 0, connectors: 0, assets: 1, editorExtensions: 0, declarativeExtensions: 0 },
    permissions: [{ type: "workspace" as const, access: "read" as const }],
    credentialSlots: [],
    packageFileCount: 3,
    packageSizeBytes: 2048,
    installed: false,
    enabled: false,
    granted: false,
    effective: false,
    revoked: false,
  };
  const theme = {
    ...review,
    id: "chogng/theme-pack",
    version: "2.0.0",
    digest: `sha256:${"c".repeat(64)}`,
    displayName: "Theme Pack",
    description: "Static visual assets.",
    contributions: { skills: 0, mcpServers: 0, connectors: 0, assets: 4, editorExtensions: 0, declarativeExtensions: 1 },
    permissions: [],
    packageFileCount: 5,
    packageSizeBytes: 1024,
  };
  const pluginService = {
    onDidChange: () => toDisposable(() => {}),
    list: async () => ({ revision: 4, activationGeneration: 0, packages: [] }),
    listMarketplace: async () => [review, theme],
    install: async (target: typeof review, revision: number) => { mutations.push(`install:${target.id}:${target.digest}:${revision}`); },
    update: async () => {},
    rollback: async () => {},
    enable: async () => {},
    disable: async () => {},
    grant: async () => {},
    revokeGrant: async () => {},
    uninstall: async () => {},
  };
  disposables.add(new SettingsEditorContribution({
    configurationService: configuration,
    container: root,
    dialogService: acceptingDialogService,
    pluginService,
    settingsService: settings,
    themeService: disposables.add(new ThemeService(darkColorTheme)),
    userThemeService: UnavailableUserThemeService,
  }));

  settings.open("plugins");
  await new Promise((resolve) => globalThis.setTimeout(resolve, 0));
  assert.deepEqual([...root.querySelectorAll(".zeta-marketplace-results h4")].map(element => element.textContent), ["Code Review", "Theme Pack"]);
  assert.equal(root.querySelector(".zeta-marketplace-badge")?.textContent, "Zeta managed");
  assert.match(root.querySelector(".zeta-marketplace-access p")?.textContent ?? "", /read workspace files/);
  assert.match(root.querySelector(".zeta-marketplace-details")?.textContent ?? "", /2.0 KB/);

  const search = root.querySelector<HTMLInputElement>(".zeta-marketplace-search input")!;
  search.value = "theme";
  search.dispatchEvent(new browserEnvironment.window.Event("input", { bubbles: true }));
  assert.deepEqual([...root.querySelectorAll(".zeta-marketplace-results h4")].map(element => element.textContent), ["Theme Pack"]);
  search.value = "review";
  search.dispatchEvent(new browserEnvironment.window.Event("input", { bubbles: true }));
  root.querySelector<HTMLButtonElement>(".zeta-marketplace-results .zeta-theme-action")?.click();
  await new Promise((resolve) => globalThis.setTimeout(resolve, 0));
  assert.deepEqual(mutations, [`install:${review.id}:${review.digest}:4`]);
});

test("Language settings require confirmation and install the exact signed catalog entry", async () => {
  using disposables = new DisposableStore();
  const ownerDocument = browserEnvironment.window.document;
  ownerDocument.body.replaceChildren();
  const root = ownerDocument.createElement("div");
  ownerDocument.body.append(root);
  const settings = disposables.add(new SettingsService());
  const configuration = disposables.add(new WorkbenchConfigurationService());
  const digest = `sha256:${"d".repeat(64)}`;
  const css = {
    marketplaceId: "zeta",
    packageId: "marketplace/css",
    version: "1.0.0",
    digest,
    displayName: "CSS Language Support",
    description: "CSS, SCSS, and Less language support.",
    license: "MIT",
    serverId: "css-language-server",
    languages: ["css", "scss", "less"],
    fileExtensions: [".css", ".scss", ".less"],
    compatibility: { status: "compatible" as const },
    installed: false,
    active: false,
  };
  const native = {
    ...css,
    packageId: "marketplace/native-css",
    displayName: "Native CSS",
    compatibility: { status: "incompatible" as const, reason: "Native provider unavailable." },
  };
  const installs: string[] = [];
  const confirmations: Array<{ message: string; detail?: string }> = [];
  const languageMarketplaceService = {
    list: async () => ({ revision: "signed-catalog:7", activationGeneration: 3, entries: [css, native] }),
    install: async (entry: { marketplaceId: string; packageId: string; version: string; digest: string; serverId: string }, revision: string) => {
      installs.push(`${entry.marketplaceId}:${entry.packageId}:${entry.version}:${entry.digest}:${entry.serverId}:${revision}`);
    },
  };
  disposables.add(new SettingsEditorContribution({
    configurationService: configuration,
    container: root,
    dialogService: {
      showMessage: async () => {},
      confirm: async (options) => {
        confirmations.push({ message: options.message, detail: options.detail });
        return true;
      },
    },
    languageMarketplaceService,
    settingsService: settings,
    themeService: disposables.add(new ThemeService(darkColorTheme)),
    userThemeService: UnavailableUserThemeService,
  }));

  settings.open("languages");
  await new Promise((resolve) => globalThis.setTimeout(resolve, 0));
  assert.deepEqual([...root.querySelectorAll(".zeta-integration-heading h4")].map(element => element.textContent), ["CSS Language Support", "Native CSS"]);
  assert.equal(root.querySelectorAll(".zeta-theme-action").length, 1);
  assert.match(root.textContent ?? "", /Native provider unavailable/);
  root.querySelector<HTMLButtonElement>(".zeta-theme-action")?.click();
  await new Promise((resolve) => globalThis.setTimeout(resolve, 0));

  assert.equal(confirmations.length, 1);
  assert.match(confirmations[0]?.message ?? "", /CSS Language Support 1\.0\.0/);
  assert.match(confirmations[0]?.detail ?? "", new RegExp(digest));
  assert.match(confirmations[0]?.detail ?? "", /shared Node-compatible runtime/);
  assert.deepEqual(installs, [`zeta:marketplace/css:1.0.0:${digest}:css-language-server:signed-catalog:7`]);
});

test("Appearance settings persist and dynamically render registered theme preferences", async () => {
  using disposables = new DisposableStore();
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

test("Indexing settings save Tool Search and semantic model consent configuration", async () => {
  using disposables = new DisposableStore();
  const ownerDocument = browserEnvironment.window.document;
  ownerDocument.body.replaceChildren();
  const root = ownerDocument.createElement("div");
  ownerDocument.body.append(root);
  const settings = disposables.add(new SettingsService());
  const configuration = disposables.add(new WorkbenchConfigurationService());
  const config = {
    revision: 4,
    generation: 4,
    preferredModel: null,
    approvalReviewModel: { type: "automatic" },
    providers: {
      ollama: {
        provider: "ollama",
        baseUrl: "http://localhost:11434/v1",
        maxOutputTokens: null,
        modelContext: {},
      },
    },
    mcpServers: {},
    skillSources: {},
    pluginRequests: {},
    hooks: {},
    languageServers: {},
    execPolicyRules: [],
    toolSearch: {
      mode: "hybridEmbedding",
      embeddingModel: { provider: "ollama", model: "nomic-embed-text" },
      embeddingStatus: {
        type: "unavailable",
        model: { provider: "ollama", model: "nomic-embed-text" },
        reason: "connection refused",
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
  } as const satisfies ConfigReadResult;
  const configured: Array<{ mode: string; embeddingModel?: { provider: string; model: string }; revision: number }> = [];
  const configuredProviders: Array<{ provider: string; baseUrl: string | null; revision: number }> = [];
  const configuredSemantic: Array<{ selection: SemanticCodeIndexSelectionDto; automaticContext: string; revision: number }> = [];
  let authorizations = 0;
  disposables.add(new SettingsEditorContribution({
    configurationService: configuration,
    container: root,
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
      readConfig: async () => ({ revision: 4, ...config.toolSearch }),
      configure: async (next, revision) => { configured.push({ ...next, revision }); },
    },
  }));

  settings.open("indexing");
  await new Promise((resolve) => globalThis.setTimeout(resolve, 0));

  assert.deepEqual(
    [...root.querySelectorAll(".zeta-indexing-setting legend")].map((legend) => legend.textContent),
    ["Agent tool search", "Semantic code search"],
  );
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
