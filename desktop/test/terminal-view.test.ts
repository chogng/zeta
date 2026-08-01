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

test("Terminal title actions separate active identity from new-terminal profile selection", async () => {
  const ownerDocument = browserEnvironment.window.document;
  ownerDocument.body.replaceChildren();
  shownProfileActions = [];
  const selectedProfiles: Array<string | undefined> = [];
  let focusCount = 0;
  let clearCount = 0;
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
    focusActive: () => focusCount++,
    relaunchActive() {},
    killActive() {},
    clearActive: () => clearCount++,
  });
  const commandPromptProfile = { profileId: "cmd", title: "Command Prompt", isDefault: true };
  const powerShellProfile = { profileId: "pwsh", title: "PowerShell", isDefault: false };
  titleActions.setProfiles([commandPromptProfile, powerShellProfile]);
  titleActions.setSupplementalSecondaryActions([{
    id: "zeta.compositeBar.open.panel.output",
    label: "Output",
    tooltip: "Output",
    enabled: true,
    run() {},
  }]);
  ownerDocument.body.append(titleActions.element);

  const toolbar = titleActions.element;
  assert.equal(toolbar.getAttribute("role"), "toolbar");
  assert.equal(toolbar.classList.contains("highlight-toggled"), true);
  const profile = toolbar.querySelector<HTMLButtonElement>(".zeta-terminal-profile-action .zeta-button");
  assert.ok(profile);
  assert.equal(profile.querySelector(".zeta-button-label")?.textContent, "New Terminal Profile");
  assert.equal(profile.getAttribute("aria-label"), "New terminal profile: Command Prompt");
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
  const currentMaximizePanel = toolbar.querySelector<HTMLElement>("[data-action-id='workbench.action.toggleMaximizedPanel']");
  assert.ok(killTerminal);
  assert.ok(moreActions);
  assert.ok(currentMaximizePanel);
  assert.equal(killTerminal.compareDocumentPosition(moreActions) & browserEnvironment.window.Node.DOCUMENT_POSITION_FOLLOWING, browserEnvironment.window.Node.DOCUMENT_POSITION_FOLLOWING);
  assert.equal(moreActions.compareDocumentPosition(currentMaximizePanel) & browserEnvironment.window.Node.DOCUMENT_POSITION_FOLLOWING, browserEnvironment.window.Node.DOCUMENT_POSITION_FOLLOWING);
  moreActions.querySelector("button")?.click();
  const clearTerminal = shownProfileActions.find((action) => action.id === "zeta.terminal.clear");
  assert.ok(clearTerminal);
  assert.ok(shownProfileActions.some((action) => action.id === "zeta.compositeBar.open.panel.output"));
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

  const currentProfile = toolbar.querySelector<HTMLButtonElement>(".zeta-terminal-profile-action .zeta-button");
  assert.ok(currentProfile);
  currentProfile.click();
  const powerShell = shownProfileActions.find((action) => action.label === "PowerShell");
  assert.ok(powerShell);
  await powerShell.run();
  assert.equal(toolbar.querySelector(".zeta-terminal-profile-action .zeta-button")?.getAttribute("aria-label"), "New terminal profile: PowerShell");
  const nextNewTerminal = [...toolbar.querySelectorAll("button")].find((button) => button.textContent === "New Terminal");
  assert.ok(nextNewTerminal);
  nextNewTerminal.click();
  await Promise.resolve();
  assert.deepEqual(selectedProfiles, ["cmd", "pwsh"]);
});
