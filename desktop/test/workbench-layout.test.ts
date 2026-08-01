import assert from "node:assert/strict";
import test from "node:test";
import { JSDOM } from "jsdom";
import type { IContextMenuProvider } from "../src/zeta/base/browser/contextmenu.js";
import type { IAction } from "../src/zeta/base/common/actions.js";
import { DisposableStore } from "../src/zeta/base/common/lifecycle.js";
import type { IViewPaneOptions } from "../src/zeta/workbench/browser/parts/views/viewPane.js";
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
const { lxiconsLibrary } = await import("../src/zeta/base/common/lxiconsLibrary.js");
const { WillSaveStateReason } = await import("../src/zeta/platform/storage/common/storage.js");
const { MenuId } = await import(
  "../src/zeta/platform/actions/common/actions.js"
);
const { MenuService } = await import(
  "../src/zeta/platform/actions/common/menuService.js"
);
const {
  WorkbenchLayout,
} = await import("../src/zeta/workbench/browser/layout.js");
const { IWorkbenchLayoutService, workbenchPartIds } = await import("../src/zeta/workbench/services/layout/browser/layoutService.js");
const { BrowserStorageService } = await import("../src/zeta/workbench/services/storage/browser/storageService.js");
const {
  bindWorkbenchPartVisibilityContextKeys,
} = await import("../src/zeta/workbench/browser/contextkeys.js");
const { WorkbenchPart } = await import("../src/zeta/workbench/browser/part.js");
const { SidebarPart } = await import(
  "../src/zeta/workbench/browser/parts/sidebar/sidebarPart.js"
);
const { PanelPart } = await import(
  "../src/zeta/workbench/browser/parts/panel/panelPart.js"
);
const { CompositeBar } = await import("../src/zeta/workbench/browser/parts/compositebar/compositeBar.js");
const { AuxiliarybarPart } = await import(
  "../src/zeta/workbench/browser/parts/auxiliarybar/auxiliarybarPart.js"
);
const { PaneComposite } = await import(
  "../src/zeta/workbench/browser/parts/views/paneComposite.js"
);
const { ViewPaneContainer } = await import(
  "../src/zeta/workbench/browser/parts/views/viewPaneContainer.js"
);
const { ViewPane } = await import(
  "../src/zeta/workbench/browser/parts/views/viewPane.js"
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
  ToggleMaximizedPanelCommandId,
  TogglePanelCommandId,
  ToggleSideBarCommandId,
} = await import(
  "../src/zeta/workbench/browser/parts/titlebar/titlebarActions.js"
);
const { CommandService } = await import(
  "../src/zeta/workbench/services/commands/common/commandService.js"
);
const { InstantiationService, ServiceCollection, SyncDescriptor } = await import(
  "../src/zeta/platform/instantiation/common/instantiation.js"
);

type WorkbenchPartId =
  import("../src/zeta/workbench/services/layout/browser/layoutService.js").WorkbenchPartId;
type WorkbenchLayoutInstance =
  import("../src/zeta/workbench/browser/layout.js").WorkbenchLayout;
type WorkbenchLayoutOptions =
  import("../src/zeta/workbench/browser/layout.js").WorkbenchLayoutOptions;
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

function createLayoutHarness(
  ownerDocument: Document,
  options: WorkbenchLayoutOptions = {},
): {
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

  const layout = disposables.add(new WorkbenchLayout(container, parts, options));
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
  assert.equal(contextKeys.getValue("auxiliaryBarVisible"), true);
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
    auxiliarybar: { width: 260, visible: false },
    panel: { height: 180, visible: true },
  });
  assert.equal("children" in state, false);

  harness.disposables.dispose();
  dom.window.close();
});

test("Workbench layout derives flexible editor size from the container", () => {
  const dom = new JSDOM("<!doctype html><body></body>");
  const harness = createLayoutHarness(dom.window.document, {
    initialDimension: new Dimension(1_200, 800),
  });
  harness.layout.layout(new Dimension(1_200, 800));

  assert.deepEqual(
    harness.layout.getPartSize("sidebar"),
    new Dimension(220, 742),
  );
  assert.deepEqual(
    harness.layout.getPartSize("auxiliarybar"),
    new Dimension(260, 742),
  );
  assert.deepEqual(
    harness.layout.getPartSize("editor"),
    new Dimension(720, 542),
  );
  assert.equal(harness.layout.getPartSize("panel").height, 200);

  harness.layout.layout(new Dimension(1_300, 800));
  assert.equal(harness.layout.getPartSize("sidebar").width, 220);
  assert.equal(harness.layout.getPartSize("auxiliarybar").width, 260);
  assert.equal(harness.layout.getPartSize("editor").width, 820);

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

test("Workbench layout restores scoped state through the storage service", async () => {
  const dom = new JSDOM("<!doctype html><body></body>", {
    url: "https://zeta.test",
  });
  const createStorage = (workspaceId: string) => new BrowserStorageService({
    ownerWindow: dom.window as unknown as Window,
    applicationId: "code",
    workspaceId,
    backend: dom.window.localStorage,
    flushInterval: 0,
  });
  const firstStorage = createStorage("workspace-a");
  const first = createLayoutHarness(dom.window.document, {
    initialDimension: new Dimension(1_000, 700),
    storageService: firstStorage,
  });
  first.layout.layout(new Dimension(1_000, 700));
  first.layout.resizePart(
    "auxiliarybar",
    first.layout.getPartSize("auxiliarybar").with(310),
  );
  first.layout.hidePart("auxiliarybar");
  first.layout.resizePart(
    "panel",
    new Dimension(first.layout.getPartSize("panel").width, 175),
  );
  await firstStorage.flush(WillSaveStateReason.SHUTDOWN);
  first.disposables.dispose();
  firstStorage.dispose();

  const restoredStorage = createStorage("workspace-a");
  const restored = createLayoutHarness(dom.window.document, {
    initialDimension: new Dimension(1_000, 700),
    storageService: restoredStorage,
  });
  restored.layout.layout(new Dimension(1_000, 700));
  assert.equal(restored.layout.getPartSize("auxiliarybar").width, 310);
  assert.equal(restored.layout.isPartVisible("auxiliarybar"), false);
  assert.equal(restored.layout.getPartSize("panel").height, 175);
  restored.disposables.dispose();
  restoredStorage.dispose();

  const otherWorkspaceStorage = createStorage("workspace-b");
  const otherWorkspace = createLayoutHarness(dom.window.document, {
    initialDimension: new Dimension(1_000, 700),
    storageService: otherWorkspaceStorage,
  });
  otherWorkspace.layout.layout(new Dimension(1_000, 700));
  assert.equal(otherWorkspace.layout.getPartSize("auxiliarybar").width, 310);
  assert.equal(otherWorkspace.layout.isPartVisible("auxiliarybar"), true);
  assert.equal(otherWorkspace.layout.getPartSize("panel").height, 175);

  otherWorkspace.disposables.dispose();
  otherWorkspaceStorage.dispose();
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
    "zeta-composite-bar zeta-composite-bar-icon",
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

test("Sidebar can host Agent Sidebar composites", () => {
  const dom = new JSDOM("<!doctype html><body></body>");
  const disposables = new DisposableStore();
  const registry = new WorkbenchViewRegistry();
  disposables.add(registry.registerViewContainer({
    id: "zeta.chat",
    title: "Chat",
    location: ViewContainerLocation.AgentSidebar,
    isDefault: true,
  }));
  const contextKeys = disposables.add(new ContextKeyService());
  const viewDescriptors = disposables.add(new ViewDescriptorService({
    contextKeyService: contextKeys,
    registry,
  }));
  const agentSidebar = disposables.add(new SidebarPart({
    ownerDocument: dom.window.document,
    viewDescriptorService: viewDescriptors,
    id: "agentSidebar",
    location: ViewContainerLocation.AgentSidebar,
    ariaLabel: "Agent sidebar",
    viewsAriaLabel: "Agent sidebar views",
  }));

  assert.equal(agentSidebar.element.dataset.part, "agentSidebar");
  assert.equal(agentSidebar.element.getAttribute("aria-label"), "Agent sidebar");
  assert.equal(
    agentSidebar.compositeBar.element.querySelector(
      "[data-action-id='zeta.chat']",
    ) !== null,
    true,
  );

  disposables.dispose();
  dom.window.close();
});

test("Panel presents its destinations as tabs and active commands as a toolbar", () => {
  const dom = new JSDOM("<!doctype html><body></body>");
  const disposables = new DisposableStore();
  const registry = new WorkbenchViewRegistry();
  const panels = [
    ["zeta.panel.problems", "Problems"],
    ["zeta.panel.output", "Output"],
    ["zeta.panel.terminal", "Terminal"],
    ["zeta.panel.ports", "Ports"],
  ] as const;
  for (const [id, title] of panels) {
    disposables.add(registry.registerViewContainer({
      id,
      title,
      location: ViewContainerLocation.Panel,
      order: panels.findIndex(([candidate]) => candidate === id),
      isDefault: title === "Terminal",
    }));
  }
  disposables.add(registry.registerViews("zeta.panel.terminal", [{
    id: "zeta.terminal",
    title: "Terminal",
    ctorDescriptor: new SyncDescriptor(TestPanelView),
  }]));
  const contextKeys = disposables.add(new ContextKeyService());
  const viewDescriptors = disposables.add(new ViewDescriptorService({
    contextKeyService: contextKeys,
    registry,
  }));
  const panel = disposables.add(new PanelPart({
    ownerDocument: dom.window.document,
    viewDescriptorService: viewDescriptors,
  }));
  dom.window.document.body.append(panel.element);

  const tablist = panel.element.querySelector(".zeta-panel-title-control [role='tablist']");
  assert.equal(panel.compositeBar.element.className, "zeta-composite-bar zeta-composite-bar-label");
  assert.equal(tablist?.getAttribute("aria-label"), "Panel views");
  assert.deepEqual(
    [...(tablist?.querySelectorAll("[role='tab']") ?? [])].map((tab) => tab.textContent),
    ["Problems", "Output", "Terminal", "Ports"],
  );
  assert.equal(panel.element.querySelectorAll(".zeta-panel-title-control [role='toolbar']").length, 0);

  const terminalDescriptor = viewDescriptors.getViewContainers(ViewContainerLocation.Panel).find((container) => container.id === "zeta.panel.terminal");
  assert.ok(terminalDescriptor);
  const terminal = new PaneComposite({
    viewContainer: terminalDescriptor,
    model: viewDescriptors.getViewContainerModel(terminalDescriptor.id),
    instantiationService: new InstantiationService(),
    contextKeyService: contextKeys,
    ownerDocument: dom.window.document,
    paneHeaders: "hidden",
    paneLayout: "fill",
  });
  panel.addComposite(terminal);
  panel.showComposite(terminal.id);
  panel.setActiveComposite(terminal.id);

  const toolbar = panel.element.querySelector(".zeta-panel-title-actions [role='toolbar']");
  const terminalTab = panel.element.querySelector("[role='tab'][aria-selected='true']");
  assert.equal(toolbar?.getAttribute("aria-label"), "Test panel actions");
  assert.equal(toolbar?.querySelector("button")?.textContent, "Run");
  assert.equal(panel.element.querySelectorAll(".zeta-panel-title-control [role='tablist']").length, 1);
  assert.equal(terminal.element.getAttribute("role"), "tabpanel");
  assert.equal(terminal.element.classList.contains("zeta-pane-composite-pane-headers-hidden"), true);
  assert.equal(terminal.element.classList.contains("zeta-pane-composite-pane-layout-fill"), true);
  assert.equal(terminalTab?.getAttribute("aria-controls"), terminal.element.id);
  assert.equal(terminal.element.getAttribute("aria-labelledby"), terminalTab?.id);

  disposables.dispose();
  dom.window.close();
});

test("CompositeBar moves non-fitting label tabs into its overflow menu", () => {
  const dom = new JSDOM("<!doctype html><body></body>");
  const disposables = new DisposableStore();
  const registry = new WorkbenchViewRegistry();
  const panels = [
    ["zeta.panel.problems", "Problems"],
    ["zeta.panel.output", "Output"],
    ["zeta.panel.terminal", "Terminal"],
  ] as const;
  for (const [id, title] of panels) {
    disposables.add(registry.registerViewContainer({
      id,
      title,
      location: ViewContainerLocation.Panel,
    }));
  }
  const contextKeys = disposables.add(new ContextKeyService());
  const viewDescriptors = disposables.add(new ViewDescriptorService({
    contextKeyService: contextKeys,
    registry,
  }));
  const selections: string[] = [];
  let overflowActions: readonly IAction[] = [];
  let hideOverflowMenu: (() => void) | undefined;
  const contextMenuProvider: IContextMenuProvider = {
    showContextMenu(options): void {
      overflowActions = options.actions;
      hideOverflowMenu = () => options.onHide?.(true);
    },
  };
  const compositeBar = disposables.add(new CompositeBar({
    ownerDocument: dom.window.document,
    viewDescriptorService: viewDescriptors,
    location: ViewContainerLocation.Panel,
    ariaLabel: "Panel views",
    presentation: "label",
    contextMenuProvider,
  }));
  disposables.add(compositeBar.onDidSelectComposite(
    ({ compositeId }) => selections.push(compositeId),
  ));
  dom.window.document.body.append(compositeBar.element);
  Object.defineProperty(compositeBar.element, "clientWidth", {
    configurable: true,
    value: 110,
  });
  compositeBar.setActiveComposite("zeta.panel.terminal");
  for (const tab of compositeBar.element.querySelectorAll<HTMLElement>(".zeta-tab")) {
    tab.getBoundingClientRect = () => ({ width: 50 } as DOMRect);
  }
  compositeBar.layout();

  assert.deepEqual(
    [...compositeBar.element.querySelectorAll("[role='tab']")]
      .map((tab) => tab.textContent),
    ["Terminal"],
  );
  const overflowButton = compositeBar.element.querySelector<HTMLButtonElement>(
    ".zeta-composite-bar-overflow",
  );
  assert.ok(overflowButton);
  assert.equal(overflowButton.hidden, false);
  overflowButton.click();
  assert.equal(overflowButton.getAttribute("aria-expanded"), "true");
  assert.deepEqual(
    overflowActions.map((action) => action.label),
    ["Problems", "Output"],
  );
  const problems = overflowActions[0];
  assert.ok(problems);
  problems.run();
  assert.deepEqual(selections, ["zeta.panel.problems"]);
  assert.ok(hideOverflowMenu);
  hideOverflowMenu();
  assert.equal(overflowButton.getAttribute("aria-expanded"), "false");
  compositeBar.setOverflowPresentation("external");
  assert.equal(overflowButton.hidden, true);
  assert.equal(overflowButton.classList.contains("hidden"), true);
  assert.deepEqual(
    compositeBar.getOverflowActions().map((action) => action.label),
    ["Problems", "Output"],
  );

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

class TestPanelView extends ViewPane {
  private readonly actions: HTMLDivElement;

  constructor(options: IViewPaneOptions) {
    super(options);
    this.actions = options.ownerDocument.createElement("div");
    this.actions.setAttribute("role", "toolbar");
    this.actions.setAttribute("aria-label", "Test panel actions");
    const button = options.ownerDocument.createElement("button");
    button.textContent = "Run";
    this.actions.append(button);
  }

  override get titleActionsElement(): HTMLElement {
    return this.actions;
  }
}

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

  assert.equal(harness.layout.isPartVisible("editor"), true);
  await commands.executeCommand(ToggleMaximizedPanelCommandId);
  assert.equal(harness.layout.isPartVisible("panel"), true);
  assert.equal(harness.layout.isPartVisible("editor"), false);
  await commands.executeCommand(ToggleMaximizedPanelCommandId);
  assert.equal(harness.layout.isPartVisible("editor"), true);

  await commands.executeCommand(ToggleMaximizedPanelCommandId);
  await commands.executeCommand(TogglePanelCommandId);
  assert.equal(harness.layout.isPartVisible("panel"), false);
  assert.equal(harness.layout.isPartVisible("editor"), true);

  harness.disposables.dispose();
  dom.window.close();
});

test("panel layout actions use state icons", () => {
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
  const maximizePanelAction = () => menuService
    .getMenuActions(MenuId.TerminalTitle)
    .flatMap(([, actions]) => actions)
    .find((action) => action.id === ToggleMaximizedPanelCommandId);

  assert.equal(panelAction()?.icon, lxiconsLibrary.layoutPanelOff);
  contextKeys.setContext("panelVisible", true);
  assert.equal(panelAction()?.icon, lxiconsLibrary.layoutPanel);
  assert.equal(maximizePanelAction()?.icon, lxiconsLibrary.screenFull);
  assert.equal(maximizePanelAction()?.checked, false);
  contextKeys.setContext("editorAreaVisible", false);
  assert.equal(maximizePanelAction()?.icon, lxiconsLibrary.screenNormal);
  assert.equal(maximizePanelAction()?.checked, true);
});
