import assert from "node:assert/strict";
import test from "node:test";
import { JSDOM } from "jsdom";
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
})) {
	Object.defineProperty(globalThis, name, {
		configurable: true,
		value,
	});
}

const { View } = await import("../../../browser/view.js");
const { MouseHandler } = await import("../../../browser/controller/mouseHandler.js");

test("Pointer click counts select and drag by word or complete line", () => {
	const dom = new JSDOM("<!doctype html><body><main></main></body>");
	const container = dom.window.document.querySelector("main");
	assert.ok(container);
	using model = new TextModel("alpha beta\ngamma delta\nlast");
	using selections = new CursorsController(
		model,
		SelectionSet.single(Selection.fromPositions(new Position((0) + 1, (0) + 1))),
	);
	using viewport = new View({
		container,
		model,
		lineHeight: 20,
		textMeasurer: new FixedTextMeasurer(),
		selectionController: selections,
	});
	viewport.layout({ width: 240, height: 60 });
	viewport.element.getBoundingClientRect = () => editorBounds();
	using pointer = new MouseHandler(viewport, selections);

	drag(dom.window, viewport.element, 1, point(20, 5), point(20, 5), 2);
	assert.deepEqual(selections.selections.primary, Selection.fromPositions(
		new Position((0) + 1, (0) + 1),
		new Position((0) + 1, (5) + 1),
	));

	drag(dom.window, viewport.element, 2, point(20, 5), point(80, 5), 2);
	assert.deepEqual(selections.selections.primary, Selection.fromPositions(
		new Position((0) + 1, (0) + 1),
		new Position((0) + 1, (10) + 1),
	));

	drag(dom.window, viewport.element, 3, point(80, 5), point(10, 5), 2);
	assert.deepEqual(selections.selections.primary, Selection.fromPositions(
		new Position((0) + 1, (10) + 1),
		new Position((0) + 1, (0) + 1),
	));

	drag(dom.window, viewport.element, 4, point(20, 25), point(20, 25), 3);
	assert.deepEqual(selections.selections.primary, Selection.fromPositions(
		new Position((1) + 1, (0) + 1),
		new Position((2) + 1, (0) + 1),
	));

	drag(dom.window, viewport.element, 5, point(20, 25), point(20, 5), 3);
	assert.deepEqual(selections.selections.primary, Selection.fromPositions(
		new Position((2) + 1, (0) + 1),
		new Position((0) + 1, (0) + 1),
	));

	selections.setSelections(SelectionSet.single(Selection.fromPositions(
		new Position((0) + 1, (2) + 1),
	)));
	drag(
		dom.window,
		viewport.element,
		6,
		point(20, 25),
		point(20, 25),
		2,
		true,
	);
	assert.deepEqual(selections.selections.primary, Selection.fromPositions(
		new Position((0) + 1, (2) + 1),
		new Position((1) + 1, (5) + 1),
	));

	viewport.element.dispatchEvent(pointerEvent(
		dom.window,
		"pointerdown",
		point(80, 5),
		{ pointerId: 7, detail: 2 },
	));
	model.applyEdits([{
		range: Range.fromPositions(new Position((0) + 1, (0) + 1)),
		text: "X",
	}]);
	dom.window.dispatchEvent(pointerEvent(
		dom.window,
		"pointermove",
		point(80, 25),
		{ pointerId: 7 },
	));
	assert.deepEqual(selections.selections.primary, Selection.fromPositions(
		new Position((0) + 1, (7) + 1),
		new Position((1) + 1, (11) + 1),
	));
	dom.window.dispatchEvent(pointerEvent(
		dom.window,
		"pointerup",
		point(80, 25),
		{ pointerId: 7 },
	));

	dom.window.close();
});

interface PointerPoint {
	readonly clientX: number;
	readonly clientY: number;
}

function point(textOffset: number, top: number): PointerPoint {
	return {
		clientX: 138 + textOffset,
		clientY: 50 + top,
	};
}

function drag(
	targetWindow: typeof browserEnvironment.window,
	element: HTMLElement,
	pointerId: number,
	start: PointerPoint,
	end: PointerPoint,
	detail: number,
	shiftKey = false,
): void {
	element.dispatchEvent(pointerEvent(
		targetWindow,
		"pointerdown",
		start,
		{ pointerId, detail, shiftKey },
	));
	targetWindow.dispatchEvent(pointerEvent(
		targetWindow,
		"pointermove",
		end,
		{ pointerId },
	));
	targetWindow.dispatchEvent(pointerEvent(
		targetWindow,
		"pointerup",
		end,
		{ pointerId },
	));
}

interface PointerEventOptions {
	readonly pointerId: number;
	readonly detail?: number;
	readonly shiftKey?: boolean;
}

function pointerEvent(
	targetWindow: typeof browserEnvironment.window,
	type: string,
	point: PointerPoint,
	options: PointerEventOptions,
): PointerEvent {
	const event = new targetWindow.MouseEvent(type, {
		bubbles: true,
		cancelable: true,
		button: 0,
		buttons: type === "pointerup" || type === "pointercancel" ? 0 : 1,
		clientX: point.clientX,
		clientY: point.clientY,
		detail: options.detail,
		shiftKey: options.shiftKey,
	});
	Object.defineProperty(event, "pointerId", {
		configurable: true,
		value: options.pointerId,
	});
	return event as unknown as PointerEvent;
}

function editorBounds(): DOMRect {
	return {
		x: 100,
		y: 50,
		left: 100,
		top: 50,
		right: 340,
		bottom: 110,
		width: 240,
		height: 60,
		toJSON: () => ({}),
	};
}
