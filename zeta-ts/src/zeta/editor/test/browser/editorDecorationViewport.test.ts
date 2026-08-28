import assert from "node:assert/strict";
import test from "node:test";
import { JSDOM } from "jsdom";
import { DecorationPresentation, createStanzaDecorationSource } from "../../browser/viewparts/decorations/decorationPresentation.js";
import { type TextMeasurer } from "../../browser/config/fontMeasurements.js";
import { createStanzaLanguageDiagnosticSource, resolveStanzaLanguageDiagnosticPresentation } from "../../contrib/gotoError/browser/languageDiagnosticPresentation.js";
import { TextDecorationCollection } from "../../common/model/decorationCollection.js";
import { LanguageDiagnosticDecorationBridge } from "../../contrib/gotoError/common/diagnosticDecorations.js";
import { LanguageResultAcceptance } from "../../common/languages/languageResultStore.js";
import { LanguageDiagnosticSeverity, createLanguageDiagnosticStore } from "../../common/languages/languageResults.js";
import { TextPosition, TextRange } from "../../common/core/text.js";
import { TextModel } from "../../common/model/textModel.js";
import { TrackedRangeStickiness } from "../../common/model/trackedRange.js";

const browserEnvironment = new JSDOM("<!doctype html><body></body>");
for (const [name, value] of Object.entries({
	window: browserEnvironment.window,
	document: browserEnvironment.window.document,
	Node: browserEnvironment.window.Node,
	Element: browserEnvironment.window.Element,
	HTMLElement: browserEnvironment.window.HTMLElement,
	Event: browserEnvironment.window.Event,
})) {
	Object.defineProperty(globalThis, name, {
		configurable: true,
		value,
	});
}

const { EditorTextDirection, EditorViewport } = await import(
	"../../browser/view.js"
);
const { EditorLineWrapping } = await import(
	"../../common/config/editorOptions.js"
);

test("Decoration sources project, update, and follow tracked model ranges", () => {
	const dom = new JSDOM("<!doctype html><body><main></main></body>");
	const container = requiredElement(dom.window.document, "main");
	using model = new TextModel("abcd\nefgh\nij");
	using matches = new TextDecorationCollection<string>(model);
	using diagnostics = new TextDecorationCollection<"error" | "warning">(model);
	const matchId = matches.add({
		range: TextRange.from(TextPosition.at(0, 1), TextPosition.at(1, 2)),
		stickiness: TrackedRangeStickiness.NeverGrowsAtEdges,
		metadata: "match",
	});
	const diagnosticId = diagnostics.add({
		range: TextRange.from(TextPosition.at(2, 0), TextPosition.at(2, 2)),
		stickiness: TrackedRangeStickiness.NeverGrowsAtEdges,
		metadata: "error",
	});
	let matchResolutionCount = 0;
	const matchSource = createStanzaDecorationSource(
		matches,
		() => {
			matchResolutionCount += 1;
			return DecorationPresentation.SearchMatch;
		},
	);
	const diagnosticSource = createStanzaDecorationSource(
		diagnostics,
		decoration => decoration.metadata === "error"
			? DecorationPresentation.ErrorUnderline
			: DecorationPresentation.WarningUnderline,
	);
	const viewport = new EditorViewport({
		container,
		model,
		glyphMargin: false,
		lineHeight: 20,
		textMeasurer: new FixedTextMeasurer(),
		decorationSources: [matchSource, diagnosticSource],
	});
	viewport.layout({ width: 300, height: 60 });
	viewport.scrollTo({ left: 0, top: 0 });
	assert.equal(matchResolutionCount, 1);

	assert.deepEqual(decorationElements(viewport.element).map(element => ({
		id: element.dataset.decorationId,
		presentation: element.classList[1],
		lineIndex: element.parentElement?.dataset.lineIndex,
		left: element.style.left,
		width: element.style.width,
	})), [{
		id: String(matchId),
		presentation: DecorationPresentation.SearchMatch,
		lineIndex: "0",
		left: "48px",
		width: "40px",
	}, {
		id: String(matchId),
		presentation: DecorationPresentation.SearchMatch,
		lineIndex: "1",
		left: "38px",
		width: "20px",
	}, {
		id: String(diagnosticId),
		presentation: DecorationPresentation.ErrorUnderline,
		lineIndex: "2",
		left: "38px",
		width: "20px",
	}]);
	const errorMarker = requiredElement<HTMLElement>(
		viewport.element,
		'.stanza-editor-diagnostic-marker[data-line-index="2"]',
	);
	assert.equal(errorMarker.hidden, false);
	assert.equal(errorMarker.classList.contains("error"), true);
	const errorOverview = requiredElement<HTMLElement>(viewport.element, ".stanza-editor-overview-marker");
	assert.equal(errorOverview.classList.contains(DecorationPresentation.ErrorUnderline), true);
	const errorMinimap = requiredElement<HTMLElement>(viewport.element, ".stanza-editor-minimap-diagnostic-marker");
	assert.equal(errorMinimap.classList.contains(DecorationPresentation.ErrorUnderline), true);
	assert.equal(errorMinimap.style.top, "40px");

	diagnostics.update(diagnosticId, {
		range: TextRange.from(TextPosition.at(1, 1), TextPosition.at(1, 3)),
		stickiness: TrackedRangeStickiness.NeverGrowsAtEdges,
		metadata: "warning",
	});
	const warning = requiredElement<HTMLElement>(
		viewport.element,
		`.stanza-editor-decoration[data-decoration-id="${diagnosticId}"]`,
	);
	assert.equal(
		warning.classList.contains(DecorationPresentation.WarningUnderline),
		true,
	);
	assert.equal(warning.parentElement?.dataset.lineIndex, "1");
	assert.equal(warning.style.left, "48px");
	assert.equal(warning.style.width, "20px");
	const warningMarker = requiredElement<HTMLElement>(
		viewport.element,
		'.stanza-editor-diagnostic-marker[data-line-index="1"]',
	);
	assert.equal(warningMarker.hidden, false);
	assert.equal(warningMarker.classList.contains("warning"), true);
	const warningOverview = requiredElement<HTMLElement>(viewport.element, ".stanza-editor-overview-marker");
	assert.equal(warningOverview.classList.contains(DecorationPresentation.WarningUnderline), true);
	const warningMinimap = requiredElement<HTMLElement>(viewport.element, ".stanza-editor-minimap-diagnostic-marker");
	assert.equal(warningMinimap.classList.contains(DecorationPresentation.WarningUnderline), true);
	assert.equal(warningMinimap.style.top, "20px");
	assert.equal(matchResolutionCount, 1);

	model.applyEdits([{
		range: TextRange.emptyAt(TextPosition.at(0, 0)),
		text: "X\n",
	}]);

	const trackedMatch = decorationElements(viewport.element).filter(
		element => element.dataset.decorationId === String(matchId),
	);
	assert.deepEqual(
		trackedMatch.map(
			element => element.parentElement?.dataset.lineIndex,
		),
		["1", "2"],
	);
	assert.equal(matchResolutionCount, 2);
	viewport.dispose();
	assert.equal(matches.size, 1);
	assert.equal(diagnostics.size, 1);

	dom.window.close();
});

test("Line and block decoration parts project source presentation details", () => {
	const dom = new JSDOM("<!doctype html><body><main></main></body>");
	const container = requiredElement(dom.window.document, "main");
	using model = new TextModel("first\nsecond\nthird");
	using decorations = new TextDecorationCollection<"lines" | "block">(model);
	decorations.add({
		range: TextRange.from(TextPosition.at(0, 0), TextPosition.at(2, 0)),
		stickiness: TrackedRangeStickiness.NeverGrowsAtEdges,
		metadata: "lines",
	});
	decorations.add({
		range: TextRange.from(TextPosition.at(0, 0), TextPosition.at(2, 1)),
		stickiness: TrackedRangeStickiness.NeverGrowsAtEdges,
		metadata: "block",
	});
	const source = createStanzaDecorationSource(
		decorations,
		decoration => decoration.metadata === "lines"
			? {
				presentation: DecorationPresentation.DiffModified,
				linesDecoration: {
					owner: "test-lines",
					className: "stanza-test-line-marker",
					firstLineClassName: "stanza-test-first-line-marker",
					tooltip: "line marker",
				},
			}
			: {
				presentation: DecorationPresentation.DiffModified,
				blockDecoration: {
					className: "stanza-test-block",
					padding: [1, 2, 3, 4],
				},
			},
		undefined,
		{ linesDecorationLanes: [{ owner: "test-lines", width: 4 }] },
	);
	using viewport = new EditorViewport({
		container,
		model,
		glyphMargin: false,
		lineHeight: 20,
		textMeasurer: new FixedTextMeasurer(),
		decorationSources: [source],
	});
	viewport.layout({ width: 200, height: 60 });

	const firstLine = requiredElement<HTMLElement>(viewport.element, '.stanza-editor-line-lines-decorations[data-line-index="0"]');
	const secondLine = requiredElement<HTMLElement>(viewport.element, '.stanza-editor-line-lines-decorations[data-line-index="1"]');
	const thirdLine = requiredElement<HTMLElement>(viewport.element, '.stanza-editor-line-lines-decorations[data-line-index="2"]');
	const firstMarker = requiredElement<HTMLElement>(firstLine, ".stanza-editor-line-decoration");
	const secondMarker = requiredElement<HTMLElement>(secondLine, ".stanza-editor-line-decoration");
	assert.equal(firstMarker.classList.contains("stanza-test-line-marker"), true);
	assert.equal(firstMarker.classList.contains("stanza-test-first-line-marker"), true);
	assert.equal(firstMarker.title, "line marker");
	assert.equal(firstMarker.dataset.decorationOwner, "test-lines");
	assert.equal(firstMarker.style.getPropertyValue("--stanza-editor-line-decoration-offset"), "0px");
	assert.equal(firstMarker.style.getPropertyValue("--stanza-editor-line-decoration-width"), "4px");
	assert.equal(secondMarker.classList.contains("stanza-test-line-marker"), true);
	assert.equal(secondMarker.classList.contains("stanza-test-first-line-marker"), false);
	assert.equal(thirdLine.querySelector(".stanza-editor-line-decoration"), null);

	const block = requiredElement<HTMLElement>(viewport.element, ".stanza-editor-block-decoration");
	assert.equal(block.classList.contains("stanza-test-block"), true);
	assert.equal(block.style.left, "38px");
	assert.equal(block.style.width, "164px");
	assert.equal(block.style.top, "-1px");
	assert.equal(block.style.height, "64px");
	dom.window.close();
});

test("Quick Diff decorations project into the overview ruler and minimap gutter", () => {
	const dom = new JSDOM("<!doctype html><body><main></main></body>");
	const container = requiredElement(dom.window.document, "main");
	using model = new TextModel("same\nadded\nmodified\nafter delete");
	using decorations = new TextDecorationCollection<DecorationPresentation>(model);
	decorations.replaceAll([
		{ range: TextRange.emptyAt(TextPosition.at(1, 0)), stickiness: TrackedRangeStickiness.NeverGrowsAtEdges, metadata: DecorationPresentation.DiffAdded },
		{ range: TextRange.emptyAt(TextPosition.at(2, 0)), stickiness: TrackedRangeStickiness.NeverGrowsAtEdges, metadata: DecorationPresentation.DiffModified },
		{ range: TextRange.emptyAt(TextPosition.at(3, 0)), stickiness: TrackedRangeStickiness.NeverGrowsAtEdges, metadata: DecorationPresentation.DiffDeleted },
	]);
	using viewport = new EditorViewport({
		container,
		model,
		lineHeight: 20,
		textMeasurer: new FixedTextMeasurer(),
		decorationSources: [createStanzaDecorationSource(decorations, decoration => ({
			presentation: decoration.metadata,
			overviewRuler: decoration.metadata !== DecorationPresentation.DiffAdded,
			minimap: decoration.metadata !== DecorationPresentation.DiffModified,
		}))],
	});
	viewport.layout({ width: 300, height: 80 });

	assert.deepEqual([...viewport.element.querySelectorAll<HTMLElement>(".stanza-editor-overview-marker")].map(marker => marker.classList[1]), [
		DecorationPresentation.DiffModified,
		DecorationPresentation.DiffDeleted,
	]);
	assert.deepEqual([...viewport.element.querySelectorAll<HTMLElement>(".stanza-editor-minimap-diagnostic-marker")].map(marker => marker.classList[1]), [
		DecorationPresentation.DiffAdded,
		DecorationPresentation.DiffDeleted,
	]);
	dom.window.close();
});

test("Decoration overlays use browser range rectangles for RTL text", () => {
	const dom = new JSDOM("<!doctype html><body><main></main></body>");
	const container = requiredElement(dom.window.document, "main");
	const createRange = dom.window.document.createRange.bind(dom.window.document);
	Object.defineProperty(dom.window.document, "createRange", {
		configurable: true,
		value: () => {
			const range = createRange();
			Object.defineProperty(range, "getClientRects", {
				configurable: true,
				value: () => [testRectangle(150, 0, 20), testRectangle(120, 0, 15)],
			});
			return range;
		},
	});
	using model = new TextModel("abc אבג");
	using decorations = new TextDecorationCollection<void>(model);
	const id = decorations.add({
		range: TextRange.from(TextPosition.at(0, 0), TextPosition.at(0, 3)),
		stickiness: TrackedRangeStickiness.NeverGrowsAtEdges,
		metadata: undefined,
	});
	const viewport = new EditorViewport({
		container,
		model,
		lineHeight: 20,
		textMeasurer: new FixedTextMeasurer(),
		textDirection: EditorTextDirection.RightToLeft,
		decorationSources: [createStanzaDecorationSource(decorations, () => DecorationPresentation.SearchMatch)],
	});
	viewport.layout({ width: 200, height: 40 });
	const line = requiredElement<HTMLElement>(viewport.element, ".stanza-editor-line");
	Object.defineProperty(line, "getBoundingClientRect", {
		configurable: true,
		value: () => testRectangle(100, 0, 200),
	});
	decorations.update(id, {
		range: TextRange.from(TextPosition.at(0, 0), TextPosition.at(0, 3)),
		stickiness: TrackedRangeStickiness.NeverGrowsAtEdges,
		metadata: undefined,
	});

	assert.deepEqual(decorationElements(viewport.element).map(element => ({ left: element.style.left, width: element.style.width })), [
		{ left: "20px", width: "15px" },
		{ left: "50px", width: "20px" },
	]);
	dom.window.close();
});

test("Decoration overlays split at soft-wrapped visual line boundaries", () => {
	const dom = new JSDOM("<!doctype html><body><main></main></body>");
	const container = requiredElement(dom.window.document, "main");
	using model = new TextModel("abcdef");
	using decorations = new TextDecorationCollection<void>(model);
	decorations.add({
		range: TextRange.from(TextPosition.at(0, 1), TextPosition.at(0, 5)),
		stickiness: TrackedRangeStickiness.NeverGrowsAtEdges,
		metadata: undefined,
	});
	using viewport = new EditorViewport({
		container,
		model,
		glyphMargin: false,
		lineHeight: 20,
		textMeasurer: new FixedTextMeasurer(),
		decorationSources: [createStanzaDecorationSource(
			decorations,
			() => DecorationPresentation.SearchMatch,
		)],
		lineWrapping: EditorLineWrapping.On,
		minimap: { enabled: false },
	});
	viewport.layout({ width: 70, height: 60 });

	assert.deepEqual(decorationElements(viewport.element).map(element => ({
		lineIndex: element.parentElement?.dataset.lineIndex,
		left: element.style.left,
		width: element.style.width,
	})), [{
		lineIndex: "0",
		left: "48px",
		width: "10px",
	}, {
		lineIndex: "1",
		left: "38px",
		width: "20px",
	}, {
		lineIndex: "2",
		left: "38px",
		width: "10px",
	}]);

	dom.window.close();
});

test("Versioned diagnostics project named severity underlines and invalidate", () => {
	const dom = new JSDOM("<!doctype html><body><main></main></body>");
	const container = requiredElement(dom.window.document, "main");
	using model = new TextModel("abcd\nefgh\nijkl\nmnop");
	using store = createLanguageDiagnosticStore(model);
	using bridge = new LanguageDiagnosticDecorationBridge(store);
	const source = createStanzaLanguageDiagnosticSource(bridge.decorations);
	const viewport = new EditorViewport({
		container,
		model,
		lineHeight: 20,
		textMeasurer: new FixedTextMeasurer(),
		decorationSources: [source],
	});
	viewport.layout({ width: 200, height: 80 });
	assert.equal(store.accept({
		requestId: 1,
		textModel: model,
		modelVersion: 1,
		value: {
			diagnostics: [
				{
					range: TextRange.from(TextPosition.at(0, 1), TextPosition.at(0, 3)),
					severity: LanguageDiagnosticSeverity.Error,
					message: "error",
					source: "language.lexical",
					code: "E100",
				},
				{
					range: TextRange.from(TextPosition.at(1, 0), TextPosition.at(1, 2)),
					severity: LanguageDiagnosticSeverity.Warning,
					message: "warning",
				},
				{
					range: TextRange.from(TextPosition.at(2, 0), TextPosition.at(2, 2)),
					severity: LanguageDiagnosticSeverity.Information,
					message: "information",
				},
				{
					range: TextRange.emptyAt(TextPosition.at(3, 1)),
					severity: LanguageDiagnosticSeverity.Hint,
					message: "hint",
				},
			],
		},
	}), LanguageResultAcceptance.Applied);

	assert.deepEqual(decorationElements(viewport.element).map(element => ({
		presentation: element.classList[1],
		lineIndex: element.parentElement?.dataset.lineIndex,
		title: element.title,
	})), [{
		presentation: DecorationPresentation.ErrorUnderline,
		lineIndex: "0",
		title: "language.lexical E100: error",
	}, {
		presentation: DecorationPresentation.WarningUnderline,
		lineIndex: "1",
		title: "warning",
	}, {
		presentation: DecorationPresentation.InformationUnderline,
		lineIndex: "2",
		title: "information",
	}, {
		presentation: DecorationPresentation.HintUnderline,
		lineIndex: "3",
		title: "hint",
	}]);
	assert.equal(
		resolveStanzaLanguageDiagnosticPresentation(
			LanguageDiagnosticSeverity.Information,
		),
		DecorationPresentation.InformationUnderline,
	);
	assert.equal(
		resolveStanzaLanguageDiagnosticPresentation(LanguageDiagnosticSeverity.Hint),
		DecorationPresentation.HintUnderline,
	);
	assert.throws(
		() => resolveStanzaLanguageDiagnosticPresentation(
			"fatal" as LanguageDiagnosticSeverity,
		),
		/Unknown language diagnostic severity/,
	);

	model.applyEdits([{
		range: TextRange.emptyAt(TextPosition.at(0, 0)),
		text: "X",
	}]);
	assert.deepEqual(decorationElements(viewport.element), []);
	assert.equal(store.result, undefined);

	viewport.dispose();
	assert.equal(store.accept({
		requestId: 2,
		textModel: model,
		modelVersion: 2,
		value: {
			diagnostics: [{
				range: TextRange.from(TextPosition.at(0, 0), TextPosition.at(0, 1)),
				severity: LanguageDiagnosticSeverity.Error,
				message: "after viewport",
			}],
		},
	}), LanguageResultAcceptance.Applied);
	assert.equal(bridge.decorations.size, 1);
	dom.window.close();
});

function requiredElement<T extends Element = HTMLElement>(
	container: ParentNode,
	selector: string,
): T {
	const element = container.querySelector<T>(selector);
	assert.ok(element, `Expected ${selector}`);
	return element;
}

function decorationElements(container: ParentNode): HTMLElement[] {
	return [...container.querySelectorAll<HTMLElement>(
		".stanza-editor-decoration",
	)];
}

function testRectangle(left: number, top: number, width: number): DOMRect {
	return { left, top, width, height: 20, right: left + width, bottom: top + 20, x: left, y: top, toJSON: () => ({}) } as DOMRect;
}

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
