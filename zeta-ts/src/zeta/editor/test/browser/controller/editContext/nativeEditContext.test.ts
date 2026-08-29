import assert from "node:assert/strict";
import test from "node:test";
import { JSDOM } from "jsdom";
import { Emitter } from "../../../../../base/common/event.js";
import { TextSelection, TextSelectionSet } from "../../../../common/core/selection.js";
import { TextPosition } from "../../../../common/core/text.js";
import { TextModel } from "../../../../common/model/textModel.js";
import { type IAccessibilityService } from "../../../../../platform/accessibility/common/accessibility.js";
import { type TextMeasurer } from "../../../../browser/config/fontMeasurements.js";
import { SemanticTokenPresentation, type BracketColorizationSource, type SemanticTokenSource } from "../../../../browser/viewparts/viewLines/semanticTokenPresentation.js";

class FixedTextMeasurer implements TextMeasurer {
	readonly horizontalPadding = 24;
	readonly contentLeftPadding = 12;

	refresh(): boolean {
		return false;
	}

	measureLineWidth(text: string): number {
		return [...text].length * 10;
	}
}

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

const { NATIVE_TEXT_WINDOW_LENGTH, createNativeTextWindow, isLowSurrogate } = await import("../../../../browser/controller/editContext/native/nativeEditContextUtils.js");
const { NativeEditContextRegistry } = await import("../../../../browser/controller/editContext/native/nativeEditContextRegistry.js");
const { SimpleScreenReaderContent } = await import("../../../../browser/controller/editContext/native/screenReaderContentSimple.js");
const { RichScreenReaderContent } = await import("../../../../browser/controller/editContext/native/screenReaderContentRich.js");
const { createScreenReaderContentState, modelOffsetAtContentOffset } = await import("../../../../browser/controller/editContext/native/screenReaderUtils.js");
const { ScreenReaderSupport } = await import("../../../../browser/controller/editContext/native/screenReaderSupport.js");
const { EditorViewport } = await import("../../../../browser/view.js");

test.after(() => browserEnvironment.window.close());

test("native text windows retain the active range and never split UTF-16 code points", () => {
	const source = `${"a".repeat(20_000)}😀${"b".repeat(20_000)}`;
	const window = createNativeTextWindow(source, 20_000, 20_000);

	assert.equal(window.endOffset - window.startOffset, NATIVE_TEXT_WINDOW_LENGTH);
	assert.equal(source.slice(window.startOffset, window.endOffset).includes("😀"), true);
	assert.equal(isLowSurrogate(source.charCodeAt(window.startOffset)), false);
	assert.equal(isLowSurrogate(source.charCodeAt(window.endOffset)), false);
});

test("native edit-context registry owns element and owner-id registrations independently", () => {
	const element = browserEnvironment.window.document.createElement("div");
	const context = {} as never;
	const ownerId = `native-test-${Math.random()}`;
	const byElement = NativeEditContextRegistry.register(element, context);
	const byOwnerId = NativeEditContextRegistry.register(ownerId, context);
	try {
		assert.equal(NativeEditContextRegistry.get(element), context);
		assert.equal(NativeEditContextRegistry.get(ownerId), context);
		assert.throws(
			() => NativeEditContextRegistry.register(ownerId, {} as never),
			/Native EditContext owner/,
		);
	} finally {
		byOwnerId.dispose();
		byElement.dispose();
	}
	assert.equal(NativeEditContextRegistry.get(element), undefined);
	assert.equal(NativeEditContextRegistry.get(ownerId), undefined);
});

test("native screen-reader projections preserve empty text, lines, and DOM selection offsets", () => {
	const dom = new JSDOM("<!doctype html><body><main></main></body>");
	const host = dom.window.document.querySelector("main")!;
	using model = new TextModel("alpha\nbeta");
	const state = createScreenReaderContentState(
		model,
		TextSelection.from(TextPosition.at(0, 2), TextPosition.at(1, 2)),
	);
	using simple = new SimpleScreenReaderContent(host);
	simple.sync(state);
	assert.equal(simple.element.textContent, "alpha\nbeta");
	assert.equal(simple.element.firstChild?.nodeType, 3);

	const selection = dom.window.getSelection()!;
	const text = simple.element.firstChild!;
	selection.setBaseAndExtent(text, 1, text, 4);
	assert.deepEqual(simple.readSelection(), {
		anchorOffset: 1,
		activeOffset: 4,
	});

	const semanticTokenSource: SemanticTokenSource = {
		textModel: model,
		onDidChange: () => ({ dispose() {}, [Symbol.dispose]() {} }),
		lines: [{ lineIndex: 0, tokens: [{ startColumn: 0, endColumn: 5, presentation: SemanticTokenPresentation.Keyword }] }],
		getLineTokens: lineIndex => lineIndex === 0
			? [{ startColumn: 0, endColumn: 5, presentation: SemanticTokenPresentation.Keyword }]
			: [],
	};
	const bracketColorizationSource: BracketColorizationSource = {
		textModel: model,
		getLineBrackets: lineIndex => lineIndex === 0 ? [{ startColumn: 0, endColumn: 1, level: 1 }] : [],
	};
	using rich = new RichScreenReaderContent(host, { model, semanticTokenSource, bracketColorizationSource });
	rich.sync(state);
	assert.deepEqual(
		[...rich.element.querySelectorAll<HTMLElement>("[data-line-index]")].map(line => line.textContent),
		["alpha", "beta"],
	);
	assert.equal(rich.element.textContent, "alpha\nbeta");
	assert.equal(rich.element.querySelector(".stanza-editor-token")?.textContent, "a");
	assert.deepEqual([...rich.element.querySelectorAll(".token-keyword")].map(token => token.textContent), ["a", "lpha"]);
	assert.equal(rich.readSelection()?.activeOffset, 8);

	using emptyModel = new TextModel("");
	const emptyState = createScreenReaderContentState(
		emptyModel,
		TextSelection.collapsedAt(TextPosition.at(0, 0)),
	);
	simple.sync(emptyState);
	assert.equal(simple.element.firstChild?.nodeType, 3);
	assert.deepEqual(simple.readSelection(), { anchorOffset: 0, activeOffset: 0 });
	dom.window.close();
});

test("native screen-reader pages keep endpoint mappings across omitted middle pages", () => {
	const dom = new JSDOM("<!doctype html><body><main></main></body>");
	const host = dom.window.document.querySelector("main")!;
	using model = new TextModel("zero\none\ntwo\nthree\nfour\nfive\nsix\nseven");
	const selection = TextSelection.from(TextPosition.at(0, 0), TextPosition.at(7, 5));
	const state = createScreenReaderContentState(model, selection, { pageSize: 2 });

	assert.equal(state.segments.length, 2);
	assert.equal(state.text, `zero\none\n${String.fromCharCode(8230)}six\nseven`);
	assert.equal(state.selectionStart, 0);
	assert.equal(state.selectionEnd, state.text.length);
	assert.equal(modelOffsetAtContentOffset(state, state.selectionEnd, "end"), model.length);

	using simple = new SimpleScreenReaderContent(host);
	simple.sync(state);
	assert.deepEqual(simple.readSelection(), { anchorOffset: 0, activeOffset: model.length });
	using rich = new RichScreenReaderContent(host, { model });
	rich.sync(state);
	assert.deepEqual(
		[...rich.element.querySelectorAll<HTMLElement>("[data-line-index]")].map(line => line.dataset.lineIndex),
		["0", "1", "6", "7"],
	);
	assert.equal(rich.element.textContent, state.text);
	dom.window.close();
});

test("native screen-reader support follows logical EditContext focus through the IME bridge", async () => {
	const dom = new JSDOM("<!doctype html><body><main></main></body>");
	const host = dom.window.document.querySelector("main")!;
	using model = new TextModel("alpha\nbeta");
	using selections = new (await import("../../../../common/cursor/editorSelectionController.js")).EditorSelectionController(
		model,
		TextSelectionSet.single(TextSelection.collapsedAt(TextPosition.at(0, 2))),
	);
	using viewport = new EditorViewport({
		container: host,
		model,
		lineHeight: 20,
		textMeasurer: new FixedTextMeasurer(),
		selectionController: selections,
	});
	const focus = new Emitter<void>();
	const blur = new Emitter<void>();
	let optimized = true;
	const accessibilityService = {
		onDidChangeScreenReaderOptimized: () => ({ dispose() {}, [Symbol.dispose]() {} }),
		isScreenReaderOptimized: () => optimized,
	} as unknown as IAccessibilityService;
	using support = new ScreenReaderSupport({
		element: host,
		model,
		viewport,
		selectionController: selections,
		onDidFocus: focus.event,
		onDidBlur: blur.event,
		accessibilityService,
	});

	focus.fire();
	await Promise.resolve();
	const mirror = host.querySelector<HTMLElement>(".stanza-native-screen-reader-content")!;
	assert.equal(mirror.getAttribute("aria-hidden"), "false");
	assert.equal(mirror.textContent, "alpha\nbeta");

	optimized = false;
	support.writeScreenReaderContent();
	assert.equal(mirror.getAttribute("aria-hidden"), "true");
	blur.fire();
	dom.window.close();
});

test("native screen-reader mirror follows viewport coordinates and scrolls to the active page line", async () => {
	const dom = new JSDOM("<!doctype html><body><main></main></body>");
	const host = dom.window.document.querySelector("main")!;
	using model = new TextModel("zero\none\ntwo\nthree");
	using selections = new (await import("../../../../common/cursor/editorSelectionController.js")).EditorSelectionController(
		model,
		TextSelectionSet.single(TextSelection.collapsedAt(TextPosition.at(2, 1))),
	);
	using viewport = new EditorViewport({
		container: host,
		model,
		lineHeight: 20,
		textMeasurer: new FixedTextMeasurer(),
		selectionController: selections,
	});
	viewport.layout({ width: 200, height: 20 });
	const focus = new Emitter<void>();
	const blur = new Emitter<void>();
	const accessibilityService = {
		onDidChangeScreenReaderOptimized: () => ({ dispose() {}, [Symbol.dispose]() {} }),
		isScreenReaderOptimized: () => true,
	} as unknown as IAccessibilityService;
	using support = new ScreenReaderSupport({
		element: host,
		model,
		viewport,
		selectionController: selections,
		onDidFocus: focus.event,
		onDidBlur: blur.event,
		accessibilityService,
	});

	focus.fire();
	await Promise.resolve();
	const mirror = host.querySelector<HTMLElement>(".stanza-native-screen-reader-content")!;
	assert.equal(mirror.style.width, "200px");
	assert.equal(mirror.style.height, "20px");
	assert.equal(mirror.scrollTop, 40);

	dom.window.close();
});
