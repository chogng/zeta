import assert from "node:assert/strict";
import test from "node:test";
import { JSDOM } from "jsdom";
import { DecorationPresentation, createStanzaDecorationSource } from "../../../../browser/viewParts/decorations/decorations.js";
import { type TextMeasurer } from "../../../../common/viewModel/textMeasurer.js";
import { LanguageBracketPairs } from "../../../../common/languages/languageBracketPairs.js";
import { TestLanguageConfigurationService } from '../../../../test/common/modes/testLanguageConfigurationService.js';
import { LanguageLexicalContextIndex } from "../../../../common/languages/languageLexicalContext.js";
import { TextDecorationCollection } from "../../../../common/model/decorationCollection.js";
import { CursorsController } from "../../../../common/cursor/cursor.js";
import { Selection } from "../../../../common/core/selection.js";
import { Position } from "../../../../common/core/position.js";
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

const { View } = await import("../../../../browser/view.js");
const { BracketMatchController } = await import("../../browser/bracketMatchController.js");

test("Bracket match controller projects current pairs and clears them for a range selection", () => {
	const dom = new JSDOM("<!doctype html><body><main></main></body>");
	const container = dom.window.document.querySelector<HTMLElement>("main")!;
	using model = new TextModel("function value() {\n}");
	using configurations = new TestLanguageConfigurationService();
	using registration = configurations.register("typescript", {
		brackets: [["(", ")"], ["{", "}"]],
	});
	using selections = new CursorsController(model, [Selection.fromPositions(new Position((0) + 1, (17) + 1))]);
	using lexical = new LanguageLexicalContextIndex(model, "typescript", configurations);
	using bracketPairs = new LanguageBracketPairs(model, lexical);
	using decorations = new TextDecorationCollection<void>(model);
	using viewport = new View({
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
	using controller = new BracketMatchController(selections, bracketPairs, decorations, "always");
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

	selections.setSelections([Selection.fromPositions(new Position((0) + 1, (0) + 1), new Position((0) + 1, (1) + 1))]);
	assert.equal(viewport.element.querySelectorAll(".bracket-match").length, 0);
	dom.window.close();
});

test("Bracket match controller distinguishes near, always, and never modes", () => {
	using model = new TextModel("{ value }");
	using configurations = new TestLanguageConfigurationService();
	using registration = configurations.register("typescript", { brackets: [["{", "}"]] });
	using lexical = new LanguageLexicalContextIndex(model, "typescript", configurations);
	using bracketPairs = new LanguageBracketPairs(model, lexical);
	using selections = new CursorsController(model, [Selection.fromPositions(new Position((0) + 1, (3) + 1))]);

	using nearDecorations = new TextDecorationCollection<void>(model);
	using near = new BracketMatchController(selections, bracketPairs, nearDecorations, "near");
	assert.equal(nearDecorations.size, 0);

	using alwaysDecorations = new TextDecorationCollection<void>(model);
	using always = new BracketMatchController(selections, bracketPairs, alwaysDecorations, "always");
	assert.equal(alwaysDecorations.size, 2);

	using neverDecorations = new TextDecorationCollection<void>(model);
	using never = new BracketMatchController(selections, bracketPairs, neverDecorations, "never");
	assert.equal(neverDecorations.size, 0);
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
