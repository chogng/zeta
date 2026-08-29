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
const { MultiCursorController, resolveStanzaAdjacentCursorDirection } = await import("../../browser/multiCursorController.js");

test("Multi-cursor shortcut adds a logical adjacent caret through Stanza common state", () => {
	const dom = new JSDOM("<!doctype html><body><main></main></body>");
	const container = dom.window.document.querySelector<HTMLElement>("main")!;
	using model = new TextModel("zero\none\ntwo");
	using selections = new EditorSelectionController(model, TextSelectionSet.single(TextSelection.collapsedAt(TextPosition.at(1, 1))));
	using viewport = new EditorViewport({ container, model, lineHeight: 20, textMeasurer: new FixedTextMeasurer(), selectionController: selections });
	viewport.layout({ width: 200, height: 60 });
	const input = h(dom.window.document, "textarea");
	container.append(input);
	using controller = new MultiCursorController(input, viewport, selections, { operatingSystem: OperatingSystem.Windows });

	const addBelow = keydown(dom.window, "ArrowDown", { ctrlKey: true, altKey: true });
	input.dispatchEvent(addBelow);
	assert.equal(addBelow.defaultPrevented, true);
	assert.deepEqual(selections.selections, TextSelectionSet.withPrimary([
		TextSelection.collapsedAt(TextPosition.at(1, 1)),
		TextSelection.collapsedAt(TextPosition.at(2, 1)),
	], 1));

	dom.window.close();
});

test("Multi-cursor shortcut replaces selected rows with line-end carets", () => {
	const dom = new JSDOM("<!doctype html><body><main></main></body>");
	const container = dom.window.document.querySelector<HTMLElement>("main")!;
	using model = new TextModel("zero\none\ntwo");
	using selections = new EditorSelectionController(model, TextSelectionSet.single(TextSelection.from(TextPosition.at(0, 1), TextPosition.at(2, 0))));
	using viewport = new EditorViewport({ container, model, lineHeight: 20, textMeasurer: new FixedTextMeasurer(), selectionController: selections });
	viewport.layout({ width: 200, height: 60 });
	const input = h(dom.window.document, "textarea");
	container.append(input);
	using controller = new MultiCursorController(input, viewport, selections);

	const addEnds = keydown(dom.window, "i", { shiftKey: true, altKey: true });
	input.dispatchEvent(addEnds);
	assert.equal(addEnds.defaultPrevented, true);
	assert.deepEqual(selections.selections, TextSelectionSet.withPrimary([
		TextSelection.collapsedAt(TextPosition.at(0, 4)),
		TextSelection.collapsedAt(TextPosition.at(1, 3)),
	], 0));

	dom.window.close();
});

test("Multi-cursor chord selection follows platform-specific non-conflicting bindings", () => {
	assert.equal(resolveStanzaAdjacentCursorDirection(
		keydown(browserEnvironment.window, "ArrowUp", { ctrlKey: true, altKey: true }),
		OperatingSystem.Windows,
	), "above");
	assert.equal(resolveStanzaAdjacentCursorDirection(
		keydown(browserEnvironment.window, "ArrowDown", { metaKey: true, altKey: true }),
		OperatingSystem.Macintosh,
	), "below");
	assert.equal(resolveStanzaAdjacentCursorDirection(
		keydown(browserEnvironment.window, "ArrowUp", { shiftKey: true, altKey: true }),
		OperatingSystem.Linux,
	), undefined);
	assert.equal(resolveStanzaAdjacentCursorDirection(
		keydown(browserEnvironment.window, "ArrowUp", { ctrlKey: true, shiftKey: true, altKey: true }),
		OperatingSystem.Linux,
	), "above");
});

test("Multi-cursor controller rejects cross-model wiring", () => {
	const dom = new JSDOM("<!doctype html><body><main></main></body>");
	const container = dom.window.document.querySelector<HTMLElement>("main")!;
	using model = new TextModel("one");
	using other = new TextModel("two");
	using selections = new EditorSelectionController(model, TextSelectionSet.single(TextSelection.collapsedAt(TextPosition.at(0, 0))));
	using otherSelections = new EditorSelectionController(other, TextSelectionSet.single(TextSelection.collapsedAt(TextPosition.at(0, 0))));
	using viewport = new EditorViewport({ container, model, lineHeight: 20, textMeasurer: new FixedTextMeasurer() });
	const input = h(dom.window.document, "textarea");
	assert.throws(() => new MultiCursorController(input, viewport, otherSelections), /must share one text model/);
	assert.throws(() => new MultiCursorController(input, viewport, selections, {
		operatingSystem: "solar" as OperatingSystem,
	}), /operating system/);

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
