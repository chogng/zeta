import assert from "node:assert/strict";
import test from "node:test";
import { JSDOM } from "jsdom";
import { IME } from "../../../../base/common/ime.js";
import { type TextMeasurer } from "../../../browser/config/fontMeasurements.js";
import { CursorsController } from "../../../common/cursor/cursor.js";
import { Selection } from "../../../common/core/selection.js";
import { SelectionSet } from "../../../common/cursor/selectionSet.js";
import { Position } from "../../../common/core/position.js";
import { Range } from "../../../common/core/range.js";
import { TextModel } from "../../../common/model/textModel.js";

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
	CompositionEvent: browserEnvironment.window.CompositionEvent,
})) {
	Object.defineProperty(globalThis, name, {
		configurable: true,
		value,
	});
}

const { EditorViewport } = await import("../../../browser/view.js");
const { EditorView } = await import("../../../browser/view.js");

test("Textarea composition commits one revision and positions the IME input", () => {
	const dom = new JSDOM("<!doctype html><body><main></main></body>");
	const container = dom.window.document.querySelector("main");
	assert.ok(container);
	using model = new TextModel("hello");
	const initial = SelectionSet.single(selection(0, 1, 0, 4));
	using selections = new CursorsController(model, initial);
	using viewport = new EditorViewport({
		container,
		model,
		glyphMargin: false,
		lineHeight: 20,
		textMeasurer: new FixedTextMeasurer(),
		selectionController: selections,
	});
	viewport.layout({ width: 100, height: 20 });
	using input = new EditorView(viewport, selections);
	const states: boolean[] = [];
	using listener = input.compositionController.onDidChange(state => states.push(state));

	input.textArea!.dispatchEvent(compositionEvent(dom.window, "compositionstart", ""));
	assert.deepEqual({
		composing: input.compositionController.composing,
		rootClass: viewport.element.classList.contains("composing"),
		inputClass: input.textArea!.classList.contains("ime-input"),
		left: input.textArea!.style.left,
		top: input.textArea!.style.top,
		height: input.textArea!.style.height,
	}, {
		composing: true,
		rootClass: true,
		inputClass: true,
		left: "48px",
		top: "0px",
		height: "20px",
	});

	input.textArea!.value = "ni";
	input.textArea!.setSelectionRange(1, 2, "backward");
	input.textArea!.dispatchEvent(compositionEvent(dom.window, "compositionupdate", "ni"));
	const firstUnderline = viewport.element.querySelector<HTMLElement>(
		".stanza-editor-composition",
	);
	assert.ok(firstUnderline);
	assert.deepEqual({
		text: model.getText(),
		selection: selections.selections.primary,
		underline: {
			left: firstUnderline.style.left,
			width: firstUnderline.style.width,
		},
	}, {
		text: "hnio",
		selection: Selection.fromPositions(new Position((0) + 1, (3) + 1), new Position((0) + 1, (2) + 1)),
		underline: {
			left: "48px",
			width: "20px",
		},
	});

	input.textArea!.value = "你";
	input.textArea!.setSelectionRange(1, 1);
	input.textArea!.dispatchEvent(compositionEvent(dom.window, "compositionupdate", "你"));
	input.textArea!.dispatchEvent(compositionEvent(dom.window, "compositionend", "你"));

	assert.deepEqual({
		text: model.getText(),
		selection: selections.selections.primary,
		composing: input.compositionController.composing,
		rootClass: viewport.element.classList.contains("composing"),
		inputValue: input.textArea!.value,
		underlineCount: viewport.element.querySelectorAll(
			".stanza-editor-composition",
		).length,
		states,
	}, {
		text: "h你o",
		selection: caret(0, 2),
		composing: false,
		rootClass: false,
		inputValue: "",
		underlineCount: 0,
		states: [true, false],
	});

	selections.undo();
	assert.deepEqual({
		text: model.getText(),
		selections: selections.selections,
	}, {
		text: "hello",
		selections: initial,
	});

	dom.window.close();
});

test("Escape, blur, and disposal cancel active textarea composition", () => {
	const dom = new JSDOM("<!doctype html><body><main></main></body>");
	const container = dom.window.document.querySelector("main");
	assert.ok(container);
	using model = new TextModel("abc");
	const initial = SelectionSet.single(caret(0, 1));
	using selections = new CursorsController(model, initial);
	using viewport = new EditorViewport({
		container,
		model,
		glyphMargin: false,
		lineHeight: 20,
		textMeasurer: new FixedTextMeasurer(),
		selectionController: selections,
	});
	viewport.layout({ width: 100, height: 20 });
	const input = new EditorView(viewport, selections);

	startAndUpdate(dom.window, input.textArea!, "中");
	const positionedInput = {
		left: input.textArea!.style.left,
		top: input.textArea!.style.top,
		height: input.textArea!.style.height,
	};
	input.textArea!.dispatchEvent(keyboardEvent(dom.window, "Escape", true));
	input.textArea!.dispatchEvent(compositionEvent(dom.window, "compositionend", "中"));
	assert.deepEqual({
		text: model.getText(),
		selections: selections.selections,
		canUndo: model.canUndo,
	}, {
		text: "abc",
		selections: initial,
		canUndo: false,
	});
	assert.deepEqual({
		left: input.textArea!.style.left,
		top: input.textArea!.style.top,
		height: input.textArea!.style.height,
	}, { left: "", top: "", height: "" });

	input.textArea!.focus();
	startAndUpdate(dom.window, input.textArea!, "X");
	assert.deepEqual({
		left: input.textArea!.style.left,
		top: input.textArea!.style.top,
		height: input.textArea!.style.height,
	}, positionedInput);
	input.textArea!.blur();
	assert.equal(model.getText(), "abc");
	assert.equal(input.compositionController.composing, false);

	input.textArea!.focus();
	startAndUpdate(dom.window, input.textArea!, "Y");
	input.dispose();
	assert.equal(model.getText(), "abc");
	assert.equal(viewport.element.classList.contains("composing"), false);

	dom.window.close();
});

test("Empty composition end commits deletion while a stray end is ignored", () => {
	const dom = new JSDOM("<!doctype html><body><main></main></body>");
	const container = dom.window.document.querySelector("main");
	assert.ok(container);
	using model = new TextModel("abc");
	const initial = SelectionSet.single(selection(0, 1, 0, 2));
	using selections = new CursorsController(model, initial);
	using viewport = new EditorViewport({
		container,
		model,
		glyphMargin: false,
		lineHeight: 20,
		textMeasurer: new FixedTextMeasurer(),
		selectionController: selections,
	});
	using input = new EditorView(viewport, selections);

	input.textArea!.dispatchEvent(compositionEvent(dom.window, "compositionstart", ""));
	input.textArea!.dispatchEvent(compositionEvent(dom.window, "compositionend", ""));
	input.textArea!.dispatchEvent(compositionEvent(dom.window, "compositionend", ""));
	assert.deepEqual({
		text: model.getText(),
		selection: selections.selections.primary,
	}, {
		text: "ac",
		selection: caret(0, 1),
	});

	selections.undo();
	assert.deepEqual({
		text: model.getText(),
		selections: selections.selections,
	}, {
		text: "abc",
		selections: initial,
	});

	dom.window.close();
});

test("IME coordination, multi-cursor rejection, and external invalidation are safe", () => {
	const dom = new JSDOM("<!doctype html><body><main></main></body>");
	const container = dom.window.document.querySelector("main");
	assert.ok(container);
	using model = new TextModel("a\nbc");
	using selections = new CursorsController(
		model,
		SelectionSet.withPrimary([caret(1, 1), caret(0, 0)], 0),
	);
	using viewport = new EditorViewport({
		container,
		model,
		glyphMargin: false,
		lineHeight: 20,
		textMeasurer: new FixedTextMeasurer(),
		selectionController: selections,
	});
	viewport.layout({ width: 100, height: 40 });
	using input = new EditorView(viewport, selections);

	const multiStart = compositionEvent(dom.window, "compositionstart", "");
	input.textArea!.dispatchEvent(multiStart);
	assert.equal(multiStart.defaultPrevented, true);
	assert.equal(input.compositionController.composing, false);

	selections.setSelections(SelectionSet.single(caret(1, 1)));
	try {
		IME.disable();
		assert.equal(input.textArea!.readOnly, true);
		const disabledStart = compositionEvent(dom.window, "compositionstart", "");
		input.textArea!.dispatchEvent(disabledStart);
		assert.equal(disabledStart.defaultPrevented, true);
		IME.enable();
		assert.equal(input.textArea!.readOnly, false);

		input.textArea!.dispatchEvent(compositionEvent(dom.window, "compositionstart", ""));
		input.textArea!.value = "x\r\ny";
		input.textArea!.setSelectionRange(4, 4);
		input.textArea!.dispatchEvent(compositionEvent(
			dom.window,
			"compositionupdate",
			"x\r\ny",
		));
		assert.deepEqual({
			text: model.getText(),
			selection: selections.selections.primary,
			top: input.textArea!.style.top,
			underlines: [...viewport.element.querySelectorAll<HTMLElement>(
				".stanza-editor-composition",
			)].map(element => ({
				left: element.style.left,
				width: element.style.width,
			})),
		}, {
			text: "a\nbx\nyc",
			selection: caret(2, 1),
			top: "40px",
			underlines: [
				{ left: "48px", width: "20px" },
				{ left: "38px", width: "10px" },
			],
		});

		model.applyEdits([{
			range: Range.fromPositions(model.positionAt(model.getText().length)),
			text: "!",
		}]);
		assert.equal(input.compositionController.composing, false);
		assert.equal(viewport.element.classList.contains("composing"), false);
		assert.equal(viewport.element.querySelectorAll(
			".stanza-editor-composition",
		).length, 0);
		input.textArea!.dispatchEvent(compositionEvent(dom.window, "compositionupdate", "ignored"));
		input.textArea!.dispatchEvent(compositionEvent(dom.window, "compositionend", "ignored"));
		assert.equal(model.getText(), "a\nbx\nyc!");
	} finally {
		IME.enable();
	}

	dom.window.close();
});

function startAndUpdate(targetWindow: typeof browserEnvironment.window, element: HTMLTextAreaElement, text: string): void {
	element.dispatchEvent(compositionEvent(targetWindow, "compositionstart", ""));
	element.value = text;
	element.setSelectionRange(text.length, text.length);
	element.dispatchEvent(compositionEvent(targetWindow, "compositionupdate", text));
}

function compositionEvent(targetWindow: typeof browserEnvironment.window, type: "compositionstart" | "compositionupdate" | "compositionend", data: string): CompositionEvent {
	return new targetWindow.CompositionEvent(type, {
		bubbles: true,
		cancelable: true,
		data,
	}) as unknown as CompositionEvent;
}

function keyboardEvent(targetWindow: typeof browserEnvironment.window, key: string, isComposing: boolean): KeyboardEvent {
	return new targetWindow.KeyboardEvent("keydown", {
		bubbles: true,
		cancelable: true,
		key,
		isComposing,
	}) as unknown as KeyboardEvent;
}

function selection(startLine: number, startColumn: number, endLine: number, endColumn: number): Selection {
	return Selection.fromPositions(
		new Position((startLine) + 1, (startColumn) + 1),
		new Position((endLine) + 1, (endColumn) + 1),
	);
}

function caret(lineIndex: number, columnIndex: number): Selection {
	return Selection.fromPositions(new Position((lineIndex) + 1, (columnIndex) + 1));
}
