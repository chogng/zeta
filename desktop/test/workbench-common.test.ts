import assert from "node:assert/strict";
import test from "node:test";
import { toDisposable } from "../src/base/common/lifecycle.js";
import { URI } from "../src/base/common/uri.js";
import {
  ConfigurationsRegistry,
} from "../src/platform/configuration/common/configurationRegistry.js";
import {
  ContextKeyService,
} from "../src/platform/contextkey/common/contextkey.js";
import {
  createServiceIdentifier,
  ServiceCollection,
  SyncDescriptor,
} from "../src/platform/instantiation/common/instantiation.js";
import {
  darkColorTheme,
  lightColorTheme,
} from "../src/platform/theme/common/colorTheme.js";
import {
  type IWorkspaceContextService,
  WorkbenchState,
} from "../src/platform/workspace/common/workspace.js";
import {
  bindWorkbenchContextKeys,
  getVisibleViewContextKey,
} from "../src/workbench/common/contextkeys.js";
import {
  WorkbenchContributionRegistry,
  WorkbenchPhase,
} from "../src/workbench/common/contributions.js";
import { WorkbenchConfiguration } from "../src/workbench/common/configuration.js";
import { DialogsModel } from "../src/workbench/common/dialogs.js";
import {
  getWorkbenchColorTheme,
  WorkbenchThemeRegistry,
} from "../src/workbench/common/theme.js";
import {
  type IView,
  ViewContainerLocation,
  WorkbenchViewRegistry,
} from "../src/workbench/common/views.js";
import {
  DialogResult,
  DialogSeverity,
} from "../src/platform/dialogs/common/dialogs.js";

test("workbench context keys describe the current workspace", () => {
  using contextKeys = new ContextKeyService();
  const workspace: IWorkspaceContextService = {
    getWorkbenchState: () => WorkbenchState.FOLDER,
    getWorkspace: () => ({
      id: "workspace",
      folders: [{
        index: 0,
        name: "project",
        uri: URI.file("C:\\project"),
      }],
    }),
  };
  using bindings = bindWorkbenchContextKeys(contextKeys, workspace);

  assert.equal(contextKeys.getValue("workbenchState"), "folder");
  assert.equal(contextKeys.getValue("workspaceFolderCount"), 1);
  assert.equal(contextKeys.getValue("sideBarVisible"), true);
  assert.equal(
    getVisibleViewContextKey("zeta.explorer"),
    "view.zeta.explorer.visible",
  );
});

test("workbench contributions start once at their declared phases", () => {
  const serviceId = createServiceIdentifier<string>("testService");
  const services = new ServiceCollection();
  services.set(serviceId, "ready");
  const registry = new WorkbenchContributionRegistry();
  const calls: string[] = [];
  using startupRegistration = registry.register(
    "test.startup",
    WorkbenchPhase.BlockStartup,
    (accessor) => {
      calls.push(`startup:${accessor.get(serviceId)}`);
      return toDisposable(() => calls.push("dispose:startup"));
    },
  );
  using restoredRegistration = registry.register(
    "test.restored",
    WorkbenchPhase.AfterRestored,
    () => {
      calls.push("restored");
      return toDisposable(() => calls.push("dispose:restored"));
    },
  );

  {
    using host = registry.createHost(services);
    host.advance(WorkbenchPhase.BlockStartup);
    host.advance(WorkbenchPhase.BlockRestore);
    host.advance(WorkbenchPhase.AfterRestored);
    host.advance(WorkbenchPhase.AfterRestored);
    assert.deepEqual(calls, ["startup:ready", "restored"]);
  }
  assert.deepEqual(calls, [
    "startup:ready",
    "restored",
    "dispose:restored",
    "dispose:startup",
  ]);
});

test("workbench configuration resolves registered color themes", () => {
  assert.equal(
    ConfigurationsRegistry.owns(WorkbenchConfiguration.colorTheme),
    true,
  );
  assert.equal(
    WorkbenchConfiguration.colorTheme.defaultValue,
    darkColorTheme.id,
  );
  assert.equal(
    WorkbenchConfiguration.colorTheme.parse(lightColorTheme.id),
    lightColorTheme.id,
  );
  assert.throws(
    () => WorkbenchConfiguration.colorTheme.parse("missing-theme"),
    /Unknown workbench color theme/,
  );
  assert.equal(
    getWorkbenchColorTheme(lightColorTheme.id),
    lightColorTheme,
  );
});

test("workbench theme registries reject duplicate themes", () => {
  const registry = new WorkbenchThemeRegistry([darkColorTheme]);
  assert.equal(registry.getColorTheme(darkColorTheme.id), darkColorTheme);
  assert.throws(
    () => registry.registerColorTheme(darkColorTheme),
    /already registered/,
  );
  using registration = registry.registerColorTheme(lightColorTheme);
  assert.deepEqual(
    registry.getColorThemes().map((theme) => theme.id),
    [darkColorTheme.id, lightColorTheme.id],
  );
});

test("dialogs model publishes and settles renderer items", async () => {
  using model = new DialogsModel();
  const events: string[] = [];
  using willShow = model.onWillShowDialog(
    (item) => events.push(`show:${item.request.kind}`),
  );
  using didClose = model.onDidCloseDialog(
    (event) => events.push(
      event.kind === "result"
        ? `close:${event.result}`
        : "close:error",
    ),
  );
  const handle = model.show({
    kind: "message",
    severity: DialogSeverity.Info,
    message: "Saved",
  });

  assert.equal(model.dialogs.length, 1);
  handle.item.close(DialogResult.Primary);
  assert.equal(await handle.result, DialogResult.Primary);
  assert.equal(model.dialogs.length, 0);
  assert.deepEqual(events, ["show:message", "close:primary"]);
});

test("view registrations are ordered and disposed atomically", () => {
  const registry = new WorkbenchViewRegistry();
  const changes: string[] = [];
  using registered = registry.onDidRegisterViews(
    (event) => changes.push(
      `add:${event.views.map((view) => view.id).join(",")}`,
    ),
  );
  using removed = registry.onDidDeregisterViews(
    (event) => changes.push(
      `remove:${event.views.map((view) => view.id).join(",")}`,
    ),
  );
  using container = registry.registerViewContainer({
    id: "zeta.sidebar",
    title: "Navigation",
    location: ViewContainerLocation.Sidebar,
  });
  using views = registry.registerViews("zeta.sidebar", [
    {
      id: "zeta.search",
      title: "Search",
      order: 20,
      ctorDescriptor: new SyncDescriptor(TestView, {
        staticArguments: ["zeta.search"],
      }),
    },
    {
      id: "zeta.explorer",
      title: "Explorer",
      order: 10,
      ctorDescriptor: new SyncDescriptor(TestView, {
        staticArguments: ["zeta.explorer"],
      }),
    },
  ]);

  assert.deepEqual(
    registry.getViews("zeta.sidebar").map((view) => view.id),
    ["zeta.explorer", "zeta.search"],
  );
  assert.equal(
    registry.getViewContainerForView("zeta.explorer")?.id,
    "zeta.sidebar",
  );
  assert.throws(
    () => registry.registerViews("zeta.sidebar", [
      {
        id: "zeta.explorer",
        title: "Duplicate",
        ctorDescriptor: new SyncDescriptor(TestView, {
          staticArguments: ["zeta.explorer"],
        }),
      },
    ]),
    /already registered/,
  );
  assert.deepEqual(
    registry.getViews("zeta.sidebar").map((view) => view.id),
    ["zeta.explorer", "zeta.search"],
  );

  views.dispose();
  assert.deepEqual(changes, [
    "add:zeta.explorer,zeta.search",
    "remove:zeta.explorer,zeta.search",
  ]);
});

class TestView implements IView {
  #visible = true;

  constructor(readonly id: string) {}

  focus(): void {}

  isVisible(): boolean {
    return this.#visible;
  }

  setVisible(visible: boolean): void {
    this.#visible = visible;
  }
}
