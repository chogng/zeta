import assert from "node:assert/strict";
import test from "node:test";
import { JSDOM } from "jsdom";
import { Disposable, toDisposable } from "../../../../base/common/lifecycle.js";
import type { CodeEditorContributionContext } from "../../../browser/widget/codeEditor/codeEditorContributions.js";
import { Position } from '../../../common/core/position.js';
import { Range } from '../../../common/core/range.js';
import { Selection } from '../../../common/core/selection.js';
import { TextModel } from "../../../common/model/textModel.js";

const browserEnvironment = new JSDOM("<!doctype html><body></body>");
class TestResizeObserver {
	observe(): void {}
	unobserve(): void {}
	disconnect(): void {}
}
for (const [name, value] of Object.entries({
	window: browserEnvironment.window,
	document: browserEnvironment.window.document,
	Node: browserEnvironment.window.Node,
	Element: browserEnvironment.window.Element,
	HTMLElement: browserEnvironment.window.HTMLElement,
	Event: browserEnvironment.window.Event,
	InputEvent: browserEnvironment.window.InputEvent,
	KeyboardEvent: browserEnvironment.window.KeyboardEvent,
	ResizeObserver: TestResizeObserver,
})) {
	Object.defineProperty(globalThis, name, { configurable: true, value });
}

const { CodeEditorWidget } = await import("../../../browser/widget/codeEditor/codeEditorWidget.js");
const { EditorContributionInstantiation } = await import('../../../browser/editorExtensions.js');
const { createServiceIdentifier, IInstantiationService, ServiceContainer, ServiceConstructionDescriptor } = await import("../../../../platform/instantiation/common/instantiation.js");
const { PlaceholderTextContribution } = await import("../../../contrib/placeholderText/browser/placeholderTextContribution.js");
const { createEditorBrowserServices } = await import('../../../browser/services/contribution.js');
await import("../../../contrib/placeholderText/browser/placeholderText.contribution.js");
await import('../../../contrib/inPlaceReplace/browser/inPlaceReplace.js');

test.after(() => browserEnvironment.window.close());

test("CodeEditorWidget owns one canonical browser editing surface", () => {
	const dom = new JSDOM("<!doctype html><body><main></main></body>");
	dom.window.HTMLCanvasElement.prototype.getContext = () => null;
	const container = requiredElement(dom.window.document, "main");
	using model = new TextModel("alpha");
	const editor = new CodeEditorWidget({ container, model, input: { resource: model.uri }, languageId: model.getLanguageId(), lineHeight: 20, ariaLabel: "Code" });

	editor.layout({ width: 320, height: 80 });
	const fontTarget = dom.window.document.createElement('span');
	editor.applyFontInfo(fontTarget);

	assert.equal(editor.element.parentElement, container);
	assert.equal(editor.element.getAttribute("aria-label"), "Code");
	assert.equal(editor.view.element.getAttribute("aria-label"), "Code");
	assert.deepEqual(editor.viewport.viewportLayout.viewportSize, { width: 320, height: 80 });
	assert.equal(fontTarget.style.fontFamily, editor.element.style.fontFamily);
	assert.equal(fontTarget.style.fontFeatureSettings, editor.element.style.fontFeatureSettings);

	editor.dispose();
	assert.equal(editor.element.isConnected, false);
	assert.equal(model.getText(), "alpha");
	assert.throws(() => editor.selections.textModel, /already disposed/);
	dom.window.close();
});

test('CodeEditorWidget publishes service lifecycle in construction order', () => {
	const dom = new JSDOM('<!doctype html><body><main></main></body>');
	dom.window.HTMLCanvasElement.prototype.getContext = () => null;
	using model = new TextModel('alpha');
	const services = createEditorBrowserServices();
	using service = services.codeEditorService;
	const events: string[] = [];
	using willCreate = service.onWillCreateCodeEditor(() => events.push('will'));
	using add = service.onCodeEditorAdd(editor => events.push(`add:${editor.getId()}`));
	using remove = service.onCodeEditorRemove(editor => events.push(`remove:${editor.getId()}`));
	const editor = new CodeEditorWidget({
		container: requiredElement(dom.window.document, 'main'),
		model,
		input: { resource: model.uri },
		languageId: model.getLanguageId(),
		lineHeight: 20,
		codeEditorService: service,
	});

	assert.deepEqual(events, ['will', `add:${editor.getId()}`]);
	assert.strictEqual(service.getActiveCodeEditor(), editor);
	editor.dispose();
	assert.deepEqual(events, ['will', `add:${editor.getId()}`, `remove:${editor.getId()}`]);
	assert.equal(service.getActiveCodeEditor(), null);
	dom.window.close();
});

test('CodeEditorWidget exposes editor-owned scroll geometry', () => {
	const dom = new JSDOM('<!doctype html><body><main></main></body>');
	dom.window.HTMLCanvasElement.prototype.getContext = () => null;
	using model = new TextModel('one\ntwo\nthree\nfour\nfive');
	using editor = new CodeEditorWidget({
		container: requiredElement(dom.window.document, 'main'),
		model,
		input: { resource: model.uri },
		languageId: model.getLanguageId(),
		lineHeight: 20,
	});
	editor.layout({ width: 240, height: 40 });
	editor.setScrollTop(40);

	assert.equal(editor.getScrollTop(), 40);
	assert.equal(editor.getContentHeight(), 100);
	assert.equal(editor.hasPendingScrollAnimation(), false);
	assert.equal(editor.getTopForLineNumber(3), 40);
	assert.equal(editor.getTopForPosition(3, 2), 40);
	assert.equal(editor.getBottomForLineNumber(3), 60);
	assert.deepEqual(editor.getVisibleRanges(), [new Range(3, 1, 4, 5)]);
	dom.window.close();
});

test('CodeEditorWidget isolates model decorations by editor lifetime', () => {
	const dom = new JSDOM('<!doctype html><body><main></main><aside></aside></body>');
	dom.window.HTMLCanvasElement.prototype.getContext = () => null;
	using model = new TextModel('alpha');
	const first = new CodeEditorWidget({
		container: requiredElement(dom.window.document, 'main'),
		model,
		input: { resource: model.uri },
		languageId: model.getLanguageId(),
		lineHeight: 20,
	});
	const second = new CodeEditorWidget({
		container: requiredElement(dom.window.document, 'aside'),
		model,
		input: { resource: model.uri },
		languageId: model.getLanguageId(),
		lineHeight: 20,
	});
	let firstId = '';
	let secondId = '';
	first.changeDecorations(accessor => {
		firstId = accessor.addDecoration(new Range(1, 1, 1, 3), { description: 'first editor' });
	});
	second.changeDecorations(accessor => {
		secondId = accessor.addDecoration(new Range(1, 3, 1, 5), { description: 'second editor' });
	});

	assert.deepEqual(model.getAllDecorations().map(decoration => decoration.id), [firstId, secondId]);
	first.dispose();
	assert.equal(model.getDecorationRange(firstId), null);
	assert.deepEqual(model.getDecorationRange(secondId), new Range(1, 3, 1, 5));
	second.removeDecorations([secondId]);
	assert.equal(model.getAllDecorations().length, 0);
	second.dispose();
	dom.window.close();
});

test('CodeEditorWidget owns decoration collections and reveals without moving selection', () => {
	const dom = new JSDOM('<!doctype html><body><main></main></body>');
	dom.window.HTMLCanvasElement.prototype.getContext = () => null;
	using model = new TextModel('one\ntwo\nthree\nfour');
	using editor = new CodeEditorWidget({
		container: requiredElement(dom.window.document, 'main'),
		model,
		input: { resource: model.uri },
		languageId: model.getLanguageId(),
		lineHeight: 20,
	});
	editor.layout({ width: 240, height: 40 });
	const selection = Selection.fromPositions(new Position(1, 2));
	editor.setSelection(selection);
	const decorations = editor.createDecorationsCollection([{ range: new Range(2, 1, 2, 4), options: { description: 'owned collection' } }]);

	editor.revealRange(new Range(4, 1, 4, 5));

	assert.deepEqual(editor.getSelection(), selection);
	assert.deepEqual(decorations.getRange(0), new Range(2, 1, 2, 4));
	decorations.clear();
	assert.equal(model.getAllDecorations().length, 0);
	dom.window.close();
});

test('CodeEditorWidget runs in-place replacement through the registered contribution', async () => {
	const dom = new JSDOM('<!doctype html><body><main></main></body>');
	dom.window.HTMLCanvasElement.prototype.getContext = () => null;
	const container = requiredElement(dom.window.document, 'main');
	using model = new TextModel('value 1');
	using editor = new CodeEditorWidget({ container, model, input: { resource: model.uri }, languageId: model.getLanguageId(), lineHeight: 20 });
	editor.selections.setSelections([Selection.fromPositions(new Position(1, 7), new Position(1, 8))]);

	const next = new dom.window.KeyboardEvent('keydown', { bubbles: true, cancelable: true, key: '.', ctrlKey: true, shiftKey: true }) as unknown as KeyboardEvent;
	editor.view.element.dispatchEvent(next);
	assert.equal(next.defaultPrevented, true);
	await waitForText(model, 'value 2');

	const previous = new dom.window.KeyboardEvent('keydown', { bubbles: true, cancelable: true, key: ',', ctrlKey: true, shiftKey: true }) as unknown as KeyboardEvent;
	editor.view.element.dispatchEvent(previous);
	assert.equal(previous.defaultPrevented, true);
	await waitForText(model, 'value 1');
	dom.window.close();
});

test("CodeEditorWidget owns padding, placeholder, and current-line presentation for embedded editors", () => {
	const dom = new JSDOM("<!doctype html><body><main></main></body>");
	dom.window.HTMLCanvasElement.prototype.getContext = () => null;
	const container = requiredElement(dom.window.document, "main");
	using model = new TextModel("alpha");
	using editor = new CodeEditorWidget({
		container,
		model,
		input: { resource: model.uri },
		languageId: model.getLanguageId(),
		lineHeight: 20,
		placeholder: "Ask Zeta",
		presentation: "embedded",
		padding: { top: 20, bottom: 20 },
	});

	editor.layout({ width: 320, height: 40 });

	assert.equal(editor.element.querySelector(".view-line.active"), null);
	assert.ok(editor.element.querySelector(".stanza-editor-caret"));
	assert.equal(requiredElement<HTMLElement>(editor.element, ".stanza-editor-lines").style.top, "20px");
	assert.equal(requiredElement<HTMLElement>(editor.element, ".stanza-editor-lines").style.transform, "");
	assert.equal(editor.element.style.getPropertyValue("--stanza-editor-padding-left"), "12px");
	assert.equal(editor.element.style.getPropertyValue("--stanza-editor-padding-right"), "12px");
	assert.equal(requiredElement<HTMLElement>(editor.element, ".stanza-editor-placeholder-text").style.top, "20px");
	assert.equal(editor.viewport.viewportLayout.contentSize.height, 60);
	dom.window.close();
});

test("PlaceholderTextContribution follows model emptiness and editor layout", () => {
	const dom = new JSDOM("<!doctype html><body><main></main></body>");
	dom.window.HTMLCanvasElement.prototype.getContext = () => null;
	const container = requiredElement(dom.window.document, "main");
	using model = new TextModel();
	using editor = new CodeEditorWidget({
		container,
		model,
		input: { resource: model.uri },
		languageId: model.getLanguageId(),
		lineHeight: 20,
		placeholder: "Ask Zeta",
		padding: { top: 8, bottom: 8 },
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
		input: { resource: model.uri },
		languageId: model.getLanguageId(),
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

test("CodeEditorWidget creates one selection controller for its model", () => {
	const dom = new JSDOM("<!doctype html><body><main></main></body>");
	dom.window.HTMLCanvasElement.prototype.getContext = () => null;
	const container = requiredElement(dom.window.document, "main");
	using model = new TextModel("alpha");
	using editor = new CodeEditorWidget({ container, model, input: { resource: model.uri }, languageId: model.getLanguageId(), lineHeight: 20 });
	assert.equal(editor.selections.textModel, model);
	dom.window.close();
});

test("CodeEditorWidget leaves text drops available to its host", () => {
	const dom = new JSDOM("<!doctype html><body><main></main></body>");
	dom.window.HTMLCanvasElement.prototype.getContext = () => null;
	const container = requiredElement(dom.window.document, "main");
	using model = new TextModel("alpha");
	using editor = new CodeEditorWidget({ container, model, input: { resource: model.uri }, languageId: model.getLanguageId(), lineHeight: 20 });
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

async function waitForText(model: TextModel, expected: string): Promise<void> {
	for (let attempt = 0; attempt < 20; attempt += 1) {
		if (model.getText() === expected) return;
		await new Promise(resolve => setTimeout(resolve, 0));
	}
	assert.equal(model.getText(), expected);
}

function requiredElement<T extends Element = HTMLElement>(root: ParentNode, selector: string): T {
	const element = root.querySelector<T>(selector);
	assert.ok(element);
	return element;
}
