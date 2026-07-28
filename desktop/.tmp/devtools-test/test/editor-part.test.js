import assert from "node:assert/strict";
import test from "node:test";
import { JSDOM } from "jsdom";
import { DisposableOwner } from "../src/base/common/lifecycle.js";
import { URI } from "../src/base/common/uri.js";
import { EditorPaneMatch, EditorPaneVisibility, } from "../src/workbench/browser/parts/editor/editorPane.js";
import { EditorPaneRegistry, } from "../src/workbench/browser/parts/editor/editorRegistry.js";
const browserEnvironment = new JSDOM("<!doctype html><body></body>");
for (const [name, value] of Object.entries({
    window: browserEnvironment.window,
    document: browserEnvironment.window.document,
    Node: browserEnvironment.window.Node,
    Element: browserEnvironment.window.Element,
    HTMLElement: browserEnvironment.window.HTMLElement,
})) {
    Object.defineProperty(globalThis, name, {
        configurable: true,
        value,
    });
}
const { EditorOpenSupersededError, EditorPart, } = await import("../src/workbench/browser/parts/editor/editorPart.js");
test.after(() => browserEnvironment.window.close());
test("editor registry resolves defaults and explicit Open With choices", () => {
    const registry = new EditorPaneRegistry();
    const monaco = descriptor("zeta.editor.monaco", ".ts", () => new TestEditorPane("zeta.editor.monaco"));
    const prosemirror = descriptor("zeta.editor.prosemirror", ".md", () => new TestEditorPane("zeta.editor.prosemirror"));
    const monacoRegistration = registry.register(monaco);
    const prosemirrorRegistration = registry.register(prosemirror);
    const typescript = input("C:\\project\\main.ts");
    const markdown = input("C:\\project\\paper.md");
    assert.equal(registry.resolve(typescript), monaco);
    assert.equal(registry.resolve(markdown), prosemirror);
    assert.deepEqual(registry.getEditors(markdown), [
        prosemirror,
        monaco,
    ]);
    assert.equal(registry.resolve(markdown, {
        preferredEditorId: "zeta.editor.monaco",
    }), monaco);
    assert.throws(() => registry.resolve(markdown, {
        preferredEditorId: "zeta.editor.unknown",
    }), /Unknown editor pane/);
    assert.throws(() => registry.register(monaco), /already registered/);
    prosemirrorRegistration.dispose();
    assert.equal(registry.resolve(markdown), monaco);
    monacoRegistration.dispose();
    assert.throws(() => registry.resolve(markdown), /No editor can open/);
});
test("EditorPart switches panes only after the next input is ready", async () => {
    const dom = new JSDOM("<!doctype html><body></body>");
    const registry = new EditorPaneRegistry();
    const panes = [];
    registry.register(descriptor("zeta.editor.monaco", ".ts", () => trackPane(panes, "zeta.editor.monaco")));
    registry.register(descriptor("zeta.editor.prosemirror", ".md", () => trackPane(panes, "zeta.editor.prosemirror")));
    const editor = new EditorPart(dom.window.document, registry);
    dom.window.document.body.append(editor.element);
    const typescript = input("C:\\project\\main.ts");
    const monaco = await editor.openEditor(typescript);
    assert.equal(editor.activePane, monaco);
    assert.equal(editor.activeInput, typescript);
    assert.equal(editor.element.textContent, "zeta.editor.monaco");
    assert.deepEqual(panes[0]?.visibilities, [
        EditorPaneVisibility.Hidden,
        EditorPaneVisibility.Visible,
    ]);
    editor.layout({ width: 800, height: 600 });
    assert.deepEqual(panes[0]?.dimension, { width: 800, height: 600 });
    editor.focus();
    assert.equal(panes[0]?.focusCount, 1);
    const markdown = input("C:\\project\\paper.md");
    const prosemirror = await editor.openEditor(markdown);
    assert.equal(editor.activePane, prosemirror);
    assert.equal(editor.activeInput, markdown);
    assert.equal(editor.element.textContent, "zeta.editor.prosemirror");
    assert.equal(panes[0]?.disposed, true);
    assert.deepEqual(panes[0]?.visibilities.slice(-1), [
        EditorPaneVisibility.Hidden,
    ]);
    assert.deepEqual(panes[1]?.dimension, { width: 800, height: 600 });
    const content = dom.window.document.createElement("div");
    content.textContent = "Welcome";
    editor.setContent(content);
    assert.equal(editor.activePane, undefined);
    assert.equal(editor.activeInput, undefined);
    assert.equal(panes[1]?.disposed, true);
    assert.equal(editor.element.textContent, "Welcome");
    editor.dispose();
    dom.window.close();
});
test("EditorPart retains the active pane when a replacement fails", async () => {
    const dom = new JSDOM("<!doctype html><body></body>");
    const registry = new EditorPaneRegistry();
    const panes = [];
    registry.register(descriptor("zeta.editor.working", ".ok", () => trackPane(panes, "zeta.editor.working")));
    registry.register(descriptor("zeta.editor.failing", ".bad", () => {
        const pane = trackPane(panes, "zeta.editor.failing");
        pane.inputError = new Error("Unable to load input");
        return pane;
    }));
    const editor = new EditorPart(dom.window.document, registry);
    dom.window.document.body.append(editor.element);
    const workingInput = input("C:\\project\\document.ok");
    const workingPane = await editor.openEditor(workingInput);
    await assert.rejects(editor.openEditor(input("C:\\project\\document.bad")), /Unable to load input/);
    assert.equal(editor.activePane, workingPane);
    assert.equal(editor.activeInput, workingInput);
    assert.equal(panes[1]?.disposed, true);
    editor.dispose();
    dom.window.close();
});
test("EditorPart rejects an open superseded by ordinary content", async () => {
    const dom = new JSDOM("<!doctype html><body></body>");
    const registry = new EditorPaneRegistry();
    const pending = deferred();
    let slowPane;
    registry.register(descriptor("zeta.editor.slow", ".slow", () => {
        const pane = new TestEditorPane("zeta.editor.slow");
        pane.inputPromise = pending.promise;
        slowPane = pane;
        return pane;
    }));
    const editor = new EditorPart(dom.window.document, registry);
    dom.window.document.body.append(editor.element);
    const opening = editor.openEditor(input("C:\\project\\document.slow"));
    const content = dom.window.document.createElement("div");
    content.textContent = "Replacement";
    editor.setContent(content);
    assert.equal(slowPane?.inputSignal?.aborted, true);
    pending.resolve(undefined);
    await assert.rejects(opening, EditorOpenSupersededError);
    assert.equal(editor.activePane, undefined);
    assert.equal(editor.element.textContent, "Replacement");
    editor.dispose();
    dom.window.close();
});
class TestEditorPane extends DisposableOwner {
    id;
    visibilities = [];
    inputError;
    inputPromise;
    inputSignal;
    dimension;
    focusCount = 0;
    disposed = false;
    constructor(id) {
        super();
        this.id = id;
        this.defer(() => {
            this.disposed = true;
        });
    }
    create(parent) {
        const element = parent.ownerDocument.createElement("div");
        element.textContent = this.id;
        parent.append(element);
    }
    async setInput(_input, signal) {
        this.inputSignal = signal;
        if (this.inputError)
            throw this.inputError;
        await this.inputPromise;
    }
    clearInput() { }
    layout(dimension) {
        this.dimension = {
            width: dimension.width,
            height: dimension.height,
        };
    }
    setVisible(visibility) {
        this.visibilities.push(visibility);
    }
    focus() {
        this.focusCount += 1;
    }
}
function descriptor(id, defaultExtension, create) {
    return {
        id,
        name: id,
        canOpen: (candidate) => candidate.resource.path.endsWith(defaultExtension)
            ? EditorPaneMatch.Default
            : EditorPaneMatch.Optional,
        create,
    };
}
function input(path) {
    return { resource: URI.file(path) };
}
function trackPane(panes, id) {
    const pane = new TestEditorPane(id);
    panes.push(pane);
    return pane;
}
function deferred() {
    let resolvePromise;
    const promise = new Promise((resolve) => {
        resolvePromise = resolve;
    });
    return { promise, resolve: resolvePromise };
}
