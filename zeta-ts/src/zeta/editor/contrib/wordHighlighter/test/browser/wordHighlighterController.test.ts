import assert from "node:assert/strict";
import test from "node:test";
import { JSDOM } from "jsdom";
import { DecorationPresentation, createStanzaDecorationSource } from "../../../../browser/viewparts/decorations/decorationPresentation.js";
import { type TextMeasurer } from "../../../../browser/measurement/fontMetrics.js";
import { TextDecorationCollection } from "../../../../common/model/decorationCollection.js";
import { EditorCommandHistoryMode, EditorSelectionController } from "../../../../common/cursor/editorSelectionController.js";
import { TextSelection, TextSelectionSet } from "../../../../common/core/selection.js";
import { TextPosition, TextRange } from "../../../../common/core/text.js";
import { TextModel } from "../../../../common/model/textModel.js";

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

const { EditorViewport } = await import("../../../../browser/view/editorViewport.js");
const { OccurrenceHighlightController } = await import("../../browser/wordHighlighterController.js");

test("Occurrence highlight controller projects and clears current-word decorations without changing selections", () => {
	const dom = new JSDOM("<!doctype html><body><main></main></body>");
	const container = dom.window.document.querySelector<HTMLElement>("main")!;
	using model = new TextModel("item itemized item\nitem");
	using selections = new EditorSelectionController(model, TextSelectionSet.single(TextSelection.collapsedAt(TextPosition.at(0, 1))));
	using decorations = new TextDecorationCollection<void>(model);
	using viewport = new EditorViewport({
		container,
		model,
		lineHeight: 20,
		textMeasurer: new FixedTextMeasurer(),
		selectionController: selections,
		decorationSources: [createStanzaDecorationSource(decorations, () => DecorationPresentation.OccurrenceHighlight)],
	});
	using controller = new OccurrenceHighlightController(selections, decorations);
	viewport.layout({ width: 240, height: 40 });

	assert.equal(decorations.size, 3);
	assert.equal(viewport.element.querySelectorAll(".occurrence-highlight").length, 3);
	assert.deepEqual(selections.selections.primary, TextSelection.collapsedAt(TextPosition.at(0, 1)));

	selections.setSelections(TextSelectionSet.single(TextSelection.collapsedAt(TextPosition.at(0, 4))));
	assert.equal(decorations.size, 0);
	assert.equal(viewport.element.querySelectorAll(".occurrence-highlight").length, 0);
	dom.window.close();
});

test("Occurrence highlight controller ignores the transient pre-command selection during replacement", () => {
	using model = new TextModel("const paper = 1;");
	using selections = new EditorSelectionController(model, TextSelectionSet.single(TextSelection.from(
		TextPosition.at(0, 0),
		TextPosition.at(0, model.getLineLength(0)),
	)));
	using decorations = new TextDecorationCollection<void>(model);
	using controller = new OccurrenceHighlightController(selections, decorations);

	assert.doesNotThrow(() => selections.execute({
		edits: [{ range: TextRange.from(TextPosition.at(0, 0), TextPosition.at(0, model.getLineLength(0))), text: "x" }],
		selectionsAfter: [{ anchorOffset: 1, activeOffset: 1 }],
		primarySelectionIndex: 0,
		historyMode: EditorCommandHistoryMode.Isolated,
	}));
	assert.equal(model.getText(), "x");
	assert.deepEqual(selections.selections.primary, TextSelection.collapsedAt(TextPosition.at(0, 1)));
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
