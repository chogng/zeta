import assert from "node:assert/strict";
import test from "node:test";
import { JSDOM } from "jsdom";
import { DisposableOwner } from "../../../../base/common/lifecycle.js";
import type { CodeEditorContributionContext } from "../../../browser/widget/codeEditor/codeEditorContributions.js";
import { EditorSelectionController } from "../../../common/cursor/editorSelectionController.js";
import { TextSelection, TextSelectionSet } from "../../../common/core/selection.js";
import { TextPosition } from "../../../common/core/text.js";
import { TextModel } from "../../../common/model/textModel.js";

const browserEnvironment = new JSDOM("<!doctype html><body></body>");
for (const [name, value] of Object.entries({
	window: browserEnvironment.window,
	document: browserEnvironment.window.document,
	Node: browserEnvironment.window.Node,
	Element: browserEnvironment.window.Element,
	HTMLElement: browserEnvironment.window.HTMLElement,
	Event: browserEnvironment.window.Event,
	InputEvent: browserEnvironment.window.InputEvent,
	KeyboardEvent: browserEnvironment.window.KeyboardEvent,
})) {
	Object.defineProperty(globalThis, name, { configurable: true, value });
}

const { CodeEditorWidget } = await import("../../../browser/widget/codeEditor/codeEditorWidget.js");
const { CodeEditorContributionInstantiation } = await import("../../../browser/widget/codeEditor/codeEditorContributions.js");
const { createServiceIdentifier, InstantiationService, ServiceCollection, SyncDescriptor } = await import("../../../../platform/instantiation/common/instantiation.js");
await import("../../../contrib/placeholderText/browser/placeholderTextController.js");

test.after(() => browserEnvironment.window.close());

test("CodeEditorWidget owns one canonical browser editing surface", () => {
	const dom = new JSDOM("<!doctype html><body><main></main></body>");
	dom.window.HTMLCanvasElement.prototype.getContext = () => null;
	const container = requiredElement(dom.window.document, "main");
	using model = new TextModel("alpha");
	using selections = new EditorSelectionController(model, TextSelectionSet.single(TextSelection.collapsedAt(TextPosition.at(0, 0))));
	const editor = new CodeEditorWidget({ container, model, selectionController: selections, lineHeight: 20, ariaLabel: "Code" });

	editor.layout({ width: 320, height: 80 });

	assert.equal(editor.element.parentElement, container);
	assert.equal(editor.element.getAttribute("aria-label"), "Code");
	assert.equal(editor.textInput.element.getAttribute("aria-label"), "Code");
	assert.deepEqual(editor.viewport.viewportLayout.viewportSize, { width: 320, height: 80 });

	editor.dispose();
	assert.equal(editor.element.isConnected, false);
	assert.equal(model.getText(), "alpha");
	assert.equal(selections.textModel, model);
	dom.window.close();
});

test("CodeEditorWidget owns padding, placeholder, and current-line presentation for embedded editors", () => {
	const dom = new JSDOM("<!doctype html><body><main></main></body>");
	dom.window.HTMLCanvasElement.prototype.getContext = () => null;
	const container = requiredElement(dom.window.document, "main");
	using model = new TextModel("alpha");
	using selections = new EditorSelectionController(model, TextSelectionSet.single(TextSelection.collapsedAt(TextPosition.at(0, 0))));
	using editor = new CodeEditorWidget({
		container,
		model,
		selectionController: selections,
		lineHeight: 20,
		placeholder: "Ask Zeta",
		viewport: { presentation: "embedded", padding: { top: 20, right: 20, bottom: 20, left: 20 } },
	});

	editor.layout({ width: 320, height: 40 });

	assert.equal(editor.element.querySelector(".stanza-editor-line.active"), null);
	assert.ok(editor.element.querySelector(".stanza-editor-caret"));
	assert.equal(requiredElement<HTMLElement>(editor.element, ".stanza-editor-lines").style.transform, "translate3d(0, 20px, 0)");
	assert.equal(editor.element.style.getPropertyValue("--stanza-editor-padding-left"), "20px");
	assert.equal(editor.element.style.getPropertyValue("--stanza-editor-padding-right"), "20px");
	assert.equal(requiredElement<HTMLElement>(editor.element, ".stanza-editor-placeholder-text").style.top, "20px");
	assert.equal(editor.viewport.viewportLayout.contentSize.height, 60);
	dom.window.close();
});

test("CodeEditorWidget stages and owns per-instance contributions", () => {
	const dom = new JSDOM("<!doctype html><body><main></main></body>");
	dom.window.HTMLCanvasElement.prototype.getContext = () => null;
	const container = requiredElement(dom.window.document, "main");
	using model = new TextModel("alpha");
	using selections = new EditorSelectionController(model, TextSelectionSet.single(TextSelection.collapsedAt(TextPosition.at(0, 0))));
	const events: string[] = [];
	const service = { kind: "test" };
	const serviceId = createServiceIdentifier<typeof service>("test.codeEditorContribution");
	const services = new ServiceCollection();
	services.set(serviceId, service);
	const instantiationService = new InstantiationService(services);
	const state = { events, model, service };
	using editor = new CodeEditorWidget({
		container,
		model,
		selectionController: selections,
		lineHeight: 20,
		instantiationService,
		contributions: [
			{
				id: "test.eager",
				instantiation: CodeEditorContributionInstantiation.Eager,
				descriptor: new SyncDescriptor(TestCodeEditorContribution, { staticArguments: [state, "eager"], serviceDependencies: [serviceId] }),
			},
			{
				id: "test.lazy",
				instantiation: CodeEditorContributionInstantiation.Lazy,
				descriptor: new SyncDescriptor(TestCodeEditorContribution, { staticArguments: [state, "lazy"], serviceDependencies: [serviceId] }),
			},
		],
	});

	assert.deepEqual(events, ["eager:create"]);
	assert.ok(editor.contributions.get("test.lazy"));
	assert.deepEqual(events, ["eager:create", "lazy:create"]);
	editor.dispose();
	assert.deepEqual(events, ["eager:create", "lazy:create", "lazy:dispose", "eager:dispose"]);
	dom.window.close();
});

class TestCodeEditorContribution extends DisposableOwner {
	constructor(
		private readonly state: { readonly events: string[]; readonly model: TextModel; readonly service: { readonly kind: string } },
		private readonly id: string,
		context: CodeEditorContributionContext,
		service: { readonly kind: string },
	) {
		super();
		assert.equal(context.model, state.model);
		assert.equal(service, state.service);
		state.events.push(`${id}:create`);
		this.defer(() => state.events.push(`${id}:dispose`));
	}
}

test("CodeEditorWidget rejects a selection controller from another model", () => {
	const dom = new JSDOM("<!doctype html><body><main></main></body>");
	dom.window.HTMLCanvasElement.prototype.getContext = () => null;
	const container = requiredElement(dom.window.document, "main");
	using model = new TextModel("alpha");
	using otherModel = new TextModel("beta");
	using selections = new EditorSelectionController(otherModel, TextSelectionSet.single(TextSelection.collapsedAt(TextPosition.at(0, 0))));

	assert.throws(() => new CodeEditorWidget({ container, model, selectionController: selections, lineHeight: 20 }), /must match/);
	dom.window.close();
});

test("CodeEditorWidget leaves text drops available to its host", () => {
	const dom = new JSDOM("<!doctype html><body><main></main></body>");
	dom.window.HTMLCanvasElement.prototype.getContext = () => null;
	const container = requiredElement(dom.window.document, "main");
	using model = new TextModel("alpha");
	using selections = new EditorSelectionController(model, TextSelectionSet.single(TextSelection.collapsedAt(TextPosition.at(0, 0))));
	using editor = new CodeEditorWidget({ container, model, selectionController: selections, lineHeight: 20 });
	const drop = textDropEvent(dom.window, "dropped");

	editor.element.dispatchEvent(drop);

	assert.equal(drop.defaultPrevented, false);
	assert.equal(model.getText(), "alpha");
	dom.window.close();
});

function textDropEvent(targetWindow: typeof browserEnvironment.window, text: string): DragEvent {
	const event = new targetWindow.Event("drop", { bubbles: true, cancelable: true });
	Object.defineProperty(event, "dataTransfer", {
		value: {
			types: ["text/plain"],
			getData(type: string): string {
				return type === "text/plain" ? text : "";
			},
		},
	});
	return event as unknown as DragEvent;
}

function requiredElement<T extends Element = HTMLElement>(root: ParentNode, selector: string): T {
	const element = root.querySelector<T>(selector);
	assert.ok(element);
	return element;
}
