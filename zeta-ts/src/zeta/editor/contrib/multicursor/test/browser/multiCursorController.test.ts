import assert from "node:assert/strict";
import test from "node:test";
import { JSDOM } from "jsdom";
import { OperatingSystem } from "../../../../../base/common/platform.js";
import { type TextMeasurer } from "../../../../browser/config/fontMeasurements.js";
import { CursorsController } from "../../../../common/cursor/cursor.js";
import { Selection } from "../../../../common/core/selection.js";
import { SelectionSet } from "../../../../common/cursor/selectionSet.js";
import { Position } from "../../../../common/core/position.js";
import { TextModel } from "../../../../common/model/textModel.js";
import { h } from "../../../../../base/browser/dom.js";

const browserEnvironment = new JSDOM("<!doctype html><body></body>");
class TestResizeObserver {
	observe(): void {}
	unobserve(): void {}
	disconnect(): void {}
}

for (const [name, value] of Object.entries({
	window: browserEnvironment.window,
	document: browserEnvironment.window.document,
	Node: browserEnvironment.window.Node,
	Element: browserEnvironment.window.Element,
	HTMLElement: browserEnvironment.window.HTMLElement,
	Event: browserEnvironment.window.Event,
	KeyboardEvent: browserEnvironment.window.KeyboardEvent,
	ResizeObserver: TestResizeObserver,
})) {
	Object.defineProperty(globalThis, name, { configurable: true, value });
}

const { View } = await import("../../../../browser/view.js");
const { MultiCursorController, resolveStanzaAdjacentCursorDirection } = await import("../../browser/multiCursorController.js");

test("Multi-cursor shortcut adds a logical adjacent caret through Stanza common state", () => {
	const dom = new JSDOM("<!doctype html><body><main></main></body>");
	const container = dom.window.document.querySelector<HTMLElement>("main")!;
	using model = new TextModel("zero\none\ntwo");
	using selections = new CursorsController(model, SelectionSet.single(Selection.fromPositions(new Position((1) + 1, (1) + 1))));
	using viewport = new View({ container, model, lineHeight: 20, textMeasurer: new FixedTextMeasurer(), selectionController: selections });
	viewport.layout({ width: 200, height: 60 });
	const input = h(dom.window.document, "textarea");
	container.append(input);
	using controller = new MultiCursorController(input, viewport, selections, { operatingSystem: OperatingSystem.Windows });

	const addBelow = keydown(dom.window, "ArrowDown", { ctrlKey: true, altKey: true });
	input.dispatchEvent(addBelow);
	assert.equal(addBelow.defaultPrevented, true);
	assert.deepEqual(selections.selections, SelectionSet.withPrimary([
		Selection.fromPositions(new Position((1) + 1, (1) + 1)),
		Selection.fromPositions(new Position((2) + 1, (1) + 1)),
	], 1));

	dom.window.close();
});

test("Multi-cursor shortcut replaces selected rows with line-end carets", () => {
	const dom = new JSDOM("<!doctype html><body><main></main></body>");
	const container = dom.window.document.querySelector<HTMLElement>("main")!;
	using model = new TextModel("zero\none\ntwo\nthree");
	using selections = new CursorsController(model, SelectionSet.withPrimary([
		Selection.fromPositions(new Position((0) + 1, (1) + 1), new Position((2) + 1, (0) + 1)),
		Selection.fromPositions(new Position((2) + 1, (0) + 1), new Position((3) + 1, (2) + 1)),
	], 1));
	using viewport = new View({ container, model, lineHeight: 20, textMeasurer: new FixedTextMeasurer(), selectionController: selections });
	viewport.layout({ width: 200, height: 80 });
	const input = h(dom.window.document, "textarea");
	container.append(input);
	using controller = new MultiCursorController(input, viewport, selections);

	const addEnds = keydown(dom.window, "i", { shiftKey: true, altKey: true });
	input.dispatchEvent(addEnds);
	assert.equal(addEnds.defaultPrevented, true);
	assert.deepEqual(selections.selections, SelectionSet.withPrimary([
		Selection.fromPositions(new Position((0) + 1, (4) + 1)),
		Selection.fromPositions(new Position((1) + 1, (3) + 1)),
		Selection.fromPositions(new Position((2) + 1, (3) + 1)),
		Selection.fromPositions(new Position((3) + 1, (2) + 1)),
	], 2));

	const collapsed = SelectionSet.single(Selection.fromPositions(new Position((0) + 1, (0) + 1)));
	selections.setSelections(collapsed);
	const ignoredAddEnds = keydown(dom.window, "i", { shiftKey: true, altKey: true });
	input.dispatchEvent(ignoredAddEnds);
	assert.equal(ignoredAddEnds.defaultPrevented, false);
	assert.equal(selections.selections, collapsed);

	dom.window.close();
});

test("Multi-cursor chord selection follows platform-specific non-conflicting bindings", () => {
	assert.equal(resolveStanzaAdjacentCursorDirection(
		keydown(browserEnvironment.window, "ArrowUp", { ctrlKey: true, altKey: true }),
		OperatingSystem.Windows,
	), "up");
	assert.equal(resolveStanzaAdjacentCursorDirection(
		keydown(browserEnvironment.window, "ArrowDown", { metaKey: true, altKey: true }),
		OperatingSystem.Macintosh,
	), "down");
	assert.equal(resolveStanzaAdjacentCursorDirection(
		keydown(browserEnvironment.window, "ArrowUp", { shiftKey: true, altKey: true }),
		OperatingSystem.Linux,
	), undefined);
	assert.equal(resolveStanzaAdjacentCursorDirection(
		keydown(browserEnvironment.window, "ArrowUp", { ctrlKey: true, shiftKey: true, altKey: true }),
		OperatingSystem.Linux,
	), "up");
});

test("Multi-cursor controller rejects cross-model wiring", () => {
	const dom = new JSDOM("<!doctype html><body><main></main></body>");
	const container = dom.window.document.querySelector<HTMLElement>("main")!;
	using model = new TextModel("one");
	using other = new TextModel("two");
	using selections = new CursorsController(model, SelectionSet.single(Selection.fromPositions(new Position((0) + 1, (0) + 1))));
	using otherSelections = new CursorsController(other, SelectionSet.single(Selection.fromPositions(new Position((0) + 1, (0) + 1))));
	using viewport = new View({ container, model, lineHeight: 20, textMeasurer: new FixedTextMeasurer() });
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
