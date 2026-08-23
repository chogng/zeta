import assert from "node:assert/strict";
import test from "node:test";
import { JSDOM } from "jsdom";
import { URI } from "../../../base/common/uri.js";
import { TextModel } from "../../common/model/textModel.js";
import { type TextModelReference } from "../../common/services/textModelService.js";

const browserEnvironment = new JSDOM("<!doctype html><body></body>");
for (const [name, value] of Object.entries({
	window: browserEnvironment.window,
	document: browserEnvironment.window.document,
	Node: browserEnvironment.window.Node,
	Element: browserEnvironment.window.Element,
	HTMLElement: browserEnvironment.window.HTMLElement,
	Event: browserEnvironment.window.Event,
	InputEvent: browserEnvironment.window.InputEvent,
})) Object.defineProperty(globalThis, name, { configurable: true, value });

await import("../../contrib/codeEditorPart.contribution.js");
const { EditorPart } = await import("../../browser/editorPart.js");

test.after(() => browserEnvironment.window.close());

test("minimal text editor assembly creates only the engine surface", () => {
	const dom = new JSDOM("<!doctype html><body><main></main></body>");
	dom.window.HTMLCanvasElement.prototype.getContext = () => null;
	const container = dom.window.document.querySelector<HTMLElement>("main")!;
	const model = new TextModel("const value = (1);");
	const reference = modelReference(URI.file("C:\\project\\minimal.ts"), model);
	const editor = new EditorPart({
		container,
		input: { resource: reference.resource, label: "minimal.ts" },
		languageId: "typescript",
		modelReference: reference,
		placeholder: "Not installed",
	});
	editor.layout({ width: 480, height: 120 });

	assert.ok(container.querySelector(".stanza-editor"));
	assert.ok(container.querySelector(".stanza-editor-input"));
	assert.equal(container.querySelector(".stanza-editor-token"), null);
	assert.equal(container.querySelector(".stanza-editor-decoration"), null);
	assert.equal(container.querySelector(".stanza-editor-fold-toggle"), null);
	assert.equal(container.querySelector(".stanza-editor-completion"), null);
	assert.equal(container.querySelector(".stanza-editor-placeholder-text"), null);

	const copy = new dom.window.Event("copy", { bubbles: true, cancelable: true });
	editor.textInput.element.dispatchEvent(copy);
	assert.equal(copy.defaultPrevented, false);

	editor.dispose();
	dom.window.close();
});

function modelReference(resource: URI, model: TextModel): TextModelReference {
	let disposed = false;
	const dispose = () => {
		if (disposed) return;
		disposed = true;
		model.dispose();
	};
	return {
		resource,
		model,
		get isDirty() { return false; },
		get hasExternalChange() { return false; },
		onDidChangeDirty: () => ({ dispose() {}, [Symbol.dispose]() {} }),
		onDidChangeExternalChange: () => ({ dispose() {}, [Symbol.dispose]() {} }),
		save: () => Promise.resolve(),
		revert: () => Promise.resolve(),
		dispose,
		[Symbol.dispose]: dispose,
	};
}
