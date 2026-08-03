import assert from "node:assert/strict";
import test from "node:test";
import {
  MenuId,
} from "../../../../platform/actions/common/actions.js";
import {
  MenuService,
} from "../../../../platform/actions/common/menuService.js";
import {
  ContextKeyService,
} from "../../../../platform/contextkey/common/contextkey.js";
import {
  ServiceCollection,
} from "../../../../platform/instantiation/common/instantiation.js";
import {
  NATIVE_HOST_OPEN_FOLDER_CHANNEL,
  NATIVE_HOST_SET_WINDOW_THEME_CHANNEL,
  NATIVE_HOST_TOGGLE_DEVELOPER_TOOLS_CHANNEL,
} from "../../../../platform/native/common/nativeHost.js";
import {
  nativeHostIpcRoutes,
} from "../../../../platform/native/electron-main/nativeHostIpc.js";
import {
  INativeHostService,
} from "../../../../workbench/common/services.js";
import {
  ToggleDeveloperToolsCommandId,
} from "../../../../workbench/electron-browser/actions/developerActions.js";
import { OpenFolderCommandId } from "../../../../workbench/electron-browser/actions/workspaceActions.js";
import "../../../../workbench/electron-browser/desktop.contribution.js";
import {
  CommandService,
} from "../../../../workbench/services/commands/common/commandService.js";
import { IWorkspaceOpenService } from "../../../../workbench/services/workspaces/browser/workspaceOpenService.js";

test("native host routes validate folder opening and developer tools", async () => {
  let folderOpens = 0;
  let toggles = 0;
  const windowThemes: unknown[] = [];
  const routes = nativeHostIpcRoutes({
    openFolder: async () => {
      folderOpens += 1;
    },
    setWindowTheme: (theme) => {
      windowThemes.push(theme);
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
  const setWindowTheme = routes.find(
    ({ channel }) => channel === NATIVE_HOST_SET_WINDOW_THEME_CHANNEL,
  );
  assert.ok(openFolder);
  assert.ok(setWindowTheme);
  assert.ok(toggleDeveloperTools);

  assert.throws(
    () => openFolder.validate(null),
    /does not accept parameters/,
  );
  await openFolder.invoke(openFolder.validate(undefined));
  assert.equal(folderOpens, 1);
  assert.throws(
    () => setWindowTheme.validate({ backgroundColor: "white", symbolColor: "#000000" }),
    /backgroundColor must be an opaque hexadecimal color/,
  );
  const validatedTheme = setWindowTheme.validate({
    backgroundColor: "#F3F3F3",
    symbolColor: "#424242",
  });
  setWindowTheme.invoke(validatedTheme);
  assert.deepEqual(windowThemes, [{
    backgroundColor: "#f3f3f3",
    symbolColor: "#424242",
  }]);
  assert.throws(
    () => toggleDeveloperTools.validate(null),
    /does not accept parameters/,
  );
  toggleDeveloperTools.invoke(
    toggleDeveloperTools.validate(undefined),
  );
  assert.equal(toggles, 1);
});

test("desktop commands are available from the command palette", async () => {
  const services = new ServiceCollection();
  let toggles = 0;
  services.set(INativeHostService, {
    openFolder: async () => {},
    setWindowTheme: async () => {},
    async toggleDeveloperTools() {
      toggles += 1;
    },
  });
  let folderOpens = 0;
  services.set(IWorkspaceOpenService, {
    canOpenFolder: true,
    async openFolder() {
      folderOpens += 1;
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

  const openFolder = paletteActions.find(
    ({ id }) => id === OpenFolderCommandId,
  );
  assert.equal(openFolder?.label, "Open Folder...");
  await openFolder?.run();
  assert.equal(folderOpens, 1);
});
