import assert from "node:assert/strict";
import test from "node:test";
import { JSDOM } from "jsdom";
import { EditorIndentationKind } from "../../../../common/core/misc/indentation.js";
import { CursorsController } from "../../../../common/cursor/cursor.js";
import { Selection } from "../../../../common/core/selection.js";
import { SelectionSet } from "../../../../common/cursor/selectionSet.js";
import { Position } from "../../../../common/core/position.js";
import { TextModel } from "../../../../common/model/textModel.js";
import { type TextMeasurer } from "../../../../browser/config/fontMeasurements.js";
import { h } from "../../../../../base/browser/dom.js";

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

const { View } = await import("../../../../browser/view.js");
const { LineOperationsController } = await import("../../browser/lineOperationsController.js");
const { EditorView } = await import('../../../../browser/editorView.js');

test.after(() => browserEnvironment.window.close());

test("Line operations controller routes selected-line Tab and Shift+Tab through indentation commands", () => {
	const dom = new JSDOM("<!doctype html><body><main></main></body>");
	const container = dom.window.document.querySelector<HTMLElement>("main")!;
	using model = new TextModel("one\n  two\nthree");
	using selections = new CursorsController(model, SelectionSet.single(Selection.fromPositions(new Position((0) + 1, (0) + 1))));
	using viewport = new View({
		container,
		model,
		lineHeight: 20,
		textMeasurer: new FixedTextMeasurer(),
		selectionController: selections,
	});
	viewport.layout({ width: 400, height: 100 });
	using input = new EditorView(viewport, selections);
	using controller = new LineOperationsController(input.element, viewport, selections, {
		indentation: { kind: EditorIndentationKind.Spaces, tabSize: 2 },
	});

	const range = Selection.fromPositions(new Position((0) + 1, (0) + 1), new Position((2) + 1, (5) + 1));
	selections.setSelections(SelectionSet.single(range));

	const indent = keyboardEvent(dom.window, "Tab");
	input.element.dispatchEvent(indent);
	assert.equal(indent.defaultPrevented, true);
	assert.equal(model.getText(), "  one\n    two\n  three");

	const outdent = keyboardEvent(dom.window, "Tab", { shiftKey: true });
	input.element.dispatchEvent(outdent);
	assert.equal(outdent.defaultPrevented, true);
	assert.equal(model.getText(), "one\n  two\nthree");
	dom.window.close();
});

test("Line operations controller validates model ownership and indentation options", () => {
	const dom = new JSDOM("<!doctype html><body><main></main></body>");
	const container = dom.window.document.querySelector<HTMLElement>("main")!;
	using model = new TextModel("one");
	using otherModel = new TextModel("two");
	using selections = new CursorsController(model, SelectionSet.single(Selection.fromPositions(new Position((0) + 1, (0) + 1))));
	using otherSelections = new CursorsController(otherModel, SelectionSet.single(Selection.fromPositions(new Position((0) + 1, (0) + 1))));
	using viewport = new View({
		container,
		model,
		lineHeight: 20,
		textMeasurer: new FixedTextMeasurer(),
		selectionController: selections,
	});
	const input = h(dom.window.document, "textarea") as unknown as HTMLTextAreaElement;
	assert.throws(() => new LineOperationsController(input, viewport, otherSelections), /must share one text model/);
	assert.throws(() => new LineOperationsController(input, viewport, selections, {
		indentation: { tabSize: 0 },
	}), /tab size/);
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
