import assert from "node:assert/strict";
import test from "node:test";
import {
  MenuId,
} from "../src/zeta/platform/actions/common/actions.js";
import {
  MenuService,
} from "../src/zeta/platform/actions/common/menuService.js";
import {
  ContextKeyService,
} from "../src/zeta/platform/contextkey/common/contextkey.js";
import {
  ServiceCollection,
} from "../src/zeta/platform/instantiation/common/instantiation.js";
import {
  NATIVE_HOST_OPEN_FOLDER_CHANNEL,
  NATIVE_HOST_TOGGLE_DEVELOPER_TOOLS_CHANNEL,
} from "../src/zeta/platform/native/common/nativeHost.js";
import {
  nativeHostIpcRoutes,
} from "../src/zeta/platform/native/electron-main/nativeHostIpc.js";
import {
  INativeHostService,
} from "../src/zeta/workbench/common/services.js";
import {
  ToggleDeveloperToolsCommandId,
} from "../src/zeta/workbench/electron-browser/actions/developerActions.js";
import "../src/zeta/workbench/electron-browser/desktop.contribution.js";
import {
  CommandService,
} from "../src/zeta/workbench/services/commands/common/commandService.js";

test("native host routes validate folder opening and developer tools", async () => {
  let folderOpens = 0;
  let toggles = 0;
  const routes = nativeHostIpcRoutes({
    openFolder: async () => {
      folderOpens += 1;
    },
    toggleDeveloperTools: () => {
      toggles += 1;
    },
  });
  const openFolder = routes.find(
    ({ channel }) => channel === NATIVE_HOST_OPEN_FOLDER_CHANNEL,
  );
  const toggleDeveloperTools = routes.find(
    ({ channel }) =>
      channel === NATIVE_HOST_TOGGLE_DEVELOPER_TOOLS_CHANNEL,
  );
  assert.ok(openFolder);
  assert.ok(toggleDeveloperTools);

  assert.throws(
    () => openFolder.validate(null),
    /does not accept parameters/,
  );
  await openFolder.invoke(openFolder.validate(undefined));
  assert.equal(folderOpens, 1);
  assert.throws(
    () => toggleDeveloperTools.validate(null),
    /does not accept parameters/,
  );
  toggleDeveloperTools.invoke(
    toggleDeveloperTools.validate(undefined),
  );
  assert.equal(toggles, 1);
});

test("developer tools command is available from the command palette", async () => {
  const services = new ServiceCollection();
  let toggles = 0;
  services.set(INativeHostService, {
    openFolder: async () => {},
    async toggleDeveloperTools() {
      toggles += 1;
    },
  });
  using commands = new CommandService(services);
  using contexts = new ContextKeyService();
  const paletteActions = new MenuService(commands, contexts)
    .getMenuActions(MenuId.CommandPalette)
    .flatMap(([, actions]) => actions);

  const action = paletteActions.find(
    ({ id }) => id === ToggleDeveloperToolsCommandId,
  );
  assert.equal(action?.label, "Developer: Toggle Developer Tools");
  await action?.run();
  assert.equal(toggles, 1);
});
