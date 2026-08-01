import assert from "node:assert/strict";
import test from "node:test";
import { JSDOM } from "jsdom";
import type { IAction } from "../src/zeta/base/common/actions.js";
import type { Event } from "../src/zeta/base/common/event.js";
import { toDisposable } from "../src/zeta/base/common/lifecycle.js";
import type { IContextMenuService } from "../src/zeta/platform/contextview/browser/contextMenu.js";
import type { ITerminalInstance } from "../src/zeta/workbench/services/terminal/common/terminal.js";

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
  { ServiceCollection },
  { CommandService },
  { TerminalTitleActions },
] = await Promise.all([
  import("../src/zeta/platform/contextkey/common/contextkey.js"),
  import("../src/zeta/platform/actions/common/menuService.js"),
  import("../src/zeta/platform/instantiation/common/instantiation.js"),
  import("../src/zeta/workbench/services/commands/common/commandService.js"),
  import("../src/zeta/workbench/contrib/terminal/browser/view/terminalTitleActions.js"),
]);

test.after(() => {
  browserEnvironment.window.close();
  for (const name of ["window", "document", "Node", "Element", "HTMLElement", "Event", "MouseEvent", "navigator"]) {
    Reflect.deleteProperty(globalThis, name);
  }
});

const noEvent = (() => toDisposable(() => {})) as Event<never>;
let shownProfileActions: readonly IAction[] = [];

const contextMenuService: IContextMenuService = {
  onDidShowContextMenu: noEvent,
  onDidHideContextMenu: noEvent,
  showContextMenu(options) {
    shownProfileActions = "actions" in options ? options.actions : [];
  },
  hideContextMenu() {},
};

test("Terminal title actions resolve through MenuService and preserve the profile control", async () => {
  const ownerDocument = browserEnvironment.window.document;
  ownerDocument.body.replaceChildren();
  shownProfileActions = [];
  const selectedProfiles: Array<string | undefined> = [];
  using contextKeyService = new ContextKeyService();
  const commandService = new CommandService(new ServiceCollection());
  const menuService = new MenuService(commandService, contextKeyService);
  using titleActions = new TerminalTitleActions({
    ownerDocument,
    menuService,
    contextMenuService,
    contextKeyService,
    createTerminal: () => {
      selectedProfiles.push(titleActions.selectedProfileId);
    },
    relaunchActive() {},
    killActive() {},
  });
  titleActions.setProfiles([
      { profileId: "cmd", title: "Command Prompt", isDefault: true },
      { profileId: "pwsh", title: "PowerShell", isDefault: false },
  ]);
  ownerDocument.body.append(titleActions.element);

  const toolbar = titleActions.element;
  assert.equal(toolbar.getAttribute("role"), "toolbar");
  assert.equal(toolbar.classList.contains("highlight-toggled"), true);
  const profile = toolbar.querySelector<HTMLButtonElement>(".zeta-terminal-profile-action .zeta-button");
  assert.ok(profile);
  assert.equal(profile.querySelector(".zeta-button-label")?.textContent, "Command Prompt");
  assert.ok(profile.querySelector("svg.zeta-icon"));
  const newTerminal = [...toolbar.querySelectorAll("button")].find((button) => button.textContent === "New Terminal");
  assert.ok(newTerminal);
  const closePanel = [...toolbar.querySelectorAll("button")].find((button) => button.textContent === "Close Panel");
  assert.ok(closePanel);
  assert.ok(closePanel.querySelector("svg.zeta-icon"));
  const maximizePanel = [...toolbar.querySelectorAll("button")].find((button) => button.textContent === "Maximize Panel");
  assert.ok(maximizePanel);
  assert.ok(maximizePanel.querySelector("svg.zeta-icon"));
  assert.equal(maximizePanel.compareDocumentPosition(closePanel) & browserEnvironment.window.Node.DOCUMENT_POSITION_FOLLOWING, browserEnvironment.window.Node.DOCUMENT_POSITION_FOLLOWING);
  contextKeyService.setContext("editorAreaVisible", false);
  const restoreEditorArea = [...toolbar.querySelectorAll("button")].find((button) => button.textContent === "Restore Editor Area");
  assert.ok(restoreEditorArea);
  assert.equal(restoreEditorArea.classList.contains("checked"), true);
  contextKeyService.setContext("editorAreaVisible", true);
  const currentNewTerminal = [...toolbar.querySelectorAll("button")].find((button) => button.textContent === "New Terminal");
  assert.ok(currentNewTerminal);
  currentNewTerminal.click();
  await Promise.resolve();
  assert.deepEqual(selectedProfiles, ["cmd"]);
  titleActions.setCreating(true);
  assert.equal([...toolbar.querySelectorAll("button")].find((button) => button.textContent === "New Terminal")?.disabled, true);
  titleActions.setCreating(false);
  titleActions.setActiveInstance({ state: "running" } as ITerminalInstance);
  assert.equal(toolbar.textContent?.includes("Kill Terminal"), true);
  assert.equal(toolbar.textContent?.includes("Relaunch Terminal"), false);
  titleActions.setActiveInstance({ state: "exited" } as ITerminalInstance);
  assert.equal(toolbar.textContent?.includes("Relaunch Terminal"), true);

  const currentProfile = toolbar.querySelector<HTMLButtonElement>(".zeta-terminal-profile-action .zeta-button");
  assert.ok(currentProfile);
  currentProfile.click();
  const powerShell = shownProfileActions.find((action) => action.label === "PowerShell");
  assert.ok(powerShell);
  await powerShell.run();
  assert.equal(toolbar.querySelector(".zeta-terminal-profile-action .zeta-button-label")?.textContent, "PowerShell");
  const nextNewTerminal = [...toolbar.querySelectorAll("button")].find((button) => button.textContent === "New Terminal");
  assert.ok(nextNewTerminal);
  nextNewTerminal.click();
  await Promise.resolve();
  assert.deepEqual(selectedProfiles, ["cmd", "pwsh"]);
});
