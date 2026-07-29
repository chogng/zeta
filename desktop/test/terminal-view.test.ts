import assert from "node:assert/strict";
import test from "node:test";
import { JSDOM } from "jsdom";
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

const contextMenuService: IContextMenuService = {
  onDidShowContextMenu: noEvent,
  onDidHideContextMenu: noEvent,
  showContextMenu() {},
  hideContextMenu() {},
};

test("Terminal title actions resolve through MenuService and preserve the profile control", async () => {
  const ownerDocument = browserEnvironment.window.document;
  ownerDocument.body.replaceChildren();
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
  const profile = toolbar.querySelector<HTMLSelectElement>(".zeta-terminal-profile");
  assert.ok(profile);
  assert.deepEqual([...profile.options].map((option) => option.value), ["cmd", "pwsh"]);
  const newTerminal = [...toolbar.querySelectorAll("button")].find((button) => button.textContent === "New Terminal");
  assert.ok(newTerminal);
  newTerminal.click();
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

  const currentProfile = toolbar.querySelector<HTMLSelectElement>(".zeta-terminal-profile");
  assert.ok(currentProfile);
  currentProfile.value = "pwsh";
  currentProfile.dispatchEvent(new browserEnvironment.window.Event("change", { bubbles: true }));
  await Promise.resolve();
  const nextNewTerminal = [...toolbar.querySelectorAll("button")].find((button) => button.textContent === "New Terminal");
  assert.ok(nextNewTerminal);
  nextNewTerminal.click();
  await Promise.resolve();
  assert.deepEqual(selectedProfiles, ["cmd", "pwsh"]);
});
