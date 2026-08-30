import assert from "node:assert/strict";
import test from "node:test";
import { JSDOM } from "jsdom";
import { IME } from "../../../../base/common/ime.js";
import { type TextMeasurer } from "../../../browser/config/fontMeasurements.js";
import { CursorsController } from "../../../common/cursor/cursor.js";
import { Selection } from "../../../common/core/selection.js";
import { SelectionSet } from "../../../common/cursor/selectionSet.js";
import { Position } from "../../../common/core/position.js";
import { TextModel } from "../../../common/model/textModel.js";
import { type NativeEditContextConstructor, type NativeEditContextObject, type NativeTextFormatUpdateEvent } from "../../../browser/controller/editContext/native/nativeEditContext.js";

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
class TestResizeObserver { observe(): void {} unobserve(): void {} disconnect(): void {} }
for (const [name, value] of Object.entries({
	window: browserEnvironment.window,
	document: browserEnvironment.window.document,
	Node: browserEnvironment.window.Node,
	Element: browserEnvironment.window.Element,
	HTMLElement: browserEnvironment.window.HTMLElement,
	Event: browserEnvironment.window.Event,
	InputEvent: browserEnvironment.window.InputEvent,
	KeyboardEvent: browserEnvironment.window.KeyboardEvent,
	CompositionEvent: browserEnvironment.window.CompositionEvent,
	ResizeObserver: TestResizeObserver,
})) {
	Object.defineProperty(globalThis, name, {
		configurable: true,
		value,
	});
}

const { View } = await import("../../../browser/view.js");
const { EditorView } = await import('../../../browser/editorView.js');
const { EditorTextAreaInputContext } = await import("../../../browser/controller/editContext/textArea/textAreaEditContext.js");
const { BrowserEditContext } = await import("../../../browser/controller/editContext/native/nativeEditContext.js");

test("EditorView falls back to the textarea EditContext", () => {
	const dom = new JSDOM("<!doctype html><body><main></main></body>");
	const container = dom.window.document.querySelector("main")!;
	using model = new TextModel("text");
	using selections = new CursorsController(model, SelectionSet.single(caret(0, 2)));
	using viewport = new View({ container, model, lineHeight: 20, textMeasurer: new FixedTextMeasurer(), selectionController: selections });
	using input = new EditorView(viewport, selections);

	assert.equal(input.editContext instanceof EditorTextAreaInputContext, true);
	assert.equal(input.textArea?.tagName, "TEXTAREA");
	dom.window.close();
});

test("Native EditContext normalizes text updates and feeds common commands", () => {
	const dom = new JSDOM("<!doctype html><body><main></main></body>");
	const container = dom.window.document.querySelector("main")!;
	Object.defineProperty(dom.window, "EditContext", {
		configurable: true,
		value: FakeNativeEditContext,
	});
	using model = new TextModel("abcd");
	using selections = new CursorsController(model, SelectionSet.single(caret(0, 2)));
	using viewport = new View({ container, model, lineHeight: 20, textMeasurer: new FixedTextMeasurer(), selectionController: selections });
	using input = new EditorView(viewport, selections);

	assert.equal(input.editContext instanceof BrowserEditContext, true);
	assert.equal(input.element.tagName, "DIV");
	assert.equal(input.textArea, undefined);
	const nativeContext = (input.editContext as InstanceType<typeof BrowserEditContext>).nativeContext as FakeNativeEditContext;

	nativeContext.dispatch(textUpdate(browserEnvironment.window, {
		text: "X",
		updateRangeStart: 2,
		updateRangeEnd: 2,
		selectionStart: 3,
		selectionEnd: 3,
	}));
	assert.equal(model.getText(), "abXcd");
	assert.deepEqual(selections.selections.primary, caret(0, 3));

	nativeContext.dispatch(textUpdate(browserEnvironment.window, {
		text: "",
		updateRangeStart: 2,
		updateRangeEnd: 3,
		selectionStart: 2,
		selectionEnd: 2,
	}));
	assert.equal(model.getText(), "abcd");
	assert.deepEqual(selections.selections.primary, caret(0, 2));

	const lineBreak = new dom.window.InputEvent("beforeinput", {
		bubbles: true,
		cancelable: true,
		inputType: "insertLineBreak",
	});
	input.element.dispatchEvent(lineBreak);
	nativeContext.dispatch(textUpdate(browserEnvironment.window, {
		text: "\n",
		updateRangeStart: 2,
		updateRangeEnd: 2,
		selectionStart: 3,
		selectionEnd: 3,
	}));
	assert.equal(lineBreak.defaultPrevented, true);
	assert.equal(model.getText(), "ab\ncd");

	selections.setSelections(SelectionSet.single(caret(0, 1)));
	assert.equal(nativeContext.selectionStart, 1);
	assert.equal(nativeContext.selectionEnd, 1);
	dom.window.close();
});

test("Native EditContext composition updates use the protected common session", () => {
	const dom = new JSDOM("<!doctype html><body><main></main></body>");
	const container = dom.window.document.querySelector("main")!;
	Object.defineProperty(dom.window, "EditContext", {
		configurable: true,
		value: FakeNativeEditContext,
	});
	using model = new TextModel("abcd");
	using selections = new CursorsController(model, SelectionSet.single(caret(0, 2)));
	using viewport = new View({ container, model, lineHeight: 20, textMeasurer: new FixedTextMeasurer(), selectionController: selections });
	using input = new EditorView(viewport, selections);
	const nativeContext = (input.editContext as InstanceType<typeof BrowserEditContext>).nativeContext as FakeNativeEditContext;

	nativeContext.dispatch(new browserEnvironment.window.CompositionEvent("compositionstart", { bubbles: true, cancelable: true }));
	nativeContext.dispatch(textUpdate(browserEnvironment.window, {
		text: "xy",
		updateRangeStart: 2,
		updateRangeEnd: 2,
		selectionStart: 4,
		selectionEnd: 4,
	}));
	assert.equal(input.compositionController.composing, true);
	assert.equal(model.getText(), "abxycd");
	assert.deepEqual(selections.selections.primary, caret(0, 4));

	nativeContext.dispatch(new browserEnvironment.window.CompositionEvent("compositionend", { bubbles: true, cancelable: true }));
	assert.equal(input.compositionController.composing, false);
	assert.equal(model.getText(), "abxycd");
	assert.equal(model.canUndo(), true);
	dom.window.close();
});

test("Native EditContext maps a bounded native window back to model offsets", () => {
	const dom = new JSDOM("<!doctype html><body><main></main></body>");
	const container = dom.window.document.querySelector("main")!;
	Object.defineProperty(dom.window, "EditContext", {
		configurable: true,
		value: FakeNativeEditContext,
	});
	const source = "a".repeat(40_000);
	using model = new TextModel(source);
	using selections = new CursorsController(model, SelectionSet.single(caret(0, 20_000)));
	using viewport = new View({ container, model, lineHeight: 20, textMeasurer: new FixedTextMeasurer(), selectionController: selections });
	using input = new EditorView(viewport, selections);
	const native = input.editContext as InstanceType<typeof BrowserEditContext>;
	const nativeContext = native.nativeContext as FakeNativeEditContext;

	assert.ok(native.textWindow.startOffset > 0);
	assert.ok(native.textWindow.endOffset < source.length);
	assert.equal(nativeContext.selectionStart, 20_000 - native.textWindow.startOffset);
	nativeContext.dispatch(textUpdate(browserEnvironment.window, {
		text: "X",
		updateRangeStart: nativeContext.selectionStart,
		updateRangeEnd: nativeContext.selectionStart,
		selectionStart: nativeContext.selectionStart + 1,
		selectionEnd: nativeContext.selectionStart + 1,
	}));
	assert.equal(model.getText().slice(19_999, 20_002), "aXa");
	assert.deepEqual(selections.selections.primary, caret(0, 20_001));
	assert.equal(nativeContext.text, model.getText().slice(native.textWindow.startOffset, native.textWindow.endOffset));
	dom.window.close();
});

test("Native EditContext combines split UTF-16 surrogate updates", () => {
	const dom = new JSDOM("<!doctype html><body><main></main></body>");
	const container = dom.window.document.querySelector("main")!;
	Object.defineProperty(dom.window, "EditContext", {
		configurable: true,
		value: FakeNativeEditContext,
	});
	using model = new TextModel("");
	using selections = new CursorsController(model, SelectionSet.single(caret(0, 0)));
	using viewport = new View({ container, model, lineHeight: 20, textMeasurer: new FixedTextMeasurer(), selectionController: selections });
	using input = new EditorView(viewport, selections);
	const nativeContext = (input.editContext as InstanceType<typeof BrowserEditContext>).nativeContext as FakeNativeEditContext;

	nativeContext.dispatch(textUpdate(browserEnvironment.window, {
		text: "\uD83D",
		updateRangeStart: 0,
		updateRangeEnd: 0,
		selectionStart: 1,
		selectionEnd: 1,
	}));
	assert.equal(model.getText(), "");
	nativeContext.dispatch(textUpdate(browserEnvironment.window, {
		text: "\uDE00",
		updateRangeStart: 1,
		updateRangeEnd: 1,
		selectionStart: 2,
		selectionEnd: 2,
	}));
	assert.equal(model.getText(), "😀");
	assert.deepEqual(selections.selections.primary, caret(0, 2));
	dom.window.close();
});

test("Native EditContext restores its browser buffer in read-only mode", () => {
	const dom = new JSDOM("<!doctype html><body><main></main></body>");
	const container = dom.window.document.querySelector("main")!;
	Object.defineProperty(dom.window, "EditContext", {
		configurable: true,
		value: FakeNativeEditContext,
	});
	using model = new TextModel("abcd");
	using selections = new CursorsController(model, SelectionSet.single(caret(0, 2)), { readOnly: true });
	using viewport = new View({ container, model, lineHeight: 20, textMeasurer: new FixedTextMeasurer(), selectionController: selections });
	using input = new EditorView(viewport, selections);
	const nativeContext = (input.editContext as InstanceType<typeof BrowserEditContext>).nativeContext as FakeNativeEditContext;

	const beforeInput = new dom.window.InputEvent("beforeinput", {
		bubbles: true,
		cancelable: true,
		inputType: "insertText",
		data: "X",
	});
	input.element.dispatchEvent(beforeInput);
	assert.equal(beforeInput.defaultPrevented, true);
	nativeContext.text = "abXcd";
	nativeContext.selectionStart = 3;
	nativeContext.selectionEnd = 3;
	nativeContext.dispatch(textUpdate(browserEnvironment.window, {
		text: "X",
		updateRangeStart: 2,
		updateRangeEnd: 2,
		selectionStart: 3,
		selectionEnd: 3,
	}));
	assert.equal(model.getText(), "abcd");
	assert.equal(nativeContext.text, "abcd");
	assert.equal(nativeContext.selectionStart, 2);
	dom.window.close();
});

test("Native EditContext normalizes text formats to model offsets", () => {
	const dom = new JSDOM("<!doctype html><body><main></main></body>");
	const container = dom.window.document.querySelector("main")!;
	Object.defineProperty(dom.window, "EditContext", {
		configurable: true,
		value: FakeNativeEditContext,
	});
	using model = new TextModel("a".repeat(40_000));
	using selections = new CursorsController(model, SelectionSet.single(caret(0, 20_000)));
	using viewport = new View({ container, model, lineHeight: 20, textMeasurer: new FixedTextMeasurer(), selectionController: selections });
	using input = new EditorView(viewport, selections);
	const native = input.editContext as InstanceType<typeof BrowserEditContext>;
	const nativeContext = native.nativeContext as FakeNativeEditContext;
	let update: { readonly rangeStart: number; readonly rangeEnd: number } | undefined;
	using listener = native.onDidTextFormatUpdate(event => {
		update = event.formats[0];
	});
	const formatEvent = new browserEnvironment.window.Event("textformatupdate") as NativeTextFormatUpdateEvent;
	formatEvent.getTextFormats = () => [{
		rangeStart: nativeContext.selectionStart,
		rangeEnd: nativeContext.selectionStart + 2,
		underlineThickness: "thick",
	}];
	nativeContext.dispatch(formatEvent);
	assert.deepEqual(update, {
		rangeStart: 20_000,
		rangeEnd: 20_002,
		underlineThickness: "thick",
	});
	dom.window.close();
});

test("Native EditContext answers character bounds in native-text coordinates", () => {
	const dom = new JSDOM("<!doctype html><body><main></main></body>");
	const container = dom.window.document.querySelector("main")!;
	Object.defineProperty(dom.window, "EditContext", {
		configurable: true,
		value: FakeNativeEditContext,
	});
	using model = new TextModel("abcd");
	using selections = new CursorsController(model, SelectionSet.single(caret(0, 2)));
	using viewport = new View({ container, model, lineHeight: 20, textMeasurer: new FixedTextMeasurer(), selectionController: selections });
	using input = new EditorView(viewport, selections);
	const nativeContext = (input.editContext as InstanceType<typeof BrowserEditContext>).nativeContext as FakeNativeEditContext;
	const boundsEvent = new browserEnvironment.window.Event("characterboundsupdate") as FakeCharacterBoundsUpdateEvent;
	Object.assign(boundsEvent, { rangeStart: 1, rangeEnd: 3 });
	nativeContext.dispatch(boundsEvent);
	assert.equal(nativeContext.characterBoundsCalls.length, 1);
	assert.equal(nativeContext.characterBoundsCalls[0]!.start, 1);
	assert.equal(nativeContext.characterBoundsCalls[0]!.bounds.length, 2);
	dom.window.close();
});

test("Native EditContext keeps focus through the IME-disabled textarea bridge", () => {
	const dom = new JSDOM("<!doctype html><body><main></main></body>");
	const container = dom.window.document.querySelector("main")!;
	Object.defineProperty(dom.window, "EditContext", {
		configurable: true,
		value: FakeNativeEditContext,
	});
	using model = new TextModel("text");
	using selections = new CursorsController(model, SelectionSet.single(caret(0, 2)));
	using viewport = new View({ container, model, lineHeight: 20, textMeasurer: new FixedTextMeasurer(), selectionController: selections });
	using input = new EditorView(viewport, selections);
	try {
		input.focus();
		assert.equal(dom.window.document.activeElement, input.element);
		IME.disable();
		assert.equal(dom.window.document.activeElement?.tagName, "TEXTAREA");
		IME.enable();
		assert.equal(dom.window.document.activeElement, input.element);
	} finally {
		IME.enable();
		dom.window.close();
	}
});

function textUpdate(
	targetWindow: typeof browserEnvironment.window,
	update: Pick<FakeNativeTextUpdateEvent, "text" | "updateRangeStart" | "updateRangeEnd" | "selectionStart" | "selectionEnd">,
): FakeNativeTextUpdateEvent {
	const event = new targetWindow.Event("textupdate", { bubbles: true }) as FakeNativeTextUpdateEvent;
	Object.assign(event, update);
	return event;
}

function caret(lineIndex: number, columnIndex: number): Selection {
	return Selection.fromPositions(new Position((lineIndex) + 1, (columnIndex) + 1));
}

interface FakeNativeTextUpdateEvent extends Event {
	text: string;
	updateRangeStart: number;
	updateRangeEnd: number;
	selectionStart: number;
	selectionEnd: number;
}

interface FakeCharacterBoundsUpdateEvent extends Event {
	rangeStart: number;
	rangeEnd: number;
}

class FakeNativeEditContext implements NativeEditContextObject {
	private readonly target = new browserEnvironment.window.EventTarget();
	text = "";
	selectionStart = 0;
	selectionEnd = 0;
	readonly characterBoundsCalls: Array<{ readonly start: number; readonly bounds: readonly DOMRect[] }> = [];

	addEventListener(type: string, listener: EventListenerOrEventListenerObject | null, options?: boolean | AddEventListenerOptions): void {
		this.target.addEventListener(type, listener, options);
	}

	removeEventListener(type: string, listener: EventListenerOrEventListenerObject | null, options?: boolean | EventListenerOptions): void {
		this.target.removeEventListener(type, listener, options);
	}

	dispatchEvent(event: Event): boolean {
		return this.target.dispatchEvent(event);
	}

	updateText(start: number, end: number, text: string): void {
		this.text = this.text.slice(0, start) + text + this.text.slice(end);
	}

	updateSelection(start: number, end: number): void {
		this.selectionStart = start;
		this.selectionEnd = end;
	}

	updateCharacterBounds(start: number, bounds: readonly DOMRect[]): void {
		this.characterBoundsCalls.push({ start, bounds: [...bounds] });
	}

	dispatch(event: Event): void {
		this.dispatchEvent(event);
	}
}

void (FakeNativeEditContext satisfies NativeEditContextConstructor);
