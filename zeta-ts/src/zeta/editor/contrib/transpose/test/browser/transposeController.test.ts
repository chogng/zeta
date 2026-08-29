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
const { TransposeCommandId, TransposeController } = await import("../../browser/transposeController.js");

test("Transpose consumes Ctrl+T only for the VS Code macOS binding", () => {
	const dom = new JSDOM("<!doctype html><body><main></main></body>");
	const container = dom.window.document.querySelector<HTMLElement>("main")!;
	using model = new TextModel("abc");
	using selections = new CursorsController(model, SelectionSet.single(Selection.fromPositions(new Position((0) + 1, (1) + 1))));
	using viewport = new EditorViewport({ container, model, lineHeight: 20, textMeasurer: new FixedTextMeasurer(), selectionController: selections });
	viewport.layout({ width: 200, height: 60 });
	const input = h(dom.window.document, "textarea");
	container.append(input);
	const executedCommands: string[] = [];
	using controller = new TransposeController(input, viewport, selections, { operatingSystem: OperatingSystem.Macintosh }, (commandId, operation) => {
		executedCommands.push(commandId);
		return operation();
	});

	const transpose = keydown(dom.window, "t", { ctrlKey: true });
	input.dispatchEvent(transpose);
	assert.equal(transpose.defaultPrevented, true);
	assert.equal(model.getText(), "bac");
	assert.deepEqual(executedCommands, [TransposeCommandId]);
	const other = keydown(dom.window, "t", { ctrlKey: true, metaKey: true });
	input.dispatchEvent(other);
	assert.equal(other.defaultPrevented, false);

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
