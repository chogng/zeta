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

test("native host route validates and toggles developer tools", () => {
  let toggles = 0;
  const [route] = nativeHostIpcRoutes({
    toggleDeveloperTools: () => {
      toggles += 1;
    },
  });

  assert.equal(
    route.channel,
    NATIVE_HOST_TOGGLE_DEVELOPER_TOOLS_CHANNEL,
  );
  assert.throws(() => route.validate(null), /does not accept parameters/);
  route.invoke(route.validate(undefined));
  assert.equal(toggles, 1);
});

test("developer tools command is available from the command palette", async () => {
  const services = new ServiceCollection();
  let toggles = 0;
  services.set(INativeHostService, {
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
