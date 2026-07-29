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
const { SidebarPart } = await import(
  "../src/zeta/workbench/browser/parts/sidebar/sidebarPart.js"
);
const { AuxiliarybarPart } = await import(
  "../src/zeta/workbench/browser/parts/auxiliarybar/auxiliarybarPart.js"
);
const { PaneComposite } = await import(
  "../src/zeta/workbench/browser/parts/views/paneComposite.js"
);
const { ViewPaneContainer } = await import(
  "../src/zeta/workbench/browser/parts/views/viewPaneContainer.js"
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
const { InstantiationService, ServiceCollection } = await import(
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
    if (this.id === "statusbar") return 23;
    if (this.id === "editor") return 84;
    if (this.id === "panel") return 80;
    return 0;
  }

  override get maximumHeight(): number {
    if (this.id === "titlebar") return 35;
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
  assert.equal(
    harness.container.querySelector("[data-part='session']"),
    null,
  );
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

test("Sidebar hosts its Composite Bar before content", () => {
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
  const sidebar = disposables.add(new SidebarPart({
    ownerDocument: dom.window.document,
    viewDescriptorService: viewDescriptors,
  }));
  dom.window.document.body.append(sidebar.element);
  const compositeBar = sidebar.compositeBar;
  const selections: string[] = [];
  disposables.add(sidebar.onDidSelectComposite(
    ({ compositeId }) => selections.push(compositeId),
  ));

  assert.equal(
    compositeBar.element.querySelectorAll(".zeta-action-view-item").length,
    3,
  );
  assert.deepEqual(
    [...compositeBar.element.querySelectorAll<HTMLElement>(
      ".zeta-action-view-item",
    )].map((item) => item.dataset.actionId),
    ["zeta.explorer", "zeta.search", "zeta.git"],
  );
  assert.equal(
    compositeBar.element.parentElement,
    sidebar.element,
  );
  assert.equal(sidebar.element.firstElementChild, compositeBar.element);
  assert.equal(
    compositeBar.element.className,
    "zeta-composite-bar",
  );
  const content = sidebar.element.querySelector(
    ":scope > .zeta-composite-content",
  );
  assert.ok(content);
  assert.equal(compositeBar.element.nextElementSibling, content);
  assert.equal(
    sidebar.element.querySelector(":scope > .zeta-workbench-part-title"),
    null,
  );
  const actionbar = compositeBar.element.querySelector(
    ".zeta-tab-list-scroll-content > .zeta-action-bar",
  );
  assert.equal(actionbar?.className, "zeta-action-bar");
  assert.equal(actionbar?.getAttribute("role"), "tablist");
  assert.equal(actionbar?.getAttribute("aria-orientation"), "horizontal");
  assert.deepEqual(
    [...(actionbar?.children ?? [])].map(
      (item) => item.classList.contains("zeta-tab"),
    ),
    [true, true, true],
  );
  assert.equal(
    compositeBar.element.querySelectorAll(
      ".zeta-action-bar > .zeta-action-view-item",
    ).length,
    3,
  );
  assert.equal(
    compositeBar.element.querySelectorAll(
      ".zeta-action-view-item > .zeta-tab-label",
    ).length,
    3,
  );
  assert.equal(compositeBar.element.querySelectorAll("button").length, 3);
  assert.equal(compositeBar.element.hasAttribute("data-part"), false);
  assert.equal(
    compositeBar.element.querySelector(".zeta-workbench-part-content"),
    null,
  );
  sidebar.setActiveComposite("zeta.explorer");
  const explorerTab = compositeBar.element.querySelector<HTMLButtonElement>(
    "[data-action-id='zeta.explorer'] > [role='tab']",
  );
  const searchTab = compositeBar.element.querySelector<HTMLButtonElement>(
    "[data-action-id='zeta.search'] > [role='tab']",
  );
  assert.ok(explorerTab);
  assert.ok(searchTab);
  assert.equal(
    explorerTab.getAttribute("aria-selected"),
    "true",
  );
  explorerTab.focus();
  explorerTab.dispatchEvent(new dom.window.KeyboardEvent("keydown", {
    bubbles: true,
    cancelable: true,
    key: "ArrowRight",
  }));
  assert.equal(dom.window.document.activeElement, searchTab);
  assert.deepEqual(selections, []);
  explorerTab.click();
  assert.deepEqual(selections, []);
  searchTab.click();
  assert.deepEqual(selections, ["zeta.search"]);
  assert.equal(compositeBar.activeCompositeId, "zeta.explorer");

  const instantiationService = new InstantiationService();
  const explorerContainer = viewDescriptors.getViewContainers(
    ViewContainerLocation.Sidebar,
  )[0];
  const searchContainer = viewDescriptors.getViewContainers(
    ViewContainerLocation.Sidebar,
  )[1];
  const explorerComposite = new PaneComposite({
    viewContainer: explorerContainer,
    model: viewDescriptors.getViewContainerModel(explorerContainer.id),
    instantiationService,
    contextKeyService: contextKeys,
    ownerDocument: dom.window.document,
  });
  const searchComposite = new PaneComposite({
    viewContainer: searchContainer,
    model: viewDescriptors.getViewContainerModel(searchContainer.id),
    instantiationService,
    contextKeyService: contextKeys,
    ownerDocument: dom.window.document,
  });
  sidebar.addComposite(explorerComposite);
  sidebar.addComposite(searchComposite);
  sidebar.showComposite(explorerComposite.id);
  assert.equal(sidebar.activeCompositeId, explorerComposite.id);
  assert.equal(explorerComposite.element.hidden, false);
  assert.equal(searchComposite.element.hidden, true);
  sidebar.showComposite(searchComposite.id);
  assert.equal(sidebar.activeCompositeId, searchComposite.id);
  assert.equal(explorerComposite.element.hidden, true);
  assert.equal(searchComposite.element.hidden, false);
  sidebar.showComposite(explorerComposite.id);
  assert.equal(
    sidebar.getComposite(explorerComposite.id),
    explorerComposite,
  );
  assert.equal(explorerComposite.element.hidden, false);

  disposables.dispose();
  dom.window.close();
});

test("Auxiliary Bar directly hosts its fixed View container", () => {
  const dom = new JSDOM("<!doctype html><body></body>");
  const disposables = new DisposableStore();
  const registry = new WorkbenchViewRegistry();
  disposables.add(registry.registerViewContainer({
    id: "zeta.chat",
    title: "Chat",
    location: ViewContainerLocation.AuxiliaryBar,
    isDefault: true,
  }));
  const contextKeys = disposables.add(new ContextKeyService());
  const viewDescriptors = disposables.add(new ViewDescriptorService({
    contextKeyService: contextKeys,
    registry,
  }));
  const descriptor = viewDescriptors.getDefaultViewContainer(
    ViewContainerLocation.AuxiliaryBar,
  );
  assert.ok(descriptor);
  const instantiationService = new InstantiationService(
    new ServiceCollection(),
  );
  const container = new ViewPaneContainer({
    viewContainer: descriptor,
    model: viewDescriptors.getViewContainerModel(descriptor.id),
    instantiationService,
    contextKeyService: contextKeys,
    ownerDocument: dom.window.document,
  });
  const auxiliarybar = disposables.add(
    new AuxiliarybarPart(dom.window.document),
  );
  auxiliarybar.setViewPaneContainer(container);
  const content = auxiliarybar.element.querySelector(
    ":scope > .zeta-auxiliarybar-content",
  );

  assert.ok(content);
  assert.equal(auxiliarybar.element.firstElementChild, content);
  assert.equal(content.firstElementChild, container.element);
  assert.equal(auxiliarybar.element.querySelector(".zeta-composite-bar"), null);
  assert.equal(
    auxiliarybar.element.querySelector(":scope > .zeta-workbench-part-title"),
    null,
  );

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
