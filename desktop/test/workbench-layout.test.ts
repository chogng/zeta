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
const {
  applyWorkbenchPartVisibilityContext,
  IWorkbenchLayoutService,
  WorkbenchLayout,
  workbenchPartIds,
} = await import("../src/zeta/workbench/browser/layout.js");
const { WorkbenchPart } = await import("../src/zeta/workbench/browser/part.js");
const { EditorPart } = await import(
  "../src/zeta/workbench/browser/parts/editor/editorPart.js"
);
const {
  ToggleAuxiliaryBarCommandId,
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
  applyWorkbenchPartVisibilityContext(contextKeys, "sidebar", true);
  applyWorkbenchPartVisibilityContext(contextKeys, "auxiliarybar", true);
  applyWorkbenchPartVisibilityContext(contextKeys, "editor", true);
  harness.disposables.add(harness.layout.onDidChangePartVisibility(
    ({ partId, visible }) =>
      applyWorkbenchPartVisibilityContext(contextKeys, partId, visible),
  ));

  harness.layout.layout(new Dimension(1_000, 700));
  assert.equal(
    harness.container.querySelectorAll(".zeta-sash").length,
    2,
  );
  assert.equal(overlay.parentElement, harness.container);
  assert.ok(overlay.isConnected);
  assert.ok(harness.container.querySelector("[data-part='sidebar']"));
  assert.equal(contextKeys.getValue("sideBarVisible"), true);

  harness.layout.hideParts(["sidebar", "auxiliarybar"]);
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
  const state = harness.layout.state;

  assert.deepEqual(state, {
    version: 1,
    sidebar: { width: 250, visible: true },
    auxiliarybar: { width: 220, visible: false },
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
    version: 1,
    sidebar: { width: 260, visible: true },
    auxiliarybar: { width: 240, visible: false },
  });

  assert.equal(harness.layout.getPartSize("sidebar").width, 260);
  assert.equal(harness.layout.isPartVisible("auxiliarybar"), false);
  assert.throws(
    () => harness.layout.restoreState({
      version: 2,
      sidebar: { width: 260, visible: true },
      auxiliarybar: { width: 240, visible: false },
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
    ".zeta-split-view-horizontal > .zeta-split-view-pane",
  );
  assert.equal(restoredSidebarPane?.style.width, "250px");

  harness.disposables.dispose();
  dom.window.close();
});

test("titlebar layout commands toggle both sidebars", async () => {
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

  harness.disposables.dispose();
  dom.window.close();
});
