import assert from "node:assert/strict";
import test from "node:test";
import { JSDOM } from "jsdom";
import { type TextMeasurer } from "../../../browser/config/fontMeasurements.js";
import { PointerMultiCursorModifier } from "../../../common/cursor/cursorMoveCommands.js";
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
})) {
	Object.defineProperty(globalThis, name, {
		configurable: true,
		value,
	});
}

const { View } = await import("../../../browser/view.js");
const { MouseHandler } = await import("../../../browser/controller/mouseHandler.js");

test("Alt pointer gestures add, toggle, drag, and track multiple selections", () => {
	const dom = new JSDOM("<!doctype html><body><main></main></body>");
	const container = dom.window.document.querySelector("main");
	assert.ok(container);
	using model = new TextModel("abcd\nefgh\nijkl");
	using selections = new CursorsController(
		model,
		SelectionSet.single(caret(0, 1)),
	);
	using viewport = new View({
		container,
		model,
		glyphMargin: false,
		lineHeight: 20,
		textMeasurer: new FixedTextMeasurer(),
		selectionController: selections,
	});
	viewport.layout({ width: 200, height: 60 });
	viewport.element.getBoundingClientRect = () => editorBounds();
	using pointer = new MouseHandler(viewport, selections);

	click(dom.window, viewport.element, 1, 158, 75, { altKey: true });
	assert.deepEqual(selections.selections, SelectionSet.withPrimary([
		caret(0, 1),
		caret(1, 2),
	], 1));

	click(dom.window, viewport.element, 2, 148, 55, { altKey: true });
	assert.deepEqual(
		selections.selections,
		SelectionSet.single(caret(1, 2)),
	);

	click(dom.window, viewport.element, 3, 158, 75, { altKey: true });
	assert.deepEqual(
		selections.selections,
		SelectionSet.single(caret(1, 2)),
	);

	selections.setSelections(SelectionSet.single(caret(0, 0)));
	click(dom.window, viewport.element, 4, 158, 75, {
		altKey: true,
		detail: 2,
	});
	assert.deepEqual(selections.selections, SelectionSet.withPrimary([
		caret(0, 0),
		Selection.fromPositions(new Position((1) + 1, (0) + 1), new Position((1) + 1, (4) + 1)),
	], 1));

	selections.setSelections(SelectionSet.withPrimary([
		caret(0, 1),
		caret(1, 2),
	], 1));
	viewport.element.dispatchEvent(pointerEvent(
		dom.window,
		"pointerdown",
		148,
		95,
		{ pointerId: 5, altKey: true },
	));
	model.applyEdits([{
		range: Range.fromPositions(new Position((0) + 1, (0) + 1)),
		text: "X",
	}]);
	dom.window.dispatchEvent(pointerEvent(
		dom.window,
		"pointermove",
		168,
		95,
		{ pointerId: 5, altKey: true },
	));
	assert.deepEqual(selections.selections, SelectionSet.withPrimary([
		caret(0, 2),
		caret(1, 2),
		Selection.fromPositions(new Position((2) + 1, (1) + 1), new Position((2) + 1, (3) + 1)),
	], 2));
	dom.window.dispatchEvent(pointerEvent(
		dom.window,
		"pointerup",
		168,
		95,
		{ pointerId: 5, altKey: true },
	));

	click(dom.window, viewport.element, 6, 168, 55, {
		altKey: true,
		shiftKey: true,
	});
	assert.deepEqual(selections.selections, SelectionSet.single(
		Selection.fromPositions(new Position((2) + 1, (1) + 1), new Position((0) + 1, (3) + 1)),
	));

	dom.window.close();
});

test("Control-or-Meta mode is explicit and leaves Alt as a normal click", () => {
	const dom = new JSDOM("<!doctype html><body><main></main></body>");
	const container = dom.window.document.querySelector("main");
	assert.ok(container);
	using model = new TextModel("abcd\nefgh\nijkl");
	using selections = new CursorsController(
		model,
		SelectionSet.single(caret(0, 0)),
	);
	using viewport = new View({
		container,
		model,
		glyphMargin: false,
		lineHeight: 20,
		textMeasurer: new FixedTextMeasurer(),
		selectionController: selections,
	});
	viewport.layout({ width: 200, height: 60 });
	viewport.element.getBoundingClientRect = () => editorBounds();
	using pointer = new MouseHandler(viewport, selections, {
		multiCursorModifier: PointerMultiCursorModifier.ControlOrMeta,
	});

	click(dom.window, viewport.element, 6, 158, 55, { ctrlKey: true });
	click(dom.window, viewport.element, 7, 148, 75, { metaKey: true });
	assert.deepEqual(selections.selections, SelectionSet.withPrimary([
		caret(0, 0),
		caret(0, 2),
		caret(1, 1),
	], 2));

	click(dom.window, viewport.element, 8, 148, 95, { altKey: true });
	assert.deepEqual(
		selections.selections,
		SelectionSet.single(caret(2, 1)),
	);
	assert.throws(
		() => new MouseHandler(viewport, selections, {
			multiCursorModifier: "shift" as PointerMultiCursorModifier,
		}),
		/Unknown Stanza pointer multi-cursor modifier/,
	);

	dom.window.close();
});

interface PointerEventOptions {
	readonly pointerId: number;
	readonly detail?: number;
	readonly altKey?: boolean;
	readonly ctrlKey?: boolean;
	readonly metaKey?: boolean;
	readonly shiftKey?: boolean;
}

function click(
	targetWindow: typeof browserEnvironment.window,
	element: HTMLElement,
	pointerId: number,
	clientX: number,
	clientY: number,
	options: Omit<PointerEventOptions, "pointerId">,
): void {
	element.dispatchEvent(pointerEvent(
		targetWindow,
		"pointerdown",
		clientX,
		clientY,
		{ pointerId, ...options },
	));
	targetWindow.dispatchEvent(pointerEvent(
		targetWindow,
		"pointerup",
		clientX,
		clientY,
		{ pointerId, ...options },
	));
}

function pointerEvent(
	targetWindow: typeof browserEnvironment.window,
	type: string,
	clientX: number,
	clientY: number,
	options: PointerEventOptions,
): PointerEvent {
	const event = new targetWindow.MouseEvent(type, {
		bubbles: true,
		cancelable: true,
		button: 0,
		buttons: type === "pointerup" || type === "pointercancel" ? 0 : 1,
		clientX,
		clientY,
		detail: options.detail,
		altKey: options.altKey,
		ctrlKey: options.ctrlKey,
		metaKey: options.metaKey,
		shiftKey: options.shiftKey,
	});
	Object.defineProperty(event, "pointerId", {
		configurable: true,
		value: options.pointerId,
	});
	return event as unknown as PointerEvent;
}

function caret(lineIndex: number, columnIndex: number): Selection {
	return Selection.fromPositions(new Position((lineIndex) + 1, (columnIndex) + 1));
}

function editorBounds(): DOMRect {
	return {
		x: 100,
		y: 50,
		left: 100,
		top: 50,
		right: 300,
		bottom: 110,
		width: 200,
		height: 60,
		toJSON: () => ({}),
	};
}
