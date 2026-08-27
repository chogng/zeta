import assert from "node:assert/strict";
import test from "node:test";
import { JSDOM } from "jsdom";
import { DecorationPresentation, createStanzaDecorationSource } from "../../../../browser/viewparts/decorations/decorationPresentation.js";
import { type TextMeasurer } from "../../../../browser/config/fontMeasurements.js";
import { LanguageBracketMatcher } from "../../common/bracketMatching.js";
import { LanguageConfigurationRegistry } from "../../../../common/languages/languageConfiguration.js";
import { TextDecorationCollection } from "../../../../common/model/decorationCollection.js";
import { EditorSelectionController } from "../../../../common/cursor/editorSelectionController.js";
import { TextSelection, TextSelectionSet } from "../../../../common/core/selection.js";
import { TextPosition } from "../../../../common/core/text.js";
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

const { EditorViewport } = await import("../../../../browser/view.js");
const { BracketMatchController } = await import("../../browser/bracketMatchController.js");

test("Bracket match controller projects current pairs and clears them for a range selection", () => {
	const dom = new JSDOM("<!doctype html><body><main></main></body>");
	const container = dom.window.document.querySelector<HTMLElement>("main")!;
	using model = new TextModel("function value() {\n}");
	using configurations = new LanguageConfigurationRegistry();
	using registration = configurations.register("typescript", {
		brackets: [{ open: "(", close: ")" }, { open: "{", close: "}" }],
	});
	using selections = new EditorSelectionController(model, TextSelectionSet.single(
		TextSelection.collapsedAt(TextPosition.at(0, 17)),
	));
	using matcher = new LanguageBracketMatcher(model, "typescript", configurations);
	using decorations = new TextDecorationCollection<void>(model);
	using viewport = new EditorViewport({
		container,
		model,
		glyphMargin: false,
		lineHeight: 20,
		textMeasurer: new FixedTextMeasurer(),
		selectionController: selections,
		decorationSources: [createStanzaDecorationSource(
			decorations,
			() => DecorationPresentation.BracketMatch,
		)],
	});
	using controller = new BracketMatchController(selections, matcher, decorations);
	viewport.layout({ width: 240, height: 40 });

	assert.deepEqual([...viewport.element.querySelectorAll<HTMLElement>(".bracket-match")].map(element => ({
		lineIndex: element.parentElement?.dataset.lineIndex,
		left: element.style.left,
		width: element.style.width,
	})), [{
		lineIndex: "0",
		left: "208px",
		width: "10px",
	}, {
		lineIndex: "1",
		left: "38px",
		width: "10px",
	}]);

	selections.setSelections(TextSelectionSet.single(
		TextSelection.from(TextPosition.at(0, 0), TextPosition.at(0, 1)),
	));
	assert.equal(viewport.element.querySelectorAll(".bracket-match").length, 0);
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
