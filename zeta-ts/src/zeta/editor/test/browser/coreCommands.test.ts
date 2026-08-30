import assert from "node:assert/strict";
import test from "node:test";
import { JSDOM } from "jsdom";
import { type TextMeasurer } from "../../common/viewModel/textMeasurer.js";
import { CursorsController } from "../../common/cursor/cursor.js";
import { Selection } from "../../common/core/selection.js";
import { Position } from "../../common/core/position.js";
import { TextModel } from "../../common/model/textModel.js";
import { h } from "../../../base/browser/dom.js";

const browserEnvironment = new JSDOM("<!doctype html><body></body>");
for (const [name, value] of Object.entries({
	window: browserEnvironment.window,
	document: browserEnvironment.window.document,
	Node: browserEnvironment.window.Node,
	Element: browserEnvironment.window.Element,
	HTMLElement: browserEnvironment.window.HTMLElement,
	Event: browserEnvironment.window.Event,
	KeyboardEvent: browserEnvironment.window.KeyboardEvent,
})) {
	Object.defineProperty(globalThis, name, { configurable: true, value });
}

const { View } = await import("../../browser/view.js");
const { installCoreTextEditorCommands } = await import("../../browser/coreCommands.js");
const { LineSelectionController } = await import("../../contrib/lineSelection/browser/lineSelectionController.js");
const { EditorView } = await import('../../browser/editorView.js');

test.after(() => browserEnvironment.window.close());

test("core commands select all", () => {
	const dom = new JSDOM("<!doctype html><body><main></main></body>");
	const container = dom.window.document.querySelector<HTMLElement>("main")!;
	using model = new TextModel("one\n  two\nthree");
	using selections = new CursorsController(model, [Selection.fromPositions(new Position((0) + 1, (0) + 1))]);
	using viewport = new View({
		container,
		model,
		lineHeight: 20,
		textMeasurer: new FixedTextMeasurer(),
		selectionController: selections,
	});
	viewport.layout({ width: 400, height: 100 });
	using input = new EditorView(viewport, selections);
	using commands = installCoreTextEditorCommands(input.element, viewport, selections);

	const selectAll = keyboardEvent(dom.window, "a", { metaKey: true });
	input.element.dispatchEvent(selectAll);
	assert.equal(selectAll.defaultPrevented, true);
	assert.deepEqual(selections.selections[0]!.getEndPosition(), new Position((2) + 1, (5) + 1));

	dom.window.close();
});

test("line selection remains an independent editor extension", () => {
	const dom = new JSDOM("<!doctype html><body><main></main></body>");
	const container = dom.window.document.querySelector<HTMLElement>("main")!;
	using model = new TextModel("one\ntwo\nthree");
	using selections = new CursorsController(model, [Selection.fromPositions(new Position((0) + 1, (1) + 1))]);
	using viewport = new View({ container, model, lineHeight: 20, textMeasurer: new FixedTextMeasurer(), selectionController: selections });
	viewport.layout({ width: 400, height: 100 });
	using input = new EditorView(viewport, selections);
	using commands = new LineSelectionController(input.element, viewport, selections);

	const first = keyboardEvent(dom.window, "l", { ctrlKey: true });
	input.element.dispatchEvent(first);
	assert.equal(first.defaultPrevented, true);
	assert.deepEqual(selections.selections[0]!, Selection.fromPositions(new Position((0) + 1, (0) + 1), new Position((1) + 1, (0) + 1)));
	input.element.dispatchEvent(keyboardEvent(dom.window, "l", { ctrlKey: true }));
	assert.deepEqual(selections.selections[0]!, Selection.fromPositions(new Position((0) + 1, (0) + 1), new Position((2) + 1, (0) + 1)));

	dom.window.close();
});

test("core commands reject dependencies from different text models", () => {
	const dom = new JSDOM("<!doctype html><body><main></main></body>");
	using model = new TextModel("one");
	using otherModel = new TextModel("two");
	using selections = new CursorsController(model, [Selection.fromPositions(new Position((0) + 1, (0) + 1))]);
	using otherSelections = new CursorsController(otherModel, [Selection.fromPositions(new Position((0) + 1, (0) + 1))]);
	using viewport = new View({
		container: dom.window.document.querySelector<HTMLElement>("main")!,
		model,
		lineHeight: 20,
		textMeasurer: new FixedTextMeasurer(),
		selectionController: selections,
	});
	const input = h(dom.window.document, "textarea") as unknown as HTMLTextAreaElement;
	assert.throws(() => installCoreTextEditorCommands(input, viewport, otherSelections), /must share one text model/);
	dom.window.close();
});

function keyboardEvent(targetWindow: typeof browserEnvironment.window, key: string, options: KeyboardEventInit = {}): KeyboardEvent {
	return new targetWindow.KeyboardEvent("keydown", {
		bubbles: true,
		cancelable: true,
		key,
		...options,
	}) as unknown as KeyboardEvent;
}

class FixedTextMeasurer implements TextMeasurer {
	readonly horizontalPadding = 24;
	readonly contentLeftPadding = 12;

	refresh(): boolean {
		return false;
	}

	measureLineWidth(text: string): number {
		return text.length * 10;
	}
}
