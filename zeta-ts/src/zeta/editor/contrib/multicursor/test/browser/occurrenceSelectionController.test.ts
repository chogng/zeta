import assert from "node:assert/strict";
import test from "node:test";
import { JSDOM } from "jsdom";
import { type TextMeasurer } from "../../../../browser/measurement/fontMetrics.js";
import { EditorSelectionController } from "../../../../common/cursor/editorSelectionController.js";
import { TextSelection, TextSelectionSet } from "../../../../common/core/selection.js";
import { TextPosition } from "../../../../common/core/text.js";
import { TextModel } from "../../../../common/model/textModel.js";
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

const { EditorViewport } = await import("../../../../browser/view/editorViewport.js");
const { OccurrenceSelectionController } = await import("../../browser/occurrenceSelectionController.js");

test("Occurrence shortcuts select a word, add its next match, and select every match", () => {
	const dom = new JSDOM("<!doctype html><body><main></main></body>");
	const container = dom.window.document.querySelector<HTMLElement>("main")!;
	using model = new TextModel("echo echo\necho");
	using selections = new EditorSelectionController(model, TextSelectionSet.single(TextSelection.collapsedAt(TextPosition.at(0, 1))));
	using viewport = new EditorViewport({ container, model, lineHeight: 20, textMeasurer: new FixedTextMeasurer(), selectionController: selections });
	viewport.layout({ width: 200, height: 60 });
	const input = h(dom.window.document, "textarea");
	container.append(input);
	using controller = new OccurrenceSelectionController(input, viewport, selections);

	const selectWord = keydown(dom.window, "d", { ctrlKey: true });
	input.dispatchEvent(selectWord);
	assert.equal(selectWord.defaultPrevented, true);
	assert.deepEqual(selections.selections.primary, TextSelection.from(TextPosition.at(0, 0), TextPosition.at(0, 4)));
	input.dispatchEvent(keydown(dom.window, "d", { ctrlKey: true }));
	assert.equal(selections.selections.selections.length, 2);
	input.dispatchEvent(keydown(dom.window, "l", { ctrlKey: true, shiftKey: true }));
	assert.equal(selections.selections.selections.length, 3);

	dom.window.close();
});

test("Occurrence controller rejects cross-model wiring and leaves unrelated chords alone", () => {
	const dom = new JSDOM("<!doctype html><body><main></main></body>");
	const container = dom.window.document.querySelector<HTMLElement>("main")!;
	using model = new TextModel("echo");
	using other = new TextModel("echo");
	using selections = new EditorSelectionController(model, TextSelectionSet.single(TextSelection.collapsedAt(TextPosition.at(0, 0))));
	using otherSelections = new EditorSelectionController(other, TextSelectionSet.single(TextSelection.collapsedAt(TextPosition.at(0, 0))));
	using viewport = new EditorViewport({ container, model, lineHeight: 20, textMeasurer: new FixedTextMeasurer() });
	const input = h(dom.window.document, "textarea");
	container.append(input);
	using controller = new OccurrenceSelectionController(input, viewport, selections);
	const unrelated = keydown(dom.window, "d", { ctrlKey: true, altKey: true });
	input.dispatchEvent(unrelated);
	assert.equal(unrelated.defaultPrevented, false);
	assert.throws(() => new OccurrenceSelectionController(input, viewport, otherSelections), /must share one text model/);

	dom.window.close();
});

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

function keydown(targetWindow: typeof browserEnvironment.window, key: string, options: KeyboardEventInit = {}): KeyboardEvent {
	return new targetWindow.KeyboardEvent("keydown", {
		bubbles: true,
		cancelable: true,
		key,
		...options,
	}) as unknown as KeyboardEvent;
}
