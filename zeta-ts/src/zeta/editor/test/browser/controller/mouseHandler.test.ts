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

const { EditorViewport } = await import(
	"../../../browser/view.js"
);
const { MouseHandler } = await import(
	"../../../browser/controller/mouseHandler.js"
);

test("Pointer selection supports clicks, Shift, drag, gutter, and cancellation", () => {
	const dom = new JSDOM("<!doctype html><body><main></main></body>");
	const container = dom.window.document.querySelector("main");
	assert.ok(container);
	using model = new TextModel("abcd\nefgh\nijkl\nmnop");
	using selections = new CursorsController(
		model,
		SelectionSet.single(
			Selection.fromPositions(new Position((0) + 1, (0) + 1)),
		),
	);
	using viewport = new EditorViewport({
		container,
		model,
		glyphMargin: false,
		lineHeight: 20,
		textMeasurer: new FixedTextMeasurer(),
		selectionController: selections,
	});
	viewport.layout({ width: 200, height: 80 });
	viewport.element.getBoundingClientRect = () => editorBounds();
	const captured = new Set<number>();
	viewport.element.setPointerCapture = pointerId => captured.add(pointerId);
	viewport.element.hasPointerCapture = pointerId => captured.has(pointerId);
	viewport.element.releasePointerCapture = pointerId => {
		captured.delete(pointerId);
	};
	const pointer = new MouseHandler(viewport, selections);

	const click = pointerEvent(dom.window, "pointerdown", 148, 75, {
		pointerId: 1,
	});
	viewport.element.dispatchEvent(click);
	dom.window.dispatchEvent(pointerEvent(
		dom.window,
		"pointerup",
		148,
		75,
		{ pointerId: 1 },
	));
	assert.equal(click.defaultPrevented, true);
	assert.deepEqual(selections.selections.primary, Selection.fromPositions(
		new Position((1) + 1, (1) + 1),
	));
	assert.deepEqual([...captured], []);

	viewport.element.dispatchEvent(pointerEvent(
		dom.window,
		"pointerdown",
		158,
		105,
		{ pointerId: 2, shiftKey: true },
	));
	dom.window.dispatchEvent(pointerEvent(
		dom.window,
		"pointerup",
		158,
		105,
		{ pointerId: 2, shiftKey: true },
	));
	assert.deepEqual(selections.selections.primary, Selection.fromPositions(
		new Position((1) + 1, (1) + 1),
		new Position((2) + 1, (2) + 1),
	));

	viewport.element.dispatchEvent(pointerEvent(
		dom.window,
		"pointerdown",
		148,
		55,
		{ pointerId: 3 },
	));
	dom.window.dispatchEvent(pointerEvent(
		dom.window,
		"pointermove",
		168,
		105,
		{ pointerId: 99 },
	));
	assert.deepEqual(selections.selections.primary, Selection.fromPositions(
		new Position((0) + 1, (1) + 1),
	));
	dom.window.dispatchEvent(pointerEvent(
		dom.window,
		"pointermove",
		168,
		105,
		{ pointerId: 3 },
	));
	assert.deepEqual(selections.selections.primary, Selection.fromPositions(
		new Position((0) + 1, (1) + 1),
		new Position((2) + 1, (3) + 1),
	));
	dom.window.dispatchEvent(pointerEvent(
		dom.window,
		"pointerup",
		168,
		105,
		{ pointerId: 3 },
	));

	viewport.element.dispatchEvent(pointerEvent(
		dom.window,
		"pointerdown",
		110,
		75,
		{ pointerId: 4 },
	));
	assert.deepEqual(selections.selections.primary, Selection.fromPositions(
		new Position((1) + 1, (0) + 1),
		new Position((2) + 1, (0) + 1),
	));
	dom.window.dispatchEvent(pointerEvent(
		dom.window,
		"pointermove",
		110,
		115,
		{ pointerId: 4 },
	));
	assert.deepEqual(selections.selections.primary, Selection.fromPositions(
		new Position((1) + 1, (0) + 1),
		new Position((3) + 1, (4) + 1),
	));
	dom.window.dispatchEvent(pointerEvent(
		dom.window,
		"pointerup",
		110,
		115,
		{ pointerId: 4 },
	));

	viewport.element.dispatchEvent(pointerEvent(
		dom.window,
		"pointerdown",
		110,
		95,
		{ pointerId: 5 },
	));
	dom.window.dispatchEvent(pointerEvent(
		dom.window,
		"pointermove",
		110,
		55,
		{ pointerId: 5 },
	));
	assert.deepEqual(selections.selections.primary, Selection.fromPositions(
		new Position((3) + 1, (0) + 1),
		new Position((0) + 1, (0) + 1),
	));
	dom.window.dispatchEvent(pointerEvent(
		dom.window,
		"pointercancel",
		110,
		55,
		{ pointerId: 5 },
	));
	const cancelledSelection = selections.selections;
	dom.window.dispatchEvent(pointerEvent(
		dom.window,
		"pointermove",
		168,
		115,
		{ pointerId: 5 },
	));
	assert.equal(selections.selections, cancelledSelection);
	assert.deepEqual([...captured], []);

	selections.setSelections(SelectionSet.single(Selection.fromPositions(
		new Position((1) + 1, (2) + 1),
		new Position((1) + 1, (2) + 1),
	)));
	viewport.element.dispatchEvent(pointerEvent(
		dom.window,
		"pointerdown",
		110,
		95,
		{ pointerId: 6, shiftKey: true },
	));
	dom.window.dispatchEvent(pointerEvent(
		dom.window,
		"pointerup",
		110,
		95,
		{ pointerId: 6, shiftKey: true },
	));
	assert.deepEqual(selections.selections.primary, Selection.fromPositions(
		new Position((1) + 1, (2) + 1),
		new Position((3) + 1, (0) + 1),
	));

	pointer.dispose();
	const disposedSelection = selections.selections;
	viewport.element.dispatchEvent(pointerEvent(
		dom.window,
		"pointerdown",
		148,
		55,
		{ pointerId: 7 },
	));
	assert.equal(selections.selections, disposedSelection);

	dom.window.close();
});

test("Pointer and viewport selection wiring rejects different text models", () => {
	const dom = new JSDOM("<!doctype html><body><main></main></body>");
	const container = dom.window.document.querySelector("main");
	assert.ok(container);
	using model = new TextModel("alpha");
	using otherModel = new TextModel("beta");
	using selections = new CursorsController(
		otherModel,
		SelectionSet.single(
			Selection.fromPositions(new Position((0) + 1, (0) + 1)),
		),
	);
	assert.throws(() => new EditorViewport({
		container,
		model,
		lineHeight: 20,
		textMeasurer: new FixedTextMeasurer(),
		selectionController: selections,
	}), /must share one text model/);
	assert.equal(container.childElementCount, 0);

	using viewport = new EditorViewport({
		container,
		model,
		lineHeight: 20,
		textMeasurer: new FixedTextMeasurer(),
	});
	assert.throws(
		() => new MouseHandler(viewport, selections),
		/must share one text model/,
	);
	model.applyEdits([{
		range: Range.fromPositions(new Position((0) + 1, (5) + 1)),
		text: " editor",
	}]);
	assert.equal(model.getText(), "alpha editor");

	dom.window.close();
});

test("Alt+Shift pointer drag creates a front-end column selection", () => {
	const dom = new JSDOM("<!doctype html><body><main></main></body>");
	const container = dom.window.document.querySelector("main");
	assert.ok(container);
	using model = new TextModel("abcdef\nab\n12345\nxy");
	using selections = new CursorsController(model, SelectionSet.single(Selection.fromPositions(new Position((0) + 1, (0) + 1))));
	using viewport = new EditorViewport({
		container,
		model,
		glyphMargin: false,
		lineHeight: 20,
		textMeasurer: new FixedTextMeasurer(),
		selectionController: selections,
	});
	viewport.layout({ width: 200, height: 80 });
	viewport.element.getBoundingClientRect = () => editorBounds();
	const captured = new Set<number>();
	viewport.element.setPointerCapture = pointerId => captured.add(pointerId);
	viewport.element.hasPointerCapture = pointerId => captured.has(pointerId);
	viewport.element.releasePointerCapture = pointerId => captured.delete(pointerId);
	using pointer = new MouseHandler(viewport, selections);

	viewport.element.dispatchEvent(pointerEvent(dom.window, "pointerdown", 172, 115, {
		pointerId: 19,
		altKey: true,
		shiftKey: true,
	}));
	dom.window.dispatchEvent(pointerEvent(dom.window, "pointermove", 202, 55, {
		pointerId: 19,
		altKey: true,
		shiftKey: true,
	}));
	dom.window.dispatchEvent(pointerEvent(dom.window, "pointerup", 202, 55, {
		pointerId: 19,
		altKey: true,
		shiftKey: true,
	}));

	assert.deepEqual(selections.selections, SelectionSet.withPrimary([
		Selection.fromPositions(new Position((0) + 1, (2) + 1), new Position((0) + 1, (6) + 1)),
		Selection.fromPositions(new Position((1) + 1, (2) + 1), new Position((1) + 1, (2) + 1)),
		Selection.fromPositions(new Position((2) + 1, (2) + 1), new Position((2) + 1, (5) + 1)),
		Selection.fromPositions(new Position((3) + 1, (2) + 1), new Position((3) + 1, (2) + 1)),
	], 0));
	assert.deepEqual([...captured], []);
	dom.window.close();
});

test("Pointer drag anchor tracks model edits and window blur ends capture", () => {
	const dom = new JSDOM("<!doctype html><body><main></main></body>");
	const container = dom.window.document.querySelector("main");
	assert.ok(container);
	using model = new TextModel("abc\ndef");
	using selections = new CursorsController(
		model,
		SelectionSet.single(
			Selection.fromPositions(new Position((0) + 1, (0) + 1)),
		),
	);
	using viewport = new EditorViewport({
		container,
		model,
		glyphMargin: false,
		lineHeight: 20,
		textMeasurer: new FixedTextMeasurer(),
		selectionController: selections,
	});
	viewport.layout({ width: 200, height: 40 });
	viewport.element.getBoundingClientRect = () => ({
		...editorBounds(),
		bottom: 90,
		height: 40,
	});
	const captured = new Set<number>();
	viewport.element.setPointerCapture = pointerId => captured.add(pointerId);
	viewport.element.hasPointerCapture = pointerId => captured.has(pointerId);
	viewport.element.releasePointerCapture = pointerId => {
		captured.delete(pointerId);
	};
	using pointer = new MouseHandler(viewport, selections);

	viewport.element.dispatchEvent(pointerEvent(
		dom.window,
		"pointerdown",
		148,
		55,
		{ pointerId: 8 },
	));
	model.applyEdits([{
		range: Range.fromPositions(new Position((0) + 1, (0) + 1)),
		text: "X",
	}]);
	dom.window.dispatchEvent(pointerEvent(
		dom.window,
		"pointermove",
		158,
		75,
		{ pointerId: 8 },
	));
	assert.deepEqual(selections.selections.primary, Selection.fromPositions(
		new Position((0) + 1, (2) + 1),
		new Position((1) + 1, (2) + 1),
	));
	assert.deepEqual([...captured], [8]);

	dom.window.dispatchEvent(new dom.window.Event("blur"));
	assert.deepEqual([...captured], []);
	const blurredSelection = selections.selections;
	dom.window.dispatchEvent(pointerEvent(
		dom.window,
		"pointermove",
		168,
		75,
		{ pointerId: 8 },
	));
	assert.equal(selections.selections, blurredSelection);

	dom.window.close();
});

interface PointerEventOptions {
	readonly pointerId: number;
	readonly altKey?: boolean;
	readonly shiftKey?: boolean;
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
		altKey: options.altKey,
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
		right: 300,
		bottom: 130,
		width: 200,
		height: 80,
		toJSON: () => ({}),
	};
}
