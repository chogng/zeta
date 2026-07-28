import assert from "node:assert/strict";
import test from "node:test";
import {
  Keybinding,
  logicalKey,
  resolveKeybinding,
} from "../src/base/common/keybindings.js";
import { DisposableStore } from "../src/base/common/lifecycle.js";
import { OperatingSystem } from "../src/base/common/platform.js";
import {
  Action2,
  MenuId,
  MenusRegistry,
  registerAction2,
  SubmenuItemAction,
} from "../src/platform/actions/common/actions.js";
import {
  MenuService,
} from "../src/platform/actions/common/menuService.js";
import {
  CommandsRegistry,
  CommandService,
} from "../src/platform/commands/common/command-registry.js";
import {
  ContextKeyExpr,
  ContextKeyService,
} from "../src/platform/contextkey/common/contextkey.js";
import {
  createServiceIdentifier,
  ServiceCollection,
  type ServicesAccessor,
} from "../src/platform/instantiation/common/instantiation.js";
import {
  KeybindingResolver,
} from "../src/platform/keybinding/common/keybindingResolver.js";
import {
  KeybindingsRegistry,
} from "../src/platform/keybinding/common/keybindingsRegistry.js";

test("registerAction2 connects command execution and menu placement", async () => {
  using registrations = new DisposableStore();
  const menuId = new MenuId("test.actions.registration");
  const serviceId = createServiceIdentifier<string>("testValue");

  class RegisteredAction extends Action2 {
    constructor() {
      super({
        id: "test.actions.registered",
        title: "Registered action",
        menu: {
          id: menuId,
          group: "navigation",
          order: 10,
        },
        keybinding: {
          primary: Keybinding.single(logicalKey("r", {
            ctrlKey: true,
          })),
        },
        f1: true,
      });
    }

    override run(
      accessor: ServicesAccessor,
      ...args: readonly unknown[]
    ): string {
      return `${accessor.get(serviceId)}:${String(args[0])}`;
    }
  }

  registrations.add(registerAction2(RegisteredAction));
  const services = new ServiceCollection();
  services.set(serviceId, "service");
  const commands = new CommandService(services);
  const contexts = registrations.add(new ContextKeyService());
  const menus = new MenuService(commands, contexts);
  assert.ok(new KeybindingResolver({
    registry: KeybindingsRegistry,
    resolveKeybinding: (keybinding) =>
      resolveKeybinding(keybinding, OperatingSystem.Windows),
  }).lookupKeybinding("test.actions.registered", contexts));

  const groups = menus.getMenuActions(menuId, {
    shouldForwardArgs: true,
  });
  assert.equal(groups.length, 1);
  assert.equal(groups[0][0], "navigation");
  assert.equal(groups[0][1][0].label, "Registered action");
  assert.equal(await groups[0][1][0].run("argument"), "service:argument");

  const paletteIds = menus.getMenuActions(MenuId.CommandPalette)
    .flatMap(([, actions]) => actions)
    .map((action) => action.id);
  assert.ok(paletteIds.includes("test.actions.registered"));
});

test("menu actions react to visibility, enablement, and toggle context", () => {
  using registrations = new DisposableStore();
  const menuId = new MenuId("test.actions.context");
  const commandId = "test.actions.contextual";

  registrations.add(CommandsRegistry.register(commandId, () => undefined));
  registrations.add(MenusRegistry.appendMenuItem(menuId, {
    command: {
      id: commandId,
      title: "Contextual action",
      precondition: ContextKeyExpr.has("test.ready"),
      toggled: ContextKeyExpr.has("test.active"),
    },
    when: ContextKeyExpr.has("test.visible"),
  }));

  const services = new ServiceCollection();
  const commands = new CommandService(services);
  const contexts = registrations.add(new ContextKeyService());
  const menus = new MenuService(commands, contexts);
  const menu = registrations.add(menus.createMenu(menuId));
  let changes = 0;
  registrations.add(menu.onDidChange(() => {
    changes += 1;
  }));

  assert.deepEqual(menu.getActions(), []);

  contexts.setContext("test.visible", true);
  let action = menu.getActions()[0][1][0];
  assert.equal(action.enabled, false);
  assert.equal(action.checked, false);

  contexts.setContext("test.ready", true);
  contexts.setContext("test.active", true);
  action = menu.getActions()[0][1][0];
  assert.equal(action.enabled, true);
  assert.equal(action.checked, true);
  assert.equal(changes, 3);
});

test("menu service sorts groups and resolves submenus", () => {
  using registrations = new DisposableStore();
  const rootMenu = new MenuId("test.actions.root");
  const childMenu = new MenuId("test.actions.child");
  const commandIds = [
    "test.actions.navigation",
    "test.actions.first",
    "test.actions.second",
  ] as const;

  for (const commandId of commandIds) {
    registrations.add(CommandsRegistry.register(commandId, () => undefined));
  }

  registrations.add(MenusRegistry.appendMenuItem(rootMenu, {
    command: {
      id: commandIds[1],
      title: "First",
    },
    group: "primary",
    order: 1,
  }));
  registrations.add(MenusRegistry.appendMenuItem(rootMenu, {
    command: {
      id: commandIds[0],
      title: "Navigation",
    },
    group: "navigation",
    order: 100,
  }));
  registrations.add(MenusRegistry.appendMenuItem(childMenu, {
    command: {
      id: commandIds[2],
      title: "Second",
    },
  }));
  registrations.add(MenusRegistry.appendMenuItem(rootMenu, {
    title: "More",
    submenu: childMenu,
    group: "primary",
    order: 2,
  }));

  const commands = new CommandService(new ServiceCollection());
  const contexts = registrations.add(new ContextKeyService());
  const groups = new MenuService(commands, contexts)
    .getMenuActions(rootMenu);

  assert.deepEqual(groups.map(([group]) => group), [
    "navigation",
    "primary",
  ]);
  assert.equal(groups[1][1][0].label, "First");
  assert.ok(groups[1][1][1] instanceof SubmenuItemAction);
  assert.equal(
    (groups[1][1][1] as SubmenuItemAction).actions[0].label,
    "Second",
  );
});
