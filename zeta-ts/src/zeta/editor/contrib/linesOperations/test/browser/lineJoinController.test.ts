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
const { LineJoinController, isStanzaJoinLinesChord } = await import("../../browser/lineJoinController.js");

test("Join-lines shortcut runs locally and leaves unrelated chords alone", () => {
	const dom = new JSDOM("<!doctype html><body><main></main></body>");
	const container = dom.window.document.querySelector<HTMLElement>("main")!;
	using model = new TextModel("first\n  second");
	using selections = new EditorSelectionController(model, TextSelectionSet.single(TextSelection.collapsedAt(TextPosition.at(0, 2))));
	using viewport = new EditorViewport({ container, model, lineHeight: 20, textMeasurer: new FixedTextMeasurer(), selectionController: selections });
	viewport.layout({ width: 200, height: 60 });
	const input = h(dom.window.document, "textarea");
	container.append(input);
	using controller = new LineJoinController(input, viewport, selections, { operatingSystem: OperatingSystem.Linux });

	const join = keydown(dom.window, "j", { ctrlKey: true });
	input.dispatchEvent(join);
	assert.equal(join.defaultPrevented, true);
	assert.equal(model.getText(), "first second");
	const unrelated = keydown(dom.window, "j", { metaKey: true });
	input.dispatchEvent(unrelated);
	assert.equal(unrelated.defaultPrevented, false);

	dom.window.close();
});

test("Join-lines uses Command+J on macOS", () => {
	const dom = new JSDOM("<!doctype html><body><main></main></body>");
	const container = dom.window.document.querySelector<HTMLElement>("main")!;
	using model = new TextModel("first\nsecond");
	using selections = new EditorSelectionController(model, TextSelectionSet.single(TextSelection.collapsedAt(TextPosition.at(0, 0))));
	using viewport = new EditorViewport({ container, model, lineHeight: 20, textMeasurer: new FixedTextMeasurer(), selectionController: selections });
	const input = h(dom.window.document, "textarea");
	container.append(input);
	using controller = new LineJoinController(input, viewport, selections, { operatingSystem: OperatingSystem.Macintosh });

	const join = keydown(dom.window, "j", { metaKey: true });
	input.dispatchEvent(join);
	assert.equal(join.defaultPrevented, true);
	assert.equal(model.getText(), "first second");
	assert.equal(isStanzaJoinLinesChord(keydown(dom.window, "j", { ctrlKey: true }), OperatingSystem.Macintosh), false);
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

function keydown(targetWindow: typeof browserEnvironment.window, key: string, options: { readonly ctrlKey?: boolean; readonly metaKey?: boolean } = {}): KeyboardEvent {
	return new targetWindow.KeyboardEvent("keydown", {
		bubbles: true,
		cancelable: true,
		key,
		ctrlKey: options.ctrlKey,
		metaKey: options.metaKey,
	}) as unknown as KeyboardEvent;
}
