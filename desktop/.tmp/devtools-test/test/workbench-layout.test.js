import assert from "node:assert/strict";
import test from "node:test";
import { JSDOM } from "jsdom";
import { DisposableStore } from "../src/base/common/lifecycle.js";
import { URI } from "../src/base/common/uri.js";
import { ContextKeyService, } from "../src/platform/contextkey/common/contextkey.js";
import { UNKNOWN_EMPTY_WINDOW_WORKSPACE, } from "../src/platform/workspace/common/workspace.js";
import { StartTurnCommandId, } from "../src/workbench/contrib/turn/common/turnCommands.js";
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
const { Dimension } = await import("../src/base/browser/geometry.js");
const { applyWorkbenchPartVisibilityContext, IWorkbenchLayoutService, WorkbenchLayout, workbenchPartIds, } = await import("../src/workbench/browser/layout.js");
const { WorkbenchPart } = await import("../src/workbench/browser/part.js");
const { EmptyWorkspaceContribution, } = await import("../src/workbench/contrib/emptyWorkspace/browser/emptyWorkspace.js");
const { EditorPart } = await import("../src/workbench/browser/parts/editor/editorPart.js");
const { WorkspaceContextService } = await import("../src/workbench/services/workspaces/browser/workspaceContextService.js");
const { ToggleAuxiliaryBarCommandId, ToggleSideBarCommandId, } = await import("../src/workbench/browser/parts/titlebar/titlebarActions.js");
const { CommandService } = await import("../src/workbench/services/commands/common/commandService.js");
const { ServiceCollection } = await import("../src/platform/instantiation/common/instantiation.js");
class TestPart extends WorkbenchPart {
    id;
    constructor(id, ownerDocument) {
        super(id, ownerDocument);
        this.id = id;
    }
    get minimumWidth() {
        return this.id === "sidebar" || this.id === "auxiliarybar"
            ? 180
            : this.id === "editor"
                ? 120
                : 0;
    }
    get maximumWidth() {
        return this.id === "sidebar" || this.id === "auxiliarybar"
            ? 600
            : Number.POSITIVE_INFINITY;
    }
    get minimumHeight() {
        if (this.id === "titlebar")
            return 35;
        if (this.id === "session")
            return 36;
        if (this.id === "statusbar")
            return 23;
        if (this.id === "editor")
            return 84;
        return 0;
    }
    get maximumHeight() {
        if (this.id === "titlebar")
            return 35;
        if (this.id === "session")
            return 36;
        if (this.id === "statusbar")
            return 23;
        return Number.POSITIVE_INFINITY;
    }
}
class TestCommandService {
    calls = [];
    onWillExecuteCommand = () => ({
        dispose() { },
        [Symbol.dispose]() { },
    });
    onDidExecuteCommand = this.onWillExecuteCommand;
    async executeCommand(id) {
        this.calls.push(id);
        return undefined;
    }
}
function createLayoutHarness(ownerDocument) {
    const disposables = new DisposableStore();
    const container = ownerDocument.createElement("main");
    ownerDocument.body.append(container);
    disposables.defer(() => container.remove());
    const parts = new Map();
    let editor;
    for (const partId of workbenchPartIds) {
        const part = partId === "editor"
            ? new EditorPart(ownerDocument)
            : new TestPart(partId, ownerDocument);
        disposables.add(part);
        parts.set(partId, part);
        if (part instanceof EditorPart)
            editor = part;
    }
    if (!editor)
        throw new Error("Test layout requires an editor Part");
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
    harness.disposables.add(harness.layout.onDidChangePartVisibility(({ partId, visible }) => applyWorkbenchPartVisibilityContext(contextKeys, partId, visible)));
    harness.layout.layout(new Dimension(1_000, 700));
    assert.equal(harness.container.querySelectorAll(".zeta-sash").length, 2);
    assert.equal(overlay.parentElement, harness.container);
    assert.ok(overlay.isConnected);
    assert.ok(harness.container.querySelector("[data-part='sidebar']"));
    assert.equal(contextKeys.getValue("sideBarVisible"), true);
    harness.layout.hideParts(["sidebar", "auxiliarybar"]);
    assert.ok(overlay.isConnected);
    assert.equal(harness.container.querySelector("[data-part='sidebar']")?.hidden, true);
    assert.equal(harness.container.querySelector("[data-part='auxiliarybar']")?.hidden, true);
    assert.ok(harness.container.querySelector("[data-part='editor']"));
    assert.equal(contextKeys.getValue("sideBarVisible"), false);
    assert.equal(contextKeys.getValue("auxiliaryBarVisible"), false);
    assert.equal(contextKeys.getValue("editorAreaVisible"), true);
    harness.layout.showPart("sidebar");
    assert.equal(harness.container.querySelector("[data-part='sidebar']")?.hidden, false);
    assert.equal(contextKeys.getValue("sideBarVisible"), true);
    harness.disposables.dispose();
    dom.window.close();
});
test("Workbench layout state is versioned and excludes topology", () => {
    const dom = new JSDOM("<!doctype html><body></body>");
    const harness = createLayoutHarness(dom.window.document);
    harness.layout.layout(new Dimension(1_000, 700));
    harness.layout.resizePart("sidebar", harness.layout.getPartSize("sidebar").with(250));
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
    assert.throws(() => harness.layout.restoreState({
        version: 2,
        sidebar: { width: 260, visible: true },
        auxiliarybar: { width: 240, visible: false },
    }), /invalid or unsupported/);
    harness.disposables.dispose();
    dom.window.close();
});
test("Workbench layout retains resized Part dimensions across visibility", () => {
    const dom = new JSDOM("<!doctype html><body></body>");
    const harness = createLayoutHarness(dom.window.document);
    harness.layout.layout(new Dimension(1_000, 700));
    harness.layout.resizePart("sidebar", harness.layout.getPartSize("sidebar").with(250));
    assert.equal(harness.layout.getPartSize("sidebar").width, 250);
    harness.layout.hidePart("sidebar");
    harness.layout.showPart("sidebar");
    assert.equal(harness.layout.getPartSize("sidebar").width, 250);
    const restoredSidebarPane = harness.container.querySelector(".zeta-split-view-horizontal > .zeta-split-view-pane");
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
test("empty workspace contribution owns its EmptyView and collapses side Parts", async () => {
    const dom = new JSDOM("<!doctype html><body></body>");
    const harness = createLayoutHarness(dom.window.document);
    const commands = new TestCommandService();
    const contribution = harness.disposables.add(new EmptyWorkspaceContribution(new WorkspaceContextService(UNKNOWN_EMPTY_WINDOW_WORKSPACE), harness.editor, harness.layout, commands));
    assert.equal(harness.layout.isPartVisible("sidebar"), false);
    assert.equal(harness.layout.isPartVisible("auxiliarybar"), false);
    assert.equal(harness.editor.element.querySelector("h1")?.textContent, "No folder open");
    const button = harness.editor.element.querySelector("button");
    assert.ok(button);
    button.click();
    await Promise.resolve();
    assert.deepEqual(commands.calls, [StartTurnCommandId]);
    contribution.dispose();
    assert.equal(harness.editor.element.querySelector(".zeta-empty-workspace-view"), null);
    harness.disposables.dispose();
    dom.window.close();
});
test("empty workspace contribution leaves project windows unchanged", () => {
    const dom = new JSDOM("<!doctype html><body></body>");
    const harness = createLayoutHarness(dom.window.document);
    const marker = dom.window.document.createElement("div");
    marker.textContent = "Project editor";
    harness.editor.setContent(marker);
    const contribution = harness.disposables.add(new EmptyWorkspaceContribution(new WorkspaceContextService({
        id: "project",
        uri: URI.file("C:\\project"),
    }), harness.editor, harness.layout, new TestCommandService()));
    assert.equal(harness.layout.isPartVisible("sidebar"), true);
    assert.equal(harness.layout.isPartVisible("auxiliarybar"), true);
    assert.equal(harness.editor.element.textContent, "Project editor");
    contribution.dispose();
    harness.disposables.dispose();
    dom.window.close();
});
