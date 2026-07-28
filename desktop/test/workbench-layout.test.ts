import assert from "node:assert/strict";
import test from "node:test";
import { JSDOM } from "jsdom";
import { DisposableStore } from "../src/zeta/base/common/lifecycle.js";
import {
  ContextKeyService,
} from "../src/zeta/platform/contextkey/common/contextkey.js";

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

const { Dimension } = await import("../src/zeta/base/browser/geometry.js");
const { LxIcon } = await import("../src/zeta/base/common/lxicons.js");
const { MenuId } = await import(
  "../src/zeta/platform/actions/common/actions.js"
);
const { MenuService } = await import(
  "../src/zeta/platform/actions/common/menuService.js"
);
const {
  IWorkbenchLayoutService,
  WorkbenchLayout,
  workbenchPartIds,
} = await import("../src/zeta/workbench/browser/layout.js");
const {
  bindWorkbenchPartVisibilityContextKeys,
} = await import("../src/zeta/workbench/browser/contextkeys.js");
const { WorkbenchPart } = await import("../src/zeta/workbench/browser/part.js");
const { ActivitybarPart } = await import(
  "../src/zeta/workbench/browser/parts/activitybar/activitybarPart.js"
);
const { SidebarPart } = await import(
  "../src/zeta/workbench/browser/parts/sidebar/sidebarPart.js"
);
const { EditorPart } = await import(
  "../src/zeta/workbench/browser/parts/editor/editorPart.js"
);
const { ViewDescriptorService } = await import(
  "../src/zeta/workbench/services/views/common/viewDescriptorService.js"
);
const {
  ViewContainerLocation,
  WorkbenchViewRegistry,
} = await import("../src/zeta/workbench/common/views.js");
const {
  ToggleAuxiliaryBarCommandId,
  TogglePanelCommandId,
  ToggleSideBarCommandId,
} = await import(
  "../src/zeta/workbench/browser/parts/titlebar/titlebarActions.js"
);
const { CommandService } = await import(
  "../src/zeta/workbench/services/commands/common/commandService.js"
);
const { ServiceCollection } = await import(
  "../src/zeta/platform/instantiation/common/instantiation.js"
);

type WorkbenchPartId =
  import("../src/zeta/workbench/browser/layout.js").WorkbenchPartId;
type WorkbenchLayoutInstance =
  import("../src/zeta/workbench/browser/layout.js").WorkbenchLayout;
type WorkbenchPartInstance =
  import("../src/zeta/workbench/browser/part.js").WorkbenchPart;
type EditorPartInstance =
  import("../src/zeta/workbench/browser/parts/editor/editorPart.js").EditorPart;

class TestPart extends WorkbenchPart {
  constructor(
    readonly id: WorkbenchPartId,
    ownerDocument: Document,
  ) {
    super(id, ownerDocument);
  }

  override get minimumWidth(): number {
    return this.id === "sidebar" || this.id === "auxiliarybar"
      ? 180
      : this.id === "editor"
      ? 120
      : 0;
  }

  override get maximumWidth(): number {
    return this.id === "sidebar" || this.id === "auxiliarybar"
      ? 600
      : Number.POSITIVE_INFINITY;
  }

  override get minimumHeight(): number {
    if (this.id === "titlebar") return 35;
    if (this.id === "session") return 36;
    if (this.id === "statusbar") return 23;
    if (this.id === "editor") return 84;
    if (this.id === "panel") return 80;
    return 0;
  }

  override get maximumHeight(): number {
    if (this.id === "titlebar") return 35;
    if (this.id === "session") return 36;
    if (this.id === "statusbar") return 23;
    return Number.POSITIVE_INFINITY;
  }
}

function createLayoutHarness(ownerDocument: Document): {
  readonly disposables: DisposableStore;
  readonly container: HTMLElement;
  readonly editor: EditorPartInstance;
  readonly layout: WorkbenchLayoutInstance;
} {
  const disposables = new DisposableStore();
  const container = ownerDocument.createElement("main");
  ownerDocument.body.append(container);
  disposables.defer(() => container.remove());

  const parts = new Map<WorkbenchPartId, WorkbenchPartInstance>();
  let editor: EditorPartInstance | undefined;
  for (const partId of workbenchPartIds) {
    const part = partId === "editor"
      ? new EditorPart(ownerDocument)
      : new TestPart(partId, ownerDocument);
    disposables.add(part);
    parts.set(partId, part);
    if (part instanceof EditorPart) editor = part;
  }
  if (!editor) throw new Error("Test layout requires an editor Part");

  const layout = disposables.add(new WorkbenchLayout(container, parts));
  return { disposables, container, editor, layout };
}

test("Workbench layout hides and restores Parts with context keys", () => {
  const dom = new JSDOM("<!doctype html><body></body>");
  const harness = createLayoutHarness(dom.window.document);
  const overlay = dom.window.document.createElement("div");
  overlay.className = "persistent-overlay";
  harness.container.append(overlay);
  const contextKeys = new ContextKeyService();
  harness.disposables.add(contextKeys);
  harness.disposables.add(bindWorkbenchPartVisibilityContextKeys(
    contextKeys,
    harness.layout,
  ));

  harness.layout.layout(new Dimension(1_000, 700));
  assert.equal(
    harness.container.querySelectorAll(".zeta-sash").length,
    3,
  );
  assert.equal(overlay.parentElement, harness.container);
  assert.ok(overlay.isConnected);
  assert.ok(harness.container.querySelector("[data-part='sidebar']"));
  assert.equal(contextKeys.getValue("sideBarVisible"), true);
  assert.equal(contextKeys.getValue("panelVisible"), true);
  harness.layout.hideParts(["sidebar", "auxiliarybar", "panel"]);
  assert.ok(overlay.isConnected);
  assert.equal(
    harness.container.querySelector<HTMLElement>(
      "[data-part='sidebar']",
    )?.hidden,
    true,
  );
  assert.equal(
    harness.container.querySelector<HTMLElement>(
      "[data-part='auxiliarybar']",
    )?.hidden,
    true,
  );
  assert.ok(harness.container.querySelector("[data-part='editor']"));
  assert.equal(contextKeys.getValue("sideBarVisible"), false);
  assert.equal(contextKeys.getValue("auxiliaryBarVisible"), false);
  assert.equal(contextKeys.getValue("panelVisible"), false);
  assert.equal(contextKeys.getValue("editorAreaVisible"), true);
  harness.layout.showPart("sidebar");
  assert.equal(
    harness.container.querySelector<HTMLElement>(
      "[data-part='sidebar']",
    )?.hidden,
    false,
  );
  assert.equal(contextKeys.getValue("sideBarVisible"), true);

  harness.disposables.dispose();
  dom.window.close();
});

test("Workbench layout state is versioned and excludes topology", () => {
  const dom = new JSDOM("<!doctype html><body></body>");
  const harness = createLayoutHarness(dom.window.document);
  harness.layout.layout(new Dimension(1_000, 700));
  harness.layout.resizePart(
    "sidebar",
    harness.layout.getPartSize("sidebar").with(250),
  );
  harness.layout.hidePart("auxiliarybar");
  harness.layout.resizePart(
    "panel",
    new Dimension(harness.layout.getPartSize("panel").width, 180),
  );
  const state = harness.layout.state;

  assert.deepEqual(state, {
    version: 2,
    sidebar: { width: 250, visible: true },
    auxiliarybar: { width: 220, visible: false },
    panel: { height: 180, visible: true },
  });
  assert.equal("children" in state, false);

  harness.disposables.dispose();
  dom.window.close();
});

test("Workbench layout validates and restores mutable state only", () => {
  const dom = new JSDOM("<!doctype html><body></body>");
  const harness = createLayoutHarness(dom.window.document);
  harness.layout.layout(new Dimension(1_000, 700));
  harness.layout.restoreState({
    version: 2,
    sidebar: { width: 260, visible: true },
    auxiliarybar: { width: 240, visible: false },
    panel: { height: 160, visible: false },
  });

  assert.equal(harness.layout.getPartSize("sidebar").width, 260);
  assert.equal(harness.layout.isPartVisible("auxiliarybar"), false);
  assert.equal(harness.layout.isPartVisible("panel"), false);
  harness.layout.restoreState({
    version: 1,
    sidebar: { width: 250, visible: true },
    auxiliarybar: { width: 230, visible: true },
  });
  assert.equal(harness.layout.getPartSize("panel").height, 200);
  assert.equal(harness.layout.isPartVisible("panel"), true);
  assert.throws(
    () => harness.layout.restoreState({
      version: 3,
      sidebar: { width: 260, visible: true },
      auxiliarybar: { width: 240, visible: false },
      panel: { height: 160, visible: false },
    }),
    /invalid or unsupported/,
  );

  harness.disposables.dispose();
  dom.window.close();
});

test("Workbench layout retains resized Part dimensions across visibility", () => {
  const dom = new JSDOM("<!doctype html><body></body>");
  const harness = createLayoutHarness(dom.window.document);
  harness.layout.layout(new Dimension(1_000, 700));
  harness.layout.resizePart(
    "sidebar",
    harness.layout.getPartSize("sidebar").with(250),
  );
  assert.equal(harness.layout.getPartSize("sidebar").width, 250);
  harness.layout.hidePart("sidebar");
  harness.layout.showPart("sidebar");
  assert.equal(harness.layout.getPartSize("sidebar").width, 250);
  const restoredSidebarPane = harness.container.querySelector<HTMLElement>(
    "[data-part='sidebar']",
  )?.parentElement;
  assert.equal(restoredSidebarPane?.style.width, "250px");

  harness.disposables.dispose();
  dom.window.close();
});

test("Activity Bar tracks and selects Sidebar View Containers", () => {
  const dom = new JSDOM("<!doctype html><body></body>");
  const disposables = new DisposableStore();
  const registry = new WorkbenchViewRegistry();
  disposables.add(registry.registerViewContainer({
    id: "zeta.explorer",
    title: "Explorer",
    location: ViewContainerLocation.Sidebar,
    order: 1,
    isDefault: true,
  }));
  disposables.add(registry.registerViewContainer({
    id: "zeta.search",
    title: "Search",
    location: ViewContainerLocation.Sidebar,
    order: 2,
  }));
  disposables.add(registry.registerViewContainer({
    id: "zeta.git",
    title: "Git",
    location: ViewContainerLocation.Sidebar,
    order: 3,
  }));
  const contextKeys = disposables.add(new ContextKeyService());
  const viewDescriptors = disposables.add(new ViewDescriptorService({
    contextKeyService: contextKeys,
    registry,
  }));
  const activitybar = disposables.add(new ActivitybarPart({
    ownerDocument: dom.window.document,
    viewDescriptorService: viewDescriptors,
  }));
  const sidebar = disposables.add(new SidebarPart(
    dom.window.document,
    activitybar,
  ));
  const selections: string[] = [];
  disposables.add(activitybar.onDidSelectViewContainer(
    ({ viewContainerId }) => selections.push(viewContainerId),
  ));

  assert.equal(
    activitybar.element.querySelectorAll(".zeta-action-view-item").length,
    3,
  );
  assert.deepEqual(
    [...activitybar.element.querySelectorAll<HTMLElement>(
      ".zeta-action-view-item",
    )].map((item) => item.dataset.actionId),
    ["zeta.explorer", "zeta.search", "zeta.git"],
  );
  assert.equal(activitybar.element.parentElement, sidebar.element);
  assert.equal(sidebar.element.firstElementChild, activitybar.element);
  assert.equal(
    activitybar.element.className,
    "zeta-activitybar-container",
  );
  const actionbar = activitybar.element.querySelector(
    ":scope > .zeta-action-bar",
  );
  assert.equal(actionbar?.className, "zeta-action-bar");
  assert.equal(actionbar?.getAttribute("role"), "tablist");
  assert.deepEqual(
    [...(actionbar?.children ?? [])].map((item) => item.className),
    [
      "zeta-action-view-item",
      "zeta-action-view-item",
      "zeta-action-view-item",
    ],
  );
  assert.equal(
    activitybar.element.querySelectorAll(
      ".zeta-action-bar > .zeta-action-view-item",
    ).length,
    3,
  );
  assert.equal(
    activitybar.element.querySelectorAll(
      ".zeta-action-view-item > .zeta-action-label",
    ).length,
    3,
  );
  assert.equal(activitybar.element.querySelector("button"), null);
  assert.equal(activitybar.element.hasAttribute("data-part"), false);
  assert.equal(
    activitybar.element.querySelector(".zeta-workbench-part-content"),
    null,
  );
  activitybar.setActiveViewContainer("zeta.explorer");
  assert.equal(
    activitybar.element.querySelector<HTMLElement>(
      "[data-action-id='zeta.explorer']",
    )?.getAttribute("aria-selected"),
    "true",
  );
  activitybar.element.querySelector<HTMLElement>(
    "[data-action-id='zeta.explorer']",
  )?.click();
  assert.deepEqual(selections, []);
  activitybar.element.querySelector<HTMLElement>(
    "[data-action-id='zeta.search']",
  )?.click();
  assert.deepEqual(selections, ["zeta.search"]);

  disposables.dispose();
  dom.window.close();
});

test("titlebar layout commands toggle shell regions", async () => {
  const dom = new JSDOM("<!doctype html><body></body>");
  const harness = createLayoutHarness(dom.window.document);
  const services = new ServiceCollection();
  services.set(IWorkbenchLayoutService, harness.layout);
  const commands = harness.disposables.add(new CommandService(services));

  assert.equal(harness.layout.isPartVisible("sidebar"), true);
  await commands.executeCommand(ToggleSideBarCommandId);
  assert.equal(harness.layout.isPartVisible("sidebar"), false);
  await commands.executeCommand(ToggleSideBarCommandId);
  assert.equal(harness.layout.isPartVisible("sidebar"), true);

  assert.equal(harness.layout.isPartVisible("auxiliarybar"), true);
  await commands.executeCommand(ToggleAuxiliaryBarCommandId);
  assert.equal(harness.layout.isPartVisible("auxiliarybar"), false);
  await commands.executeCommand(ToggleAuxiliaryBarCommandId);
  assert.equal(harness.layout.isPartVisible("auxiliarybar"), true);

  assert.equal(harness.layout.isPartVisible("panel"), true);
  await commands.executeCommand(TogglePanelCommandId);
  assert.equal(harness.layout.isPartVisible("panel"), false);
  await commands.executeCommand(TogglePanelCommandId);
  assert.equal(harness.layout.isPartVisible("panel"), true);

  harness.disposables.dispose();
  dom.window.close();
});

test("panel toggle action uses state icons in the right titlebar", () => {
  using disposables = new DisposableStore();
  const contextKeys = disposables.add(new ContextKeyService());
  const commands = disposables.add(
    new CommandService(new ServiceCollection()),
  );
  const menuService = new MenuService(commands, contextKeys);
  const panelAction = () => menuService
    .getMenuActions(MenuId.TitleBar)
    .flatMap(([, actions]) => actions)
    .find((action) => action.id === TogglePanelCommandId);

  assert.equal(panelAction()?.icon, LxIcon.layoutPanelOff);
  contextKeys.setContext("panelVisible", true);
  assert.equal(panelAction()?.icon, LxIcon.layoutPanel);
});
