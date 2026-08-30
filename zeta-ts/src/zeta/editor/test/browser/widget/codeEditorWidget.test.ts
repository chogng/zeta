import assert from "node:assert/strict";
import test from "node:test";
import { JSDOM } from "jsdom";
import { Disposable, toDisposable } from "../../../../base/common/lifecycle.js";
import type { CodeEditorContributionContext } from "../../../browser/widget/codeEditor/codeEditorContributions.js";
import { CursorsController } from "../../../common/cursor/cursor.js";
import { Selection } from "../../../common/core/selection.js";
import { SelectionSet } from "../../../common/cursor/selectionSet.js";
import { Position } from "../../../common/core/position.js";
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
const { EditorContributionInstantiation } = await import('../../../browser/editorExtensions.js');
const { createServiceIdentifier, IInstantiationService, ServiceContainer, ServiceConstructionDescriptor } = await import("../../../../platform/instantiation/common/instantiation.js");
const { PlaceholderTextContribution } = await import("../../../contrib/placeholderText/browser/placeholderTextContribution.js");
await import("../../../contrib/placeholderText/browser/placeholderText.contribution.js");

test.after(() => browserEnvironment.window.close());

test("CodeEditorWidget owns one canonical browser editing surface", () => {
	const dom = new JSDOM("<!doctype html><body><main></main></body>");
	dom.window.HTMLCanvasElement.prototype.getContext = () => null;
	const container = requiredElement(dom.window.document, "main");
	using model = new TextModel("alpha");
	using selections = new CursorsController(model, SelectionSet.single(Selection.fromPositions(new Position((0) + 1, (0) + 1))));
	const editor = new CodeEditorWidget({ container, model, selectionController: selections, lineHeight: 20, ariaLabel: "Code" });

	editor.layout({ width: 320, height: 80 });

	assert.equal(editor.element.parentElement, container);
	assert.equal(editor.element.getAttribute("aria-label"), "Code");
	assert.equal(editor.view.element.getAttribute("aria-label"), "Code");
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
	using selections = new CursorsController(model, SelectionSet.single(Selection.fromPositions(new Position((0) + 1, (0) + 1))));
	using editor = new CodeEditorWidget({
		container,
		model,
		selectionController: selections,
		lineHeight: 20,
		placeholder: "Ask Zeta",
		viewport: { presentation: "embedded", padding: { top: 20, right: 20, bottom: 20, left: 20 } },
	});

	editor.layout({ width: 320, height: 40 });

	assert.equal(editor.element.querySelector(".view-line.active"), null);
	assert.ok(editor.element.querySelector(".stanza-editor-caret"));
	assert.equal(requiredElement<HTMLElement>(editor.element, ".stanza-editor-lines").style.top, "20px");
	assert.equal(requiredElement<HTMLElement>(editor.element, ".stanza-editor-lines").style.transform, "");
	assert.equal(editor.element.style.getPropertyValue("--stanza-editor-padding-left"), "20px");
	assert.equal(editor.element.style.getPropertyValue("--stanza-editor-padding-right"), "20px");
	assert.equal(requiredElement<HTMLElement>(editor.element, ".stanza-editor-placeholder-text").style.top, "20px");
	assert.equal(editor.viewport.viewportLayout.contentSize.height, 60);
	dom.window.close();
});

test("PlaceholderTextContribution follows model emptiness and editor layout", () => {
	const dom = new JSDOM("<!doctype html><body><main></main></body>");
	dom.window.HTMLCanvasElement.prototype.getContext = () => null;
	const container = requiredElement(dom.window.document, "main");
	using model = new TextModel();
	using selections = new CursorsController(model, SelectionSet.single(Selection.fromPositions(new Position((0) + 1, (0) + 1))));
	using editor = new CodeEditorWidget({
		container,
		model,
		selectionController: selections,
		lineHeight: 20,
		placeholder: "Ask Zeta",
		viewport: { padding: { top: 8, right: 12, bottom: 8, left: 12 } },
	});

	editor.layout({ width: 320, height: 80 });
	const placeholder = requiredElement<HTMLElement>(editor.element, ".stanza-editor-placeholder-text");
	assert.strictEqual(PlaceholderTextContribution.get(editor), editor.getContribution(PlaceholderTextContribution.ID));
	assert.deepEqual({
		display: placeholder.style.display,
		left: placeholder.style.left,
		top: placeholder.style.top,
		width: placeholder.style.width,
		lineHeight: placeholder.style.lineHeight,
	}, {
		display: "block",
		left: "45px",
		top: "8px",
		width: "275px",
		lineHeight: "20px",
	});

	model.reset("alpha");
	assert.equal(placeholder.style.display, "none");
	model.reset("");
	assert.equal(placeholder.style.display, "block");
	dom.window.close();
});

test("CodeEditorWidget stages and owns per-instance contributions", () => {
	const dom = new JSDOM("<!doctype html><body><main></main></body>");
	dom.window.HTMLCanvasElement.prototype.getContext = () => null;
	const container = requiredElement(dom.window.document, "main");
	using model = new TextModel("alpha");
	using selections = new CursorsController(model, SelectionSet.single(Selection.fromPositions(new Position((0) + 1, (0) + 1))));
	const events: string[] = [];
	const service = { kind: "test" };
	const serviceId = createServiceIdentifier<typeof service>("test.codeEditorContribution");
	const services = new ServiceContainer();
	services.registerInstance(serviceId, service);
	const instantiationService = services;
	const state = { events, instantiationService, model, service };
	using editor = new CodeEditorWidget({
		container,
		model,
		selectionController: selections,
		lineHeight: 20,
		instantiationService,
		contributions: [
			{
				id: "test.eager",
				instantiation: EditorContributionInstantiation.Eager,
				descriptor: new ServiceConstructionDescriptor(TestCodeEditorContribution, { staticArguments: [state, "eager"], serviceDependencies: [serviceId, IInstantiationService] }),
			},
			{
				id: "test.lazy",
				instantiation: EditorContributionInstantiation.Lazy,
				descriptor: new ServiceConstructionDescriptor(TestCodeEditorContribution, { staticArguments: [state, "lazy"], serviceDependencies: [serviceId, IInstantiationService] }),
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

class TestCodeEditorContribution extends Disposable {
	constructor(
		private readonly state: { readonly events: string[]; readonly instantiationService: InstanceType<typeof ServiceContainer>; readonly model: TextModel; readonly service: { readonly kind: string } },
		private readonly id: string,
		context: CodeEditorContributionContext,
		service: { readonly kind: string },
		instantiationService: InstanceType<typeof ServiceContainer>,
	) {
		super();
		assert.equal(context.model, state.model);
		assert.equal(service, state.service);
		assert.notEqual(instantiationService, state.instantiationService);
		assert.equal(instantiationService.get(IInstantiationService), instantiationService);
		state.events.push(`${id}:create`);
		this._register(toDisposable(() => state.events.push(`${id}:dispose`)));
	}
}

test("CodeEditorWidget rejects a selection controller from another model", () => {
	const dom = new JSDOM("<!doctype html><body><main></main></body>");
	dom.window.HTMLCanvasElement.prototype.getContext = () => null;
	const container = requiredElement(dom.window.document, "main");
	using model = new TextModel("alpha");
	using otherModel = new TextModel("beta");
	using selections = new CursorsController(otherModel, SelectionSet.single(Selection.fromPositions(new Position((0) + 1, (0) + 1))));

	assert.throws(() => new CodeEditorWidget({ container, model, selectionController: selections, lineHeight: 20 }), /must match/);
	dom.window.close();
});

test("CodeEditorWidget leaves text drops available to its host", () => {
	const dom = new JSDOM("<!doctype html><body><main></main></body>");
	dom.window.HTMLCanvasElement.prototype.getContext = () => null;
	const container = requiredElement(dom.window.document, "main");
	using model = new TextModel("alpha");
	using selections = new CursorsController(model, SelectionSet.single(Selection.fromPositions(new Position((0) + 1, (0) + 1))));
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
