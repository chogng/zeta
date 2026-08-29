import assert from "node:assert/strict";
import test from "node:test";
import { JSDOM } from "jsdom";
import { DecorationPresentation, createStanzaDecorationSource } from "../../../../browser/viewparts/decorations/decorations.js";
import { DiagnosticHoverController } from "../../browser/diagnosticHoverController.js";
import { type TextMeasurer } from "../../../../browser/config/fontMeasurements.js";
import { TextDecorationCollection } from "../../../../common/model/decorationCollection.js";
import { Position } from "../../../../common/core/position.js";
import { Range } from "../../../../common/core/range.js";
import { TextModel } from "../../../../common/model/textModel.js";
import { TrackedRangeStickiness } from "../../../../common/model/trackedRange.js";

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
	Object.defineProperty(globalThis, name, { configurable: true, value });
}

const { EditorViewport } = await import("../../../../browser/view.js");

test("Diagnostic hover presents current gutter-marker messages and hides on pointer exit", () => {
	const dom = new JSDOM("<!doctype html><body><main></main></body>");
	const container = dom.window.document.querySelector<HTMLElement>("main")!;
	using model = new TextModel("const value");
	using decorations = new TextDecorationCollection<string>(model);
	decorations.add({
		range: Range.fromPositions(new Position((0) + 1, (0) + 1), new Position((0) + 1, (5) + 1)),
		stickiness: TrackedRangeStickiness.NeverGrowsAtEdges,
		metadata: "Use let instead",
	});
	using viewport = new EditorViewport({
		container,
		model,
		lineHeight: 20,
		textMeasurer: new FixedTextMeasurer(),
		decorationSources: [createStanzaDecorationSource(
			decorations,
			() => DecorationPresentation.WarningUnderline,
			decoration => decoration.metadata,
		)],
	});
	using controller = new DiagnosticHoverController(viewport);
	viewport.layout({ width: 160, height: 20 });
	const marker = viewport.element.querySelector<HTMLElement>(".stanza-editor-diagnostic-marker")!;
	marker.dispatchEvent(new dom.window.Event("pointerover", { bubbles: true }));
	const hover = dom.window.document.body.querySelector<HTMLElement>(".stanza-editor-diagnostic-hover")!;
	assert.equal(hover.hidden, false);
	assert.equal(hover.textContent, "Use let instead");

	marker.dispatchEvent(new dom.window.Event("pointerout", { bubbles: true }));
	assert.equal(hover.hidden, true);
	dom.window.close();
});
