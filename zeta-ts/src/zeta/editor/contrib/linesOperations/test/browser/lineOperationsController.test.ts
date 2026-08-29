import assert from "node:assert/strict";
import test from "node:test";
import { JSDOM } from "jsdom";
import { OperatingSystem } from "../../../../../base/common/platform.js";
import { type TextMeasurer } from "../../../../browser/config/fontMeasurements.js";
import { EditorSelectionController } from "../../../../common/cursor/cursor.js";
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

const { EditorViewport } = await import("../../../../browser/view.js");
const { LineOperationsController, resolveStanzaDuplicateLineDirection } = await import("../../browser/lineOperationsController.js");

test("Line operation shortcuts duplicate and delete through Stanza commands", () => {
	const dom = new JSDOM("<!doctype html><body><main></main></body>");
	const container = dom.window.document.querySelector<HTMLElement>("main")!;
	using model = new TextModel("zero\none\ntwo");
	using selections = new EditorSelectionController(model, TextSelectionSet.single(
		TextSelection.collapsedAt(TextPosition.at(1, 1)),
	));
	using viewport = new EditorViewport({
		container,
		model,
		lineHeight: 20,
		textMeasurer: new FixedTextMeasurer(),
		selectionController: selections,
	});
	viewport.layout({ width: 200, height: 60 });
	const input = h(dom.window.document, "textarea");
	container.append(input);
	using controller = new LineOperationsController(input, viewport, selections);

	const duplicate = keydown(dom.window, "ArrowDown", { shiftKey: true, altKey: true });
	input.dispatchEvent(duplicate);
	assert.equal(duplicate.defaultPrevented, true);
	assert.equal(model.getText(), "zero\none\none\ntwo");
	input.dispatchEvent(keydown(dom.window, "k", { ctrlKey: true, shiftKey: true }));
	assert.equal(model.getText(), "zero\none\ntwo");
	input.dispatchEvent(keydown(dom.window, "ArrowUp", { shiftKey: true, altKey: true }));
	assert.equal(model.getText(), "zero\none\none\ntwo");

	dom.window.close();
});

test("Line operation shortcuts insert blank lines above and below selected groups", () => {
	const dom = new JSDOM("<!doctype html><body><main></main></body>");
	const container = dom.window.document.querySelector<HTMLElement>("main")!;
	using model = new TextModel("zero\none");
	using selections = new EditorSelectionController(model, TextSelectionSet.single(TextSelection.collapsedAt(TextPosition.at(0, 1))));
	using viewport = new EditorViewport({ container, model, lineHeight: 20, textMeasurer: new FixedTextMeasurer(), selectionController: selections });
	viewport.layout({ width: 200, height: 60 });
	const input = h(dom.window.document, "textarea");
	container.append(input);
	using controller = new LineOperationsController(input, viewport, selections);

	const after = keydown(dom.window, "Enter", { ctrlKey: true });
	input.dispatchEvent(after);
	assert.equal(after.defaultPrevented, true);
	assert.equal(model.getText(), "zero\n\none");
	input.dispatchEvent(keydown(dom.window, "Enter", { ctrlKey: true, shiftKey: true }));
	assert.equal(model.getText(), "zero\n\n\none");

	dom.window.close();
});

test("Line operation shortcuts move selected lines without duplicating them", () => {
	const dom = new JSDOM("<!doctype html><body><main></main></body>");
	const container = dom.window.document.querySelector<HTMLElement>("main")!;
	using model = new TextModel("zero\none\ntwo");
	using selections = new EditorSelectionController(model, TextSelectionSet.single(
		TextSelection.collapsedAt(TextPosition.at(1, 1)),
	));
	using viewport = new EditorViewport({
		container,
		model,
		lineHeight: 20,
		textMeasurer: new FixedTextMeasurer(),
		selectionController: selections,
	});
	viewport.layout({ width: 200, height: 60 });
	const input = h(dom.window.document, "textarea");
	container.append(input);
	using controller = new LineOperationsController(input, viewport, selections);

	const moveDown = keydown(dom.window, "ArrowDown", { altKey: true });
	input.dispatchEvent(moveDown);
	assert.equal(moveDown.defaultPrevented, true);
	assert.equal(model.getText(), "zero\ntwo\none");
	assert.deepEqual(selections.selections.primary, TextSelection.collapsedAt(TextPosition.at(2, 1)));
	input.dispatchEvent(keydown(dom.window, "ArrowUp", { altKey: true }));
	assert.equal(model.getText(), "zero\none\ntwo");
	assert.deepEqual(selections.selections.primary, TextSelection.collapsedAt(TextPosition.at(1, 1)));

	dom.window.close();
});

test("Line operation controller rejects cross-model wiring and leaves unrelated chords alone", () => {
	const dom = new JSDOM("<!doctype html><body><main></main></body>");
	const container = dom.window.document.querySelector<HTMLElement>("main")!;
	using model = new TextModel("alpha");
	using other = new TextModel("beta");
	using selections = new EditorSelectionController(model, TextSelectionSet.single(TextSelection.collapsedAt(TextPosition.at(0, 0))));
	using otherSelections = new EditorSelectionController(other, TextSelectionSet.single(TextSelection.collapsedAt(TextPosition.at(0, 0))));
	using viewport = new EditorViewport({ container, model, lineHeight: 20, textMeasurer: new FixedTextMeasurer() });
	const input = h(dom.window.document, "textarea");
	container.append(input);
	using controller = new LineOperationsController(input, viewport, selections);
	const unrelated = keydown(dom.window, "ArrowDown", { altKey: true, ctrlKey: true });
	input.dispatchEvent(unrelated);
	assert.equal(unrelated.defaultPrevented, false);
	assert.equal(model.getText(), "alpha");
	assert.throws(() => new LineOperationsController(input, viewport, otherSelections), /must share one text model/);

	dom.window.close();
});

test("Line duplication reserves Linux Shift+Alt arrows for multi-cursor commands", () => {
	assert.equal(resolveStanzaDuplicateLineDirection(
		keydown(browserEnvironment.window, "ArrowDown", { shiftKey: true, altKey: true }),
		OperatingSystem.Linux,
	), undefined);
	assert.equal(resolveStanzaDuplicateLineDirection(
		keydown(browserEnvironment.window, "ArrowDown", { ctrlKey: true, shiftKey: true, altKey: true }),
		OperatingSystem.Linux,
	), "down");
	assert.equal(resolveStanzaDuplicateLineDirection(
		keydown(browserEnvironment.window, "ArrowUp", { shiftKey: true, altKey: true }),
		OperatingSystem.Windows,
	), "up");
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

interface KeyOptions {
	readonly ctrlKey?: boolean;
	readonly shiftKey?: boolean;
	readonly altKey?: boolean;
}

function keydown(targetWindow: typeof browserEnvironment.window, key: string, options: KeyOptions = {}): KeyboardEvent {
	return new targetWindow.KeyboardEvent("keydown", {
		bubbles: true,
		cancelable: true,
		key,
		ctrlKey: options.ctrlKey,
		shiftKey: options.shiftKey,
		altKey: options.altKey,
	}) as unknown as KeyboardEvent;
}
