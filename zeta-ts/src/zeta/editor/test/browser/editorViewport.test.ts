import assert from "node:assert/strict";
import test from "node:test";
import { JSDOM } from "jsdom";
import { h } from "../../../base/browser/dom.js";
import { Emitter } from '../../../base/common/event.js';
import { ContentWidgetPositionPreference, type IContentWidgetPosition, OverlayWidgetPositionPreference } from '../../browser/editorBrowser.js';
import { type TextMeasurer } from "../../browser/config/fontMeasurements.js";
import { createStanzaDecorationSource, DecorationPresentation } from "../../browser/viewParts/decorations/decorations.js";
import { type BracketColorizationSource } from '../../browser/viewParts/viewLines/viewLine.js';
import { CursorsController } from "../../common/cursor/cursor.js";
import { EditorFoldingModel } from "../../contrib/folding/browser/foldingModel.js";
import { EditorHiddenRangeModel } from "../../contrib/folding/browser/hiddenRangeModel.js";
import { EditorFoldingDecorationSource } from '../../contrib/folding/browser/editorFoldingDecorationSource.js';
import { Selection } from "../../common/core/selection.js";
import { SelectionSet } from "../../common/cursor/selectionSet.js";
import { Position } from "../../common/core/position.js";
import { Range } from "../../common/core/range.js";
import { WrappingIndent } from "../../common/config/editorOptions.js";
import { TextModel } from "../../common/model/textModel.js";
import { PositionAffinity, TrackedRangeStickiness, GlyphMarginLane } from '../../common/model.js';
import { TextDecorationCollection } from "../../common/model/decorationCollection.js";

import { SemanticTokenModifier, SemanticTokenPresentation, type ResolvedSemanticToken, type SemanticTokenSource } from '../../common/services/resolvedSemanticTokens.js';

const browserEnvironment = new JSDOM("<!doctype html><body></body>");
for (const [name, value] of Object.entries({
	window: browserEnvironment.window,
	document: browserEnvironment.window.document,
	Node: browserEnvironment.window.Node,
	Element: browserEnvironment.window.Element,
	HTMLElement: browserEnvironment.window.HTMLElement,
	Event: browserEnvironment.window.Event,
	ResizeObserver: class {
		observe(): void {}
		unobserve(): void {}
		disconnect(): void {}
	},
})) {
	Object.defineProperty(globalThis, name, {
		configurable: true,
		value,
	});
}

const { View } = await import(
	"../../browser/view.js"
);
const { EditorTextDirection } = await import(
	"../../browser/view.js"
);
const { EditorLineWrapping } = await import(
	"../../common/config/editorOptions.js"
);

test("View projects the initial virtual line window", () => {
	const dom = new JSDOM("<!doctype html><body><main></main></body>");
	const container = requiredElement(dom.window.document, "main");
	using model = new TextModel([
		"<strong>not markup</strong>",
		...lines(99),
	].join("\n"));
	using viewport = new View({
		container,
		model,
		lineHeight: 20,
		overscanLineCount: 2,
		ariaLabel: "Read-only source",
		textMeasurer: fixedTextMeasurer(),
	});

	viewport.layout({ width: 300, height: 100 });
	const rows = lineElements(viewport.element);

	assert.equal(viewport.element.getAttribute("role"), "region");
	assert.equal(viewport.element.getAttribute("aria-label"), "Read-only source");
	assert.equal(viewport.element.tabIndex, 0);
	assert.equal(viewport.element.parentElement, container);
	assert.equal(viewport.element.querySelector("strong"), null);
	assert.equal(rows.length, 7);
	assert.equal(rows[0]?.dataset.lineIndex, "0");
	assert.equal(
		lineText(rows[0]).textContent,
		"<strong>not markup</strong>",
	);
	assert.equal(lineNumber(rows[0]).textContent, "1");
	assert.equal(rows[0]?.style.height, "20px");
	assert.equal(rows[6]?.dataset.lineIndex, "6");
	assert.equal(lineNumber(rows[6]).textContent, "7");
	assert.equal(
		viewport.element.style.getPropertyValue(
			"--stanza-editor-gutter-width",
		),
		"60px",
	);
	assert.equal(
		requiredElement(viewport.element, ".stanza-editor-content").style.height,
		"2000px",
	);

	dom.window.close();
});

test("View gives browser text shaping an explicit paragraph direction", () => {
	const dom = new JSDOM("<!doctype html><body><main></main></body>");
	const container = requiredElement(dom.window.document, "main");
	using model = new TextModel("שלום alpha");
	using viewport = new View({
		container,
		model,
		lineHeight: 20,
		textMeasurer: fixedTextMeasurer(),
		textDirection: EditorTextDirection.RightToLeft,
	});
	viewport.layout({ width: 300, height: 40 });

	assert.equal(viewport.editorTextDirection, EditorTextDirection.RightToLeft);
	assert.equal(viewport.element.dir, "rtl");
	assert.equal(viewport.element.classList.contains("stanza-editor-direction-rtl"), true);
	assert.equal(lineText(requiredLine(viewport.element, 0)).dir, "rtl");
	dom.window.close();
});

test("View projects configured column rulers through the margin coordinate system", () => {
	const dom = new JSDOM("<!doctype html><body><main></main></body>");
	const container = requiredElement(dom.window.document, "main");
	using model = new TextModel("abcdefghij");
	using viewport = new View({
		container,
		model,
		lineHeight: 20,
		textMeasurer: fixedTextMeasurer(8, 24),
		rulers: [{ column: 4 }, { column: 10, color: "red" }],
	});
	viewport.layout({ width: 200, height: 20 });

	const rulers = [...viewport.element.querySelectorAll<HTMLElement>(".stanza-editor-ruler")];
	assert.equal(rulers.length, 2);
	assert.deepEqual(rulers.map(ruler => ruler.style.left), ["88px", "136px"]);
	assert.equal(rulers[1]?.style.boxShadow, "1px 0 0 0 red inset");
	assert.equal(rulers[0]?.style.height, "20px");

	dom.window.close();
});

test("View uses browser range geometry for RTL selections and carets", () => {
	const dom = new JSDOM("<!doctype html><body><main></main></body>");
	const container = requiredElement(dom.window.document, "main");
	const createRange = dom.window.document.createRange.bind(dom.window.document);
	Object.defineProperty(dom.window.document, "createRange", {
		configurable: true,
		value: () => {
			const range = createRange();
			Object.defineProperty(range, "getClientRects", {
				configurable: true,
				value: () => range.collapsed
					? [testRectangle(135, 0, 0)]
					: [testRectangle(150, 0, 20), testRectangle(120, 0, 15)],
			});
			Object.defineProperty(range, "getBoundingClientRect", {
				configurable: true,
				value: () => testRectangle(135, 0, 0),
			});
			return range;
		},
	});
	using model = new TextModel("abc אבג");
	using selections = new CursorsController(model, SelectionSet.single(Selection.fromPositions(new Position((0) + 1, (0) + 1))));
	using viewport = new View({
		container,
		model,
		lineHeight: 20,
		textMeasurer: fixedTextMeasurer(),
		selectionController: selections,
		textDirection: EditorTextDirection.RightToLeft,
	});
	viewport.layout({ width: 300, height: 40 });
	const line = requiredLine(viewport.element, 0);
	Object.defineProperty(line, "getBoundingClientRect", {
		configurable: true,
		value: () => testRectangle(100, 0, 300),
	});
	selections.setSelections(SelectionSet.single(Selection.fromPositions(new Position((0) + 1, (0) + 1), new Position((0) + 1, (3) + 1))));

	const selectionRow = requiredPartRow(viewport.element, "stanza-editor-line-selections", 0);
	const selectionElements = [...selectionRow.querySelectorAll<HTMLElement>(".stanza-editor-selection")];
	assert.deepEqual(selectionElements.map(element => ({ left: element.style.left, width: element.style.width })), [
		{ left: "20px", width: "15px" },
		{ left: "50px", width: "20px" },
	]);
	assert.equal(requiredElement<HTMLElement>(viewport.element, ".stanza-editor-caret").style.left, "34px");
	assert.equal(viewport.getPositionContentCoordinates(new Position((0) + 1, (3) + 1)).left, 35);
	dom.window.close();
});

test("View selection geometry includes selected newlines on empty lines", () => {
	const dom = new JSDOM("<!doctype html><body><main></main></body>");
	const container = requiredElement(dom.window.document, "main");
	const createRange = dom.window.document.createRange.bind(dom.window.document);
	Object.defineProperty(dom.window.document, "createRange", {
		configurable: true,
		value: () => {
			const range = createRange();
			Object.defineProperty(range, "getClientRects", {
				configurable: true,
				value: () => range.collapsed
					? [testRectangle(130, 0, 0)]
					: [testRectangle(130, 0, 50)],
			});
			return range;
		},
	});
	using model = new TextModel("alpha\n\nomega");
	using selections = new CursorsController(model, SelectionSet.single(Selection.fromPositions(new Position((0) + 1, (0) + 1))));
	using viewport = new View({
		container,
		model,
		lineHeight: 20,
		textMeasurer: fixedTextMeasurer(10, 24),
		selectionController: selections,
	});
	viewport.layout({ width: 300, height: 60 });
	for (const line of lineElements(viewport.element)) {
		Object.defineProperty(line, "getBoundingClientRect", {
			configurable: true,
			value: () => testRectangle(100, 0, 300),
		});
	}
	const emptyLineSpan = lineText(requiredLine(viewport.element, 1)).firstElementChild;
	assert.ok(emptyLineSpan);
	Object.defineProperty(emptyLineSpan, "getClientRects", {
		configurable: true,
		value: () => [testRectangle(130, 0, 0)],
	});

	selections.setSelections(SelectionSet.single(Selection.fromPositions(new Position((0) + 1, (0) + 1), new Position((2) + 1, (0) + 1))));

	assert.deepEqual([...viewport.element.querySelectorAll<HTMLElement>(".stanza-editor-selection")].map(element => ({
		lineIndex: element.parentElement?.dataset.lineIndex,
		left: element.style.left,
		width: element.style.width,
	})), [
		{ lineIndex: "0", left: "30px", width: "60px" },
		{ lineIndex: "1", left: "30px", width: "10px" },
	]);
	dom.window.close();
});

test("View resolves RTL pointer hits from the browser caret position", () => {
	const dom = new JSDOM("<!doctype html><body><main></main></body>");
	const container = requiredElement(dom.window.document, "main");
	using model = new TextModel("abc אבג");
	using viewport = new View({
		container,
		model,
		lineHeight: 20,
		textMeasurer: fixedTextMeasurer(),
		textDirection: EditorTextDirection.RightToLeft,
	});
	viewport.layout({ width: 300, height: 40 });
	const text = lineText(requiredLine(viewport.element, 0));
	const textNode = text.firstElementChild?.firstChild;
	assert.ok(textNode);
	Object.defineProperty(dom.window.document, "caretPositionFromPoint", {
		configurable: true,
		value: () => ({ offsetNode: textNode, offset: 5 }),
	});

	assert.deepEqual(viewport.getTargetAtClientPoint({ clientX: 170, clientY: 10 }), {
		kind: "text",
		position: new Position((0) + 1, (5) + 1),
	});
	dom.window.close();
});

test("View announces cursor and selection changes through its live region", () => {
	const dom = new JSDOM("<!doctype html><body><main></main></body>");
	const container = requiredElement(dom.window.document, "main");
	using model = new TextModel("alpha\nbeta");
	using selections = new CursorsController(model, SelectionSet.single(Selection.fromPositions(new Position((0) + 1, (0) + 1))));
	using viewport = new View({ container, model, lineHeight: 20, textMeasurer: fixedTextMeasurer(), selectionController: selections });

	const status = requiredElement(viewport.element, ".stanza-editor-accessibility-status");
	assert.equal(status.getAttribute("aria-live"), "polite");
	assert.equal(status.textContent, "Line 1, column 1");
	selections.setSelections(SelectionSet.single(Selection.fromPositions(new Position((1) + 1, (1) + 1), new Position((1) + 1, (4) + 1))));
	assert.equal(status.textContent, "Line 2, column 5, 3 characters selected");
	selections.setSelections(SelectionSet.withPrimary([
		Selection.fromPositions(new Position((0) + 1, (2) + 1)),
		Selection.fromPositions(new Position((1) + 1, (0) + 1), new Position((1) + 1, (2) + 1)),
	], 1));
	assert.equal(status.textContent, "2 selections, 2 characters selected; primary at Line 2, column 3");
	dom.window.close();
});

test("View accepts explicit accessibility status announcements", () => {
	const dom = new JSDOM("<!doctype html><body><main></main></body>");
	const container = requiredElement(dom.window.document, "main");
	using model = new TextModel("alpha");
	using viewport = new View({ container, model, lineHeight: 20, textMeasurer: fixedTextMeasurer() });

	viewport.announceAccessibilityStatus("  Saved  ");
	assert.equal(requiredElement(viewport.element, ".stanza-editor-accessibility-status").textContent, "Saved");
	assert.throws(() => viewport.announceAccessibilityStatus("  "), /non-empty string/);
	dom.window.close();
});

test("View projects indentation guides for visible logical rows only", () => {
	const dom = new JSDOM("<!doctype html><body><main></main></body>");
	const container = requiredElement(dom.window.document, "main");
	using model = new TextModel("    alpha\n  beta");
	using viewport = new View({
		container,
		model,
		lineHeight: 20,
		textMeasurer: fixedTextMeasurer(10),
		indentation: { tabSize: 2 },
	});
	viewport.layout({ width: 300, height: 40 });
	const firstGuides = requiredPartRow(viewport.element, "stanza-editor-line-indent-guides", 0).querySelectorAll<HTMLElement>(".stanza-editor-indent-guide");
	assert.deepEqual([...firstGuides].map(guide => ({ level: guide.dataset.indentLevel, left: guide.style.left })), [
		{ level: "1", left: "77px" },
		{ level: "2", left: "97px" },
	]);
	assert.equal(requiredPartRow(viewport.element, "stanza-editor-line-indent-guides", 1).querySelectorAll(".stanza-editor-indent-guide").length, 1);
	dom.window.close();
});

test("View renders a canvas minimap and maps a primary click to document scroll", () => {
	const dom = new JSDOM("<!doctype html><body><main></main></body>");
	const container = requiredElement(dom.window.document, "main");
	using model = new TextModel(lines(200).join("\n"));
	using viewport = new View({
		container,
		model,
		lineHeight: 20,
		textMeasurer: fixedTextMeasurer(),
		minimap: { enabled: true },
	});
	viewport.layout({ width: 300, height: 100 });
	const minimap = requiredElement<HTMLElement>(viewport.element, ".stanza-editor-minimap");
	assert.equal(minimap.hidden, false);
	const canvas = requiredElement<HTMLCanvasElement>(minimap, ".stanza-editor-minimap-canvas");
	assert.equal(canvas.width, 32);
	assert.equal(canvas.height, 100);
	const overview = requiredElement<HTMLElement>(viewport.element, ".stanza-editor-overview-ruler");
	assert.equal(overview.style.left, "290px");

	viewport.element.scrollTop = 1950;
	viewport.element.dispatchEvent(new dom.window.Event("scroll"));
	assert.equal(viewport.viewportLayout.scrollPosition.top, 1950);
	assert.equal(minimap.style.transform, "translate3d(254px, 1950px, 0)");
	assert.equal(requiredElement<HTMLElement>(minimap, ".stanza-editor-minimap-slider").style.transform, "translate3d(0, 45px, 0)");

	minimap.dispatchEvent(new dom.window.MouseEvent("pointerdown", {
		bubbles: true,
		cancelable: true,
		button: 0,
		clientY: 75,
	}));
	assert.equal(viewport.viewportLayout.scrollPosition.top, 2210);
	assert.equal(requiredElement<HTMLElement>(minimap, ".stanza-editor-minimap-slider").style.transform, "translate3d(0, 51px, 0)");

	dom.window.document.dispatchEvent(new dom.window.MouseEvent("pointermove", {
		bubbles: true,
		cancelable: true,
		clientY: 100,
	}));
	assert.equal(viewport.viewportLayout.scrollPosition.top, 3900);
	assert.equal(minimap.classList.contains("dragging"), true);
	dom.window.document.dispatchEvent(new dom.window.MouseEvent("pointerup", { bubbles: true }));
	assert.equal(minimap.classList.contains("dragging"), false);
	dom.window.document.dispatchEvent(new dom.window.MouseEvent("pointermove", {
		bubbles: true,
		cancelable: true,
		clientY: 0,
	}));
	assert.equal(viewport.viewportLayout.scrollPosition.top, 3900);

	dom.window.close();
});

test("View keeps minimaps out of embedded presentations", () => {
	const dom = new JSDOM("<!doctype html><body><main></main></body>");
	const container = requiredElement(dom.window.document, "main");
	using model = new TextModel("alpha");
	using viewport = new View({
		container,
		model,
		lineHeight: 20,
		textMeasurer: fixedTextMeasurer(),
		presentation: "embedded",
	});
	assert.equal(requiredElement<HTMLElement>(viewport.element, ".stanza-editor-minimap").hidden, true);
	dom.window.close();
});

test("View lets a direct host own its focus outline and omits active lines by default when embedded", () => {
	const dom = new JSDOM("<!doctype html><body><main></main></body>");
	const container = requiredElement(dom.window.document, "main");
	using model = new TextModel("alpha");
	using selections = new CursorsController(model, SelectionSet.single(Selection.fromPositions(new Position((0) + 1, (0) + 1))));
	using embeddedViewport = new View({
		container,
		model,
		lineHeight: 20,
		textMeasurer: fixedTextMeasurer(),
		presentation: "embedded",
		focusOutlineOwner: "host",
		selectionController: selections,
	});
	embeddedViewport.layout({ width: 300, height: 40 });
	assert.equal(embeddedViewport.element.classList.contains("stanza-editor-focus-owner-host"), true);
	assert.equal(embeddedViewport.element.classList.contains("stanza-editor-focus-owner-editor"), false);
	assert.equal(embeddedViewport.element.querySelector(".view-line.active"), null);
	assert.ok(embeddedViewport.element.querySelector(".stanza-editor-caret"));
	assert.throws(() => new View({
		container,
		model,
		lineHeight: 20,
		textMeasurer: fixedTextMeasurer(),
		focusOutlineOwner: "unknown" as never,
	}), /Unknown Stanza editor focus outline owner/);
	assert.throws(() => new View({
		container,
		model,
		lineHeight: 20,
		textMeasurer: fixedTextMeasurer(),
		renderLineHighlight: "unknown" as never,
	}), /Unknown Stanza editor line highlight mode/);
	dom.window.close();
});

test("View normalizes minimap options through common editor configuration", () => {
	const dom = new JSDOM("<!doctype html><body><main></main></body>");
	const container = requiredElement(dom.window.document, "main");
	using model = new TextModel("alpha");
	using viewport = new View({
		container,
		model,
		lineHeight: 20,
		textMeasurer: fixedTextMeasurer(),
		minimap: { enabled: true, side: 'left', scale: 99 },
	});
	viewport.layout({ width: 300, height: 40 });
	const minimap = requiredElement<HTMLElement>(viewport.element, '.stanza-editor-minimap');
	assert.equal(minimap.style.transform, 'translate3d(0px, 0px, 0)');
	const canvas = requiredElement<HTMLCanvasElement>(minimap, '.stanza-editor-minimap-canvas');
	assert.equal(canvas.height, 40);
	assert.equal(requiredElement<HTMLElement>(viewport.element, '.stanza-editor-content').style.transform, `translate3d(${Number.parseFloat(canvas.style.width)}px, 0, 0)`);
	dom.window.close();
});

test("View uses the common proportional minimap size by default and preserves an explicit fill size", () => {
	const dom = new JSDOM("<!doctype html><body><main></main><aside></aside></body>");
	using model = new TextModel("alpha\nbeta");
	using proportionalViewport = new View({
		container: requiredElement(dom.window.document, "main"),
		model,
		lineHeight: 20,
		textMeasurer: fixedTextMeasurer(),
	});
	using fillingViewport = new View({
		container: requiredElement(dom.window.document, "aside"),
		model,
		lineHeight: 20,
		textMeasurer: fixedTextMeasurer(),
		minimap: { size: 'fill' },
	});
	proportionalViewport.layout({ width: 300, height: 100 });
	fillingViewport.layout({ width: 300, height: 100 });
	const proportionalCanvas = requiredElement<HTMLCanvasElement>(proportionalViewport.element, '.stanza-editor-minimap-canvas');
	const fillingCanvas = requiredElement<HTMLCanvasElement>(fillingViewport.element, '.stanza-editor-minimap-canvas');

	assert.equal(fillingCanvas.style.width, proportionalCanvas.style.width);
	assert.equal(fillingCanvas.width, proportionalCanvas.width * 2);
	dom.window.close();
});

test("View rejects an unknown GPU acceleration mode", () => {
	const dom = new JSDOM("<!doctype html><body><main></main></body>");
	const container = requiredElement(dom.window.document, "main");
	using model = new TextModel("alpha");
	assert.throws(() => new View({
		container,
		model,
		lineHeight: 20,
		textMeasurer: fixedTextMeasurer(),
		experimentalGpuAcceleration: "automatic" as never,
	}), /Unknown Stanza editor GPU acceleration mode/);
	dom.window.close();
});

test("View keeps DOM text visible when WebGPU initialization is unavailable", () => {
	const dom = new JSDOM("<!doctype html><body><main></main></body>");
	const container = requiredElement(dom.window.document, "main");
	using model = new TextModel("alpha");
	using viewport = new View({
		container,
		model,
		lineHeight: 20,
		textMeasurer: fixedTextMeasurer(),
		experimentalGpuAcceleration: "on",
		rulers: [{ column: 80 }],
	});
	viewport.layout({ width: 300, height: 40 });

	const canvas = requiredElement(viewport.element, ".stanza-editor-gpu-canvas") as HTMLCanvasElement;
	assert.equal(canvas.hidden, true);
	assert.ok(viewport.element.querySelector('.stanza-editor-gpu-mark-layer'));
	assert.equal(viewport.element.querySelector('.stanza-editor-rulers'), null);
	assert.equal(lineText(requiredLine(viewport.element, 0)).textContent, "alpha");
	assert.equal(requiredLine(viewport.element, 0).classList.contains("gpu-rendered"), false);
	dom.window.close();
});

test("Scrolling virtualizes rows while preserving overlapping DOM identity", () => {
	const dom = new JSDOM("<!doctype html><body><main></main></body>");
	const container = requiredElement(dom.window.document, "main");
	using model = new TextModel(lines(100).join("\n"));
	using viewport = new View({
		container,
		model,
		lineHeight: 20,
		overscanLineCount: 2,
		textMeasurer: fixedTextMeasurer(),
	});
	viewport.layout({ width: 300, height: 100 });

	viewport.scrollTo({ left: 0, top: 400 });
	const line20 = requiredLine(viewport.element, 20);
	assert.deepEqual(viewport.viewportLayout.renderLines, {
		startLineIndex: 18,
		endLineIndexExclusive: 27,
	});
	assert.equal(
		requiredElement(viewport.element, ".stanza-editor-lines").style.top,
		"360px",
	);
	assert.equal(requiredElement(viewport.element, ".stanza-editor-lines").style.transform, "");

	viewport.element.scrollTop = 420;
	viewport.element.dispatchEvent(new dom.window.Event("scroll"));

	assert.equal(viewport.viewportLayout.scrollPosition.top, 420);
	assert.deepEqual(viewport.viewportLayout.renderLines, {
		startLineIndex: 19,
		endLineIndexExclusive: 28,
	});
	assert.equal(requiredLine(viewport.element, 20), line20);
	assert.equal(viewport.element.querySelector('[data-line-index="18"]'), null);

	dom.window.close();
});

test("Soft wrapping virtualizes visual rows and maps DOM coordinates back to logical text", () => {
	const dom = new JSDOM("<!doctype html><body><main></main></body>");
	const container = requiredElement(dom.window.document, "main");
	using model = new TextModel("abcdef\ngh");
	using viewport = new View({
		container,
		model,
		lineHeight: 20,
		textMeasurer: fixedTextMeasurer(10, 24),
		lineWrapping: EditorLineWrapping.On,
		minimap: { enabled: false },
	});

	viewport.layout({ width: 90, height: 40 });

	assert.equal(viewport.viewportLayout.contentSize.height, 80);
	assert.equal(viewport.viewportLayout.maximumScrollPosition.left, 0);
	assert.deepEqual(
		lineElements(viewport.element).map(line => ({
			lineIndex: line.dataset.lineIndex,
			logicalLineIndex: line.dataset.logicalLineIndex,
			number: lineNumber(line).textContent,
			text: lineText(line).textContent,
		})),
		[{
			lineIndex: "0",
			logicalLineIndex: "0",
			number: "1",
			text: "ab",
		}, {
			lineIndex: "1",
			logicalLineIndex: "0",
			number: "",
			text: "cd",
		}, {
			lineIndex: "2",
			logicalLineIndex: "0",
			number: "",
			text: "ef",
		}, {
			lineIndex: "3",
			logicalLineIndex: "1",
			number: "2",
			text: "gh",
		}],
	);
	assert.deepEqual(
		viewport.getPositionContentCoordinates(new Position((0) + 1, (3) + 1)),
		{ left: 68, top: 20, height: 20 },
	);
	assert.deepEqual(viewport.getTargetAtClientPoint({ clientX: 70, clientY: 25 }), {
		kind: "text",
		position: new Position((0) + 1, (3) + 1),
	});

	viewport.layout({ width: 110, height: 40 });

	assert.equal(viewport.viewportLayout.contentSize.height, 60);
	assert.deepEqual(
		lineElements(viewport.element).map(line => lineText(line).textContent),
		["abcd", "ef", "gh"],
	);

	dom.window.close();
});

test("Soft wrapping applies the configured indent to continuation DOM rows", () => {
	const dom = new JSDOM("<!doctype html><body><main></main></body>");
	const container = requiredElement(dom.window.document, "main");
	using model = new TextModel("  abcdefgh");
	using viewport = new View({
		container,
		model,
		lineHeight: 20,
		textMeasurer: fixedTextMeasurer(10, 24),
		lineWrapping: EditorLineWrapping.On,
		wrappingIndent: WrappingIndent.Same,
		minimap: { enabled: false },
	});

	viewport.layout({ width: 116, height: 60 });

	const rendered = lineElements(viewport.element);
	assert.equal(rendered[0]?.querySelector<HTMLSpanElement>(".stanza-editor-line-text")?.style.marginInlineStart, "0px");
	assert.equal(rendered[1]?.querySelector<HTMLSpanElement>(".stanza-editor-line-text")?.style.marginInlineStart, "20px");

	dom.window.close();
});

test("Folding model removes folded physical rows from the viewport projection", () => {
	const dom = new JSDOM("<!doctype html><body><main></main></body>");
	const container = requiredElement(dom.window.document, "main");
	using model = new TextModel("header\nbody\nend\nafter");
	using folding = new EditorFoldingModel(model);
	using hiddenRanges = new EditorHiddenRangeModel(model, folding);
	folding.setRanges([{ startLineIndex: 0, endLineIndex: 2 }]);
	using decorations = new EditorFoldingDecorationSource(folding);
	using viewport = new View({
		container,
		model,
		lineHeight: 20,
		textMeasurer: fixedTextMeasurer(),
		lineVisibilitySource: hiddenRanges,
		decorationSources: [decorations],
	});
	viewport.layout({ width: 300, height: 20 });
	assert.equal(viewport.element.style.getPropertyValue("--stanza-editor-line-numbers-width"), "24px");
	assert.equal(viewport.element.style.getPropertyValue("--stanza-editor-glyph-margin-width"), "20px");
	assert.equal(viewport.element.style.getPropertyValue("--stanza-editor-line-numbers-left"), "20px");
	assert.equal(viewport.element.style.getPropertyValue("--stanza-editor-line-decorations-left"), "44px");
	assert.equal(viewport.element.style.getPropertyValue("--stanza-editor-line-decorations-width"), "20px");
	assert.equal(viewport.element.style.getPropertyValue("--stanza-editor-gutter-width"), "64px");
	const initialToggle = requiredElement<HTMLButtonElement>(viewport.element, ".stanza-editor-fold-toggle");
	assert.equal(initialToggle.getAttribute("aria-expanded"), "true");
	assert.equal(initialToggle.textContent, "");
	assert.equal(initialToggle.querySelectorAll("svg").length, 1);
	assert.equal(initialToggle.dataset.iconId, "folding-expanded");
	folding.setContainingLineCollapsed(0, true);

	assert.equal(viewport.viewportLayout.contentSize.height, 40);
	assert.deepEqual(lineElements(viewport.element).map(line => ({
		logicalLineIndex: line.dataset.logicalLineIndex,
		number: lineNumber(line).textContent,
		text: lineText(line).textContent,
	})), [{
		logicalLineIndex: "0",
		number: "1",
		text: "header",
	}, {
		logicalLineIndex: "3",
		number: "4",
		text: "after",
	}]);
	assert.deepEqual(viewport.getPositionContentCoordinates(new Position((1) + 1, (0) + 1)), {
		left: 76,
		top: 0,
		height: 20,
	});
	const collapsedToggle = requiredElement<HTMLButtonElement>(viewport.element, ".stanza-editor-fold-toggle");
	assert.equal(collapsedToggle.getAttribute("aria-expanded"), "false");
	assert.equal(collapsedToggle.querySelectorAll("svg").length, 1);
	assert.equal(collapsedToggle.dataset.iconId, "folding-collapsed");

	dom.window.close();
});

test("View keeps short minimap content at the top and projects a proportional hover slider", () => {
	const dom = new JSDOM("<!doctype html><body><main></main></body>");
	const container = requiredElement(dom.window.document, "main");
	using model = new TextModel("alpha\nbeta\ngamma");
	using viewport = new View({
		container,
		model,
		lineHeight: 20,
		textMeasurer: fixedTextMeasurer(),
		minimap: { enabled: true },
	});
	viewport.layout({ width: 300, height: 100 });
	const minimap = requiredElement<HTMLElement>(viewport.element, ".stanza-editor-minimap");
	const canvas = requiredElement<HTMLCanvasElement>(minimap, '.stanza-editor-minimap-canvas');
	assert.equal(canvas.height, 100);
	const slider = requiredElement<HTMLElement>(minimap, ".stanza-editor-minimap-slider");
	assert.equal(slider.hidden, false);
	assert.equal(slider.style.height, '10px');
	assert.equal(minimap.style.transform, "translate3d(252px, 0px, 0)");
	dom.window.close();
});

test("Editor gutter orders generic glyphs, line numbers, folding controls, then content", () => {
	const dom = new JSDOM("<!doctype html><body><main></main></body>");
	const container = requiredElement(dom.window.document, "main");
	using model = new TextModel("header\nbody\nend");
	using glyphs = new TextDecorationCollection<string>(model);
	glyphs.add({
		range: Range.fromPositions(new Position((0) + 1, (0) + 1)),
		stickiness: TrackedRangeStickiness.NeverGrowsWhenTypingAtEdges,
		metadata: "generic",
	});
	const glyphSource = createStanzaDecorationSource(glyphs, () => ({
		presentation: DecorationPresentation.GlyphMargin,
		glyphMargin: {
			owner: "test-glyph",
			lane: GlyphMarginLane.Left,
			ariaLabel: "Generic gutter marker",
		},
	}), undefined, {
		glyphMarginLanes: [{ owner: "test-glyph", lane: GlyphMarginLane.Left }],
	});
	using folding = new EditorFoldingModel(model);
	folding.setRanges([{ startLineIndex: 0, endLineIndex: 2 }]);
	using foldingDecorations = new EditorFoldingDecorationSource(folding);
	using viewport = new View({
		container,
		model,
		lineHeight: 20,
		textMeasurer: fixedTextMeasurer(),
		decorationSources: [glyphSource, foldingDecorations],
	});

	viewport.layout({ width: 300, height: 60 });

	assert.equal(viewport.element.style.getPropertyValue("--stanza-editor-glyph-margin-width"), "20px");
	assert.equal(viewport.element.style.getPropertyValue("--stanza-editor-line-numbers-left"), "20px");
	assert.equal(viewport.element.style.getPropertyValue("--stanza-editor-line-numbers-width"), "24px");
	assert.equal(viewport.element.style.getPropertyValue("--stanza-editor-line-decorations-left"), "44px");
	assert.equal(viewport.element.style.getPropertyValue("--stanza-editor-line-decorations-width"), "20px");
	assert.equal(viewport.element.style.getPropertyValue("--stanza-editor-gutter-width"), "64px");
	assert.equal(lineNumber(requiredLine(viewport.element, 0)).textContent, "1");
	assert.equal(requiredElement<HTMLElement>(viewport.element, ".stanza-editor-glyph-margin").style.left, "0px");
	assert.equal(requiredElement<HTMLElement>(viewport.element, ".stanza-editor-fold-toggle").dataset.decorationOwner, "folding");
	assert.equal(viewport.element.querySelector(".stanza-editor-decoration.line-decoration"), null);
	assert.deepEqual(viewport.getPositionContentCoordinates(new Position((0) + 1, (0) + 1)), {
		left: 76,
		top: 0,
		height: 20,
	});

	dom.window.close();
});

test("Editor gutter can disable the glyph margin without changing remaining column order", () => {
	const dom = new JSDOM("<!doctype html><body><main></main></body>");
	const container = requiredElement(dom.window.document, "main");
	using model = new TextModel("header");
	using viewport = new View({
		container,
		model,
		lineHeight: 20,
		textMeasurer: fixedTextMeasurer(),
		glyphMargin: false,
	});

	viewport.layout({ width: 300, height: 20 });

	assert.equal(viewport.element.style.getPropertyValue("--stanza-editor-glyph-margin-width"), "0px");
	assert.equal(viewport.element.style.getPropertyValue("--stanza-editor-line-numbers-left"), "0px");
	assert.equal(viewport.element.style.getPropertyValue("--stanza-editor-line-decorations-left"), "24px");
	assert.equal(viewport.element.style.getPropertyValue("--stanza-editor-gutter-width"), "24px");
	assert.equal(requiredElement<HTMLElement>(viewport.element, ".stanza-editor-glyph-margin").hidden, true);

	dom.window.close();
});

test("Model edits refresh visible rows and clamp a shrinking document", () => {
	const dom = new JSDOM("<!doctype html><body><main></main></body>");
	const container = requiredElement(dom.window.document, "main");
	using model = new TextModel(lines(100).join("\n"));
	using viewport = new View({
		container,
		model,
		lineHeight: 20,
		overscanLineCount: 2,
		textMeasurer: fixedTextMeasurer(),
	});
	viewport.layout({ width: 300, height: 100 });
	viewport.scrollTo({ left: 0, top: 400 });
	const line20 = requiredLine(viewport.element, 20);

	model.applyEdits([{
		range: Range.fromPositions(
			new Position((20) + 1, (0) + 1),
			new Position((20) + 1, (model.getLineContent((20) + 1).length) + 1),
		),
		text: "changed line",
	}]);

	assert.equal(requiredLine(viewport.element, 20), line20);
	assert.equal(lineText(line20).textContent, "changed line");
	assert.equal(viewport.viewportLayout.modelVersion, 2);

	const snapshot = model.createVersionedSnapshot();
	model.applyEdits([{
		range: Range.fromPositions(model.positionAt(0), model.positionAt(snapshot.length)),
		text: "first\nsecond",
	}]);

	assert.equal(viewport.element.scrollTop, 0);
	assert.equal(viewport.viewportLayout.scrollPosition.top, 0);
	assert.equal(viewport.viewportLayout.contentSize.height, 100);
	assert.deepEqual(
		lineElements(viewport.element).map(line => lineText(line).textContent),
		["first", "second"],
	);

	dom.window.close();
});

test("Selection controller projects gutter state, ranges, and carets", () => {
	const dom = new JSDOM("<!doctype html><body><main></main></body>");
	const container = requiredElement(dom.window.document, "main");
	using model = new TextModel("abcd\nefgh\nij");
	using controller = new CursorsController(
		model,
		SelectionSet.withPrimary([
			Selection.fromPositions(
				new Position((1) + 1, (3) + 1),
				new Position((0) + 1, (1) + 1),
			),
			Selection.fromPositions(new Position((2) + 1, (1) + 1)),
		], 0),
	);
	using viewport = new View({
		container,
		model,
		lineHeight: 20,
		textMeasurer: fixedTextMeasurer(10, 24),
		selectionController: controller,
	});
	viewport.layout({ width: 200, height: 60 });

	const selectionElements = [
		...viewport.element.querySelectorAll<HTMLElement>(
			".stanza-editor-selection",
		),
	];
	const caretElements = [
		...viewport.element.querySelectorAll<HTMLElement>(
			".stanza-editor-caret",
		),
	];
	assert.deepEqual(
		selectionElements.map(element => ({
			lineIndex: element.parentElement?.dataset.lineIndex,
			left: element.style.left,
			width: element.style.width,
		})),
		[{
			lineIndex: "0",
			left: "68px",
			width: "40px",
		}, {
			lineIndex: "1",
			left: "58px",
			width: "30px",
		}],
	);
	assert.equal(selectionElements.every(element => element.parentElement?.classList.contains("stanza-editor-line-selections")), true);
	assert.equal(caretElements.length, 2);
	assert.equal(caretElements.every(element => element.parentElement?.classList.contains("stanza-editor-cursors-layer")), true);
	assert.equal(caretElements[0]?.classList.contains("cursor-primary"), true);
	assert.equal(caretElements[0]?.style.left, "67px");
	assert.equal(
		lineNumber(requiredLine(viewport.element, 0))
			.classList.contains("active"),
		true,
	);

	controller.setSelections(SelectionSet.single(
		Selection.fromPositions(new Position((1) + 1, (2) + 1)),
	));

	assert.equal(
		viewport.element.querySelectorAll(
			".stanza-editor-selection",
		).length,
		0,
	);
	assert.equal(
		viewport.element.querySelector<HTMLElement>('.stanza-editor-caret[data-selection-index="0"]')?.style.left,
		"77px",
	);
	assert.equal(
		lineNumber(requiredLine(viewport.element, 1))
			.classList.contains("active"),
		true,
	);
	assert.equal(requiredPartRow(viewport.element, "stanza-editor-current-line-highlight", 1).classList.contains("active"), true);

	model.applyEdits([{
		range: Range.fromPositions(new Position((0) + 1, (0) + 1)),
		text: "X\n",
	}]);

	assert.equal(controller.selections.primary.getPosition().lineNumber, 3);
	assert.equal(
		viewport.element.querySelector<HTMLElement>('.stanza-editor-caret[data-selection-index="0"]')?.style.left,
		"77px",
	);
	assert.equal(
		lineNumber(requiredLine(viewport.element, 2)).textContent,
		"3",
	);
	assert.equal(
		lineNumber(requiredLine(viewport.element, 2))
			.classList.contains("active"),
		true,
	);

	dom.window.close();
});

test('View projects the configured mouse style onto its text layer', () => {
	const dom = new JSDOM('<!doctype html><body><main></main></body>');
	const container = requiredElement(dom.window.document, 'main');
	using model = new TextModel('alpha');
	using viewport = new View({
		container,
		model,
		lineHeight: 20,
		textMeasurer: fixedTextMeasurer(),
		mouseStyle: 'copy',
	});

	assert.equal(viewport.element.classList.contains('stanza-editor-mouse-copy'), true);
	dom.window.close();
});

test('View places RTL block cursors over their following glyph or trailing space', () => {
	const dom = new JSDOM('<!doctype html><body><main></main></body>');
	const container = requiredElement(dom.window.document, 'main');
	const createRange = dom.window.document.createRange.bind(dom.window.document);
	Object.defineProperty(dom.window.document, 'createRange', {
		configurable: true,
		value: () => {
			const range = createRange();
			Object.defineProperty(range, 'getClientRects', {
				configurable: true,
				value: () => range.collapsed
					? [testRectangle(range.startOffset === 0 ? 128 : 120, 0, 0)]
					: [testRectangle(120, 0, 8)],
			});
			return range;
		},
	});
	using model = new TextModel('讗');
	using selections = new CursorsController(model, SelectionSet.single(Selection.fromPositions(new Position((0) + 1, (1) + 1))));
	using viewport = new View({
		container,
		model,
		lineHeight: 20,
		textMeasurer: fixedTextMeasurer(),
		selectionController: selections,
		textDirection: EditorTextDirection.RightToLeft,
		cursorStyle: 'block',
		cursorBlinking: 'solid',
	});
	viewport.layout({ width: 300, height: 40 });
	const line = requiredLine(viewport.element, 0);
	Object.defineProperty(line, 'getBoundingClientRect', {
		configurable: true,
		value: () => testRectangle(100, 0, 300),
	});
	const caret = requiredElement<HTMLElement>(viewport.element, '.stanza-editor-caret');
	assert.equal(caret.getAttribute('aria-hidden'), 'true');
	selections.setSelections(SelectionSet.single(Selection.fromPositions(new Position((0) + 1, (0) + 1))));
	const overGlyph = { left: caret.style.left, width: caret.style.width, text: caret.textContent };
	selections.setSelections(SelectionSet.single(Selection.fromPositions(new Position((0) + 1, (1) + 1))));

	assert.deepEqual({ overGlyph, afterLine: { left: caret.style.left, width: caret.style.width, text: caret.textContent } }, {
		overGlyph: { left: '20px', width: '8px', text: '讗' },
		afterLine: { left: '12px', width: '8px', text: '\u00a0' },
	});
	dom.window.close();
});

test('View renders active bracket and indentation guides from the structural bracket source', () => {
	const dom = new JSDOM('<!doctype html><body><main></main></body>');
	const container = requiredElement(dom.window.document, 'main');
	using model = new TextModel('{\n    value\n}\ntail');
	using selections = new CursorsController(model, SelectionSet.single(Selection.fromPositions(new Position((1) + 1, (4) + 1))));
	const bracketSource: BracketColorizationSource = {
		textModel: model,
		getLineBrackets: () => Object.freeze([]),
		getBracketGuides: () => Object.freeze([Object.freeze({
			opening: Range.fromPositions(new Position((0) + 1, (0) + 1), new Position((0) + 1, (1) + 1)),
			closing: Range.fromPositions(new Position((2) + 1, (0) + 1), new Position((2) + 1, (1) + 1)),
			level: 1,
		})]),
	};
	using viewport = new View({
		container,
		model,
		selectionController: selections,
		bracketColorizationSource: bracketSource,
		lineHeight: 20,
		textMeasurer: fixedTextMeasurer(8, 24),
		guides: {
			bracketPairs: 'active',
			bracketPairsHorizontal: 'active',
			highlightActiveBracketPair: true,
			indentation: true,
			highlightActiveIndentation: 'always',
		},
	});
	viewport.layout({ width: 200, height: 80 });

	assert.equal(viewport.element.querySelectorAll('.stanza-editor-bracket-guide').length, 3);
	assert.equal(viewport.element.querySelectorAll('.stanza-editor-bracket-guide.active').length, 3);
	assert.equal(viewport.element.querySelectorAll('.stanza-editor-bracket-guide-horizontal.active').length, 1);
	assert.equal(viewport.element.querySelectorAll('.stanza-editor-indent-guide.active').length, 1);
	dom.window.close();
});

test('View preserves line, gutter, focus, and multi-cursor highlight semantics', () => {
	const dom = new JSDOM('<!doctype html><body><main></main></body>');
	const container = requiredElement(dom.window.document, 'main');
	using model = new TextModel('alpha\nbeta\ngamma');
	using controller = new CursorsController(model, SelectionSet.withPrimary([
		Selection.fromPositions(new Position((0) + 1, (0) + 1), new Position((0) + 1, (2) + 1)),
		Selection.fromPositions(new Position((2) + 1, (1) + 1)),
	], 1));
	using viewport = new View({
		container,
		model,
		lineHeight: 20,
		textMeasurer: fixedTextMeasurer(),
		selectionController: controller,
		renderLineHighlight: 'all',
		renderLineHighlightOnlyWhenFocus: true,
		cursorStyle: 'block-outline',
		cursorBlinking: 'solid',
		cursorWidth: 7,
		cursorHeight: 12,
	});
	viewport.layout({ width: 300, height: 60 });
	const primaryCaret = requiredElement<HTMLElement>(viewport.element, '.stanza-editor-caret.cursor-primary');
	assert.equal(primaryCaret.classList.contains('cursor-style-block-outline'), true);
	assert.equal(requiredElement(viewport.element, '.stanza-editor-cursors-layer').classList.contains('cursor-blinking-solid'), true);
	assert.equal(primaryCaret.style.width, '8px');
	assert.equal(primaryCaret.style.height, '20px');
	assert.equal(primaryCaret.style.top, '40px');

	for (const lineIndex of [0, 2]) {
		const row = requiredPartRow(viewport.element, 'stanza-editor-current-line-highlight', lineIndex);
		assert.equal(row.classList.contains('active'), true);
		assert.equal(row.classList.contains('highlight-gutter'), true);
		assert.equal(row.classList.contains('highlight-line'), false);
		assert.equal(row.classList.contains('focus-only'), true);
	}

	controller.setSelections(SelectionSet.withPrimary([
		Selection.fromPositions(new Position((0) + 1, (2) + 1)),
		Selection.fromPositions(new Position((2) + 1, (1) + 1)),
	], 1));
	assert.equal(requiredPartRow(viewport.element, 'stanza-editor-current-line-highlight', 0).classList.contains('highlight-line'), true);
	assert.equal(requiredPartRow(viewport.element, 'stanza-editor-current-line-highlight', 2).classList.contains('highlight-line'), true);
	dom.window.close();
});

test('View matches line and thin-underline cursor geometry', () => {
	const dom = new JSDOM('<!doctype html><body><main></main></body>');
	const container = requiredElement(dom.window.document, 'main');
	using model = new TextModel('abc');
	using controller = new CursorsController(model, SelectionSet.single(Selection.fromPositions(new Position((0) + 1, (1) + 1))));
	using viewport = new View({
		container,
		model,
		lineHeight: 20,
		textMeasurer: fixedTextMeasurer(10, 24),
		selectionController: controller,
		cursorStyle: 'line',
		overtypeCursorStyle: 'underline-thin',
		cursorBlinking: 'solid',
		cursorWidth: 6,
		cursorHeight: 12,
	});
	viewport.layout({ width: 200, height: 40 });
	const caret = requiredElement<HTMLElement>(viewport.element, '.stanza-editor-caret');
	const line = {
		left: caret.style.left,
		paddingLeft: caret.style.paddingLeft,
		width: caret.style.width,
		height: caret.style.height,
		top: caret.style.top,
		text: caret.textContent,
	};

	viewport.setOvertype(true);

	assert.deepEqual({
		line,
		underlineThin: {
			left: caret.style.left,
			paddingLeft: caret.style.paddingLeft,
			width: caret.style.width,
			height: caret.style.height,
			top: caret.style.top,
			text: caret.textContent,
		},
	}, {
		line: { left: '67px', paddingLeft: '1px', width: '6px', height: '12px', top: '4px', text: 'b' },
		underlineThin: { left: '68px', paddingLeft: '0px', width: '10px', height: '1px', top: '19px', text: '' },
	});
	dom.window.close();
});

test('View normalizes a block cursor to the complete containing grapheme', () => {
	const dom = new JSDOM('<!doctype html><body><main></main></body>');
	const container = requiredElement(dom.window.document, 'main');
	using model = new TextModel('a😊b');
	using controller = new CursorsController(model, SelectionSet.single(Selection.fromPositions(new Position((0) + 1, (2) + 1))));
	using viewport = new View({
		container,
		model,
		selectionController: controller,
		lineHeight: 20,
		textMeasurer: fixedTextMeasurer(),
		cursorStyle: 'block',
		cursorBlinking: 'solid',
	});
	viewport.layout({ width: 200, height: 40 });
	const caret = requiredElement<HTMLElement>(viewport.element, '.stanza-editor-caret');
	const insideGrapheme = { left: caret.style.left, width: caret.style.width, text: caret.textContent };
	controller.setSelections(SelectionSet.single(Selection.fromPositions(new Position((0) + 1, (1) + 1))));

	assert.deepEqual({ insideGrapheme, atGraphemeStart: { left: caret.style.left, width: caret.style.width, text: caret.textContent } }, {
		insideGrapheme: { left: '64px', width: '8px', text: '😊' },
		atGraphemeStart: { left: '64px', width: '8px', text: '😊' },
	});
	dom.window.close();
});

test('View gives every cursor one shared blinking animation', () => {
	const dom = new JSDOM('<!doctype html><body><main></main></body>');
	const container = requiredElement(dom.window.document, 'main');
	using model = new TextModel('alpha;\nbeta;');
	using controller = new CursorsController(model, SelectionSet.single(Selection.fromPositions(new Position((0) + 1, (5) + 1))));
	using viewport = new View({
		container,
		model,
		selectionController: controller,
		lineHeight: 20,
		textMeasurer: fixedTextMeasurer(),
		cursorBlinking: 'expand',
	});
	viewport.layout({ width: 300, height: 40 });
	const cursorsLayer = requiredElement(viewport.element, '.stanza-editor-cursors-layer');
	const blinkingAnimation = { currentTime: 600 };
	Object.defineProperty(cursorsLayer, 'getAnimations', { value: () => [blinkingAnimation] });
	controller.setSelections(SelectionSet.withPrimary([
		Selection.fromPositions(new Position((0) + 1, (5) + 1)),
		Selection.fromPositions(new Position((1) + 1, (4) + 1)),
	], 1));

	assert.deepEqual({
		layerExpands: cursorsLayer.classList.contains('cursor-blinking-expand'),
		animationTime: blinkingAnimation.currentTime,
		cursorAnimationClasses: [...viewport.element.querySelectorAll('.stanza-editor-caret')]
			.map(caret => [...caret.classList].filter(className => className.startsWith('cursor-blinking-'))),
	}, {
		layerExpands: true,
		animationTime: 0,
		cursorAnimationClasses: [[], []],
	});
	dom.window.close();
});

test('View animates stable explicit cursor movement and pauses cursor-count changes', () => {
	const dom = new JSDOM('<!doctype html><body><main></main></body>');
	const container = requiredElement(dom.window.document, 'main');
	using model = new TextModel('alpha\nbeta');
	using controller = new CursorsController(model, SelectionSet.single(Selection.fromPositions(new Position((0) + 1, (0) + 1))));
	using viewport = new View({
		container,
		model,
		selectionController: controller,
		lineHeight: 20,
		textMeasurer: fixedTextMeasurer(),
		cursorBlinking: 'solid',
		cursorSmoothCaretAnimation: 'explicit',
	});
	viewport.layout({ width: 200, height: 40 });
	const layer = requiredElement<HTMLElement>(viewport.element, '.stanza-editor-cursors-layer');
	const firstCaret = requiredElement<HTMLElement>(viewport.element, '.stanza-editor-caret');
	const initial = { top: firstCaret.style.top, transitionProperty: firstCaret.style.transitionProperty };

	controller.setCursorSelections(SelectionSet.single(Selection.fromPositions(new Position((1) + 1, (0) + 1))));
	const moved = { top: firstCaret.style.top, transitionProperty: firstCaret.style.transitionProperty };
	controller.setCursorSelections(SelectionSet.withPrimary([
		Selection.fromPositions(new Position((0) + 1, (0) + 1)),
		Selection.fromPositions(new Position((1) + 1, (0) + 1)),
	], 1));

	assert.deepEqual({
		smoothClass: layer.classList.contains('cursor-smooth-caret-animation'),
		initial,
		moved,
		multiple: [...viewport.element.querySelectorAll<HTMLElement>('.stanza-editor-caret')].map(caret => ({
			plurality: caret.classList.contains('cursor-primary') ? 'primary' : 'secondary',
			transitionProperty: caret.style.transitionProperty,
		})),
	}, {
		smoothClass: true,
		initial: { top: '0px', transitionProperty: 'none' },
		moved: { top: '20px', transitionProperty: '' },
		multiple: [
			{ plurality: 'secondary', transitionProperty: 'none' },
			{ plurality: 'primary', transitionProperty: 'none' },
		],
	});
	dom.window.close();
});

test('View snaps line cursors to physical pixels', () => {
	const dom = new JSDOM('<!doctype html><body><main></main></body>');
	Object.defineProperty(dom.window, 'devicePixelRatio', { configurable: true, value: 1.25 });
	const container = requiredElement(dom.window.document, 'main');
	using model = new TextModel('abc');
	using controller = new CursorsController(model, SelectionSet.single(Selection.fromPositions(new Position((0) + 1, (1) + 1))));
	using viewport = new View({
		container,
		model,
		selectionController: controller,
		lineHeight: 20,
		textMeasurer: fixedTextMeasurer(10, 24),
		cursorStyle: 'line',
		cursorBlinking: 'solid',
		cursorWidth: 2,
	});
	viewport.layout({ width: 200, height: 40 });
	const caret = requiredElement<HTMLElement>(viewport.element, '.stanza-editor-caret');

	assert.deepEqual({ left: caret.style.left, paddingLeft: caret.style.paddingLeft, width: caret.style.width }, {
		left: '68px',
		paddingLeft: '0px',
		width: '1.6px',
	});
	dom.window.close();
});

test('View keeps token font styling on characters redrawn inside cursors', () => {
	const dom = new JSDOM('<!doctype html><body><main></main></body>');
	const container = requiredElement(dom.window.document, 'main');
	using model = new TextModel('alpha');
	using tokenChanges = new Emitter<void>();
	let tokens: readonly ResolvedSemanticToken[] = Object.freeze([Object.freeze({
		startColumn: 0,
		endColumn: 5,
		presentation: SemanticTokenPresentation.Keyword,
		modifiers: Object.freeze([SemanticTokenModifier.Declaration, SemanticTokenModifier.Readonly]),
		syntaxPresentation: Object.freeze({ fontStyle: Object.freeze(['italic', 'bold', 'underline'] as const) }),
	})]);
	const semanticTokenSource: SemanticTokenSource = {
		textModel: model,
		onDidChange: tokenChanges.event,
		get lines() { return Object.freeze([Object.freeze({ lineIndex: 0, tokens })]); },
		getLineTokens: () => tokens,
	};
	using controller = new CursorsController(model, SelectionSet.single(Selection.fromPositions(new Position((0) + 1, (1) + 1))));
	using viewport = new View({
		container,
		model,
		selectionController: controller,
		semanticTokenSource,
		lineHeight: 20,
		textMeasurer: fixedTextMeasurer(),
		cursorStyle: 'block',
		cursorBlinking: 'solid',
	});
	viewport.layout({ width: 200, height: 40 });
	const caret = requiredElement<HTMLElement>(viewport.element, '.stanza-editor-caret');
	const styled = {
		classes: [...caret.classList].filter(className => className.startsWith('token-')),
		fontStyle: caret.style.fontStyle,
		fontWeight: caret.style.fontWeight,
		textDecorationLine: caret.style.textDecorationLine,
	};

	tokens = Object.freeze([]);
	tokenChanges.fire();

	assert.deepEqual({
		styled,
		cleared: {
			classes: [...caret.classList].filter(className => className.startsWith('token-')),
			fontStyle: caret.style.fontStyle,
			fontWeight: caret.style.fontWeight,
			textDecorationLine: caret.style.textDecorationLine,
		},
	}, {
		styled: {
			classes: ['token-keyword', 'token-modifier-declaration', 'token-modifier-readonly'],
			fontStyle: 'italic',
			fontWeight: 'bold',
			textDecorationLine: 'underline',
		},
		cleared: { classes: [], fontStyle: '', fontWeight: '', textDecorationLine: '' },
	});
	dom.window.close();
});

test('View sizes block cursors from contextual tab advances', () => {
	const dom = new JSDOM('<!doctype html><body><main></main></body>');
	const container = requiredElement(dom.window.document, 'main');
	using model = new TextModel('a\tb\nabcd\tb');
	using controller = new CursorsController(model, SelectionSet.withPrimary([
		Selection.fromPositions(new Position((0) + 1, (1) + 1)),
		Selection.fromPositions(new Position((1) + 1, (4) + 1)),
	], 0));
	using viewport = new View({
		container,
		model,
		selectionController: controller,
		lineHeight: 20,
		textMeasurer: tabTextMeasurer(),
		cursorStyle: 'block',
		cursorBlinking: 'solid',
	});
	viewport.layout({ width: 300, height: 40 });

	assert.deepEqual([...viewport.element.querySelectorAll<HTMLElement>('.stanza-editor-caret')]
		.map(caret => caret.style.width), ['24px', '32px']);
	dom.window.close();
});

test("Measured content width, line height, and scroll stay synchronized", () => {
	const dom = new JSDOM("<!doctype html><body><main></main></body>");
	const container = requiredElement(dom.window.document, "main");
	using model = new TextModel([
		"x".repeat(458),
		...lines(29),
	].join("\n"));
	using viewport = new View({
		container,
		model,
		lineHeight: 20,
		textMeasurer: fixedTextMeasurer(1, 24),
	});

	viewport.layout({ width: 200, height: 100 });
	viewport.scrollTo({ left: 1_000, top: 200 });
	viewport.setLineHeight(40);

	assert.equal(viewport.element.style.getPropertyValue("--stanza-editor-glyph-margin-width"), "40px");
	assert.deepEqual(viewport.viewportLayout.scrollPosition, {
		left: 320,
		top: 400,
	});
	assert.equal(viewport.element.scrollLeft, 320);
	assert.equal(viewport.element.scrollTop, 400);
	assert.equal(
		requiredElement(viewport.element, ".stanza-editor-content").style.width,
		"540px",
	);
	for (const row of lineElements(viewport.element)) {
		assert.equal(row.style.height, "40px");
		assert.equal(row.style.lineHeight, "40px");
	}

	dom.window.close();
});

test("Line width indexing updates only affected model line groups", () => {
	const dom = new JSDOM("<!doctype html><body><main></main></body>");
	const container = requiredElement(dom.window.document, "main");
	using model = new TextModel("abcdef\nxx");
	using viewport = new View({
		container,
		model,
		lineHeight: 20,
		textMeasurer: fixedTextMeasurer(10, 24),
	});
	viewport.layout({ width: 50, height: 40 });
	viewport.scrollTo({ left: 1_000, top: 0 });
	assert.equal(viewport.viewportLayout.contentSize.width, 130);
	assert.equal(viewport.element.scrollLeft, 80);

	model.applyEdits([{
		range: Range.fromPositions(new Position((0) + 1, (0) + 1), new Position((0) + 1, (1) + 1)),
		text: "",
	}, {
		range: Range.fromPositions(new Position((0) + 1, (4) + 1), new Position((0) + 1, (5) + 1)),
		text: "",
	}]);

	assert.equal(model.getLineContent((0) + 1), "bcdf");
	assert.equal(viewport.viewportLayout.contentSize.width, 110);
	assert.equal(viewport.element.scrollLeft, 60);

	model.applyEdits([{
		range: Range.fromPositions(new Position((1) + 1, (2) + 1)),
		text: "\n0123456789",
	}]);

	assert.equal(viewport.viewportLayout.contentSize.width, 170);
	assert.equal(viewport.viewportLayout.maximumScrollPosition.left, 120);

	dom.window.close();
});

test("Font metric refresh rebuilds authoritative horizontal width", () => {
	const dom = new JSDOM("<!doctype html><body><main></main></body>");
	const container = requiredElement(dom.window.document, "main");
	const measurer = fixedTextMeasurer(10, 20);
	using model = new TextModel("xxxx");
	using selections = new CursorsController(model, SelectionSet.single(Selection.fromPositions(new Position((0) + 1, (0) + 1))));
	using viewport = new View({
		container,
		model,
		lineHeight: 20,
		textMeasurer: measurer,
		selectionController: selections,
		cursorBlinking: 'solid',
		cursorWidth: 12,
	});
	viewport.layout({ width: 50, height: 20 });
	assert.equal(viewport.viewportLayout.contentSize.width, 106);
	assert.equal(requiredElement<HTMLElement>(viewport.element, '.stanza-editor-caret').style.width, '10px');

	measurer.setCharacterWidth(20);
	viewport.refreshFontMetrics();

	assert.equal(viewport.viewportLayout.contentSize.width, 156);
	assert.equal(viewport.viewportLayout.maximumScrollPosition.left, 106);
	assert.equal(requiredElement<HTMLElement>(viewport.element, '.stanza-editor-caret').style.width, '12px');

	dom.window.close();
});

test("Viewport disposal removes DOM without owning the text model", () => {
	const dom = new JSDOM("<!doctype html><body><main></main></body>");
	const container = requiredElement(dom.window.document, "main");
	using model = new TextModel("alpha");
	const viewport = new View({
		container,
		model,
		lineHeight: 20,
		textMeasurer: fixedTextMeasurer(),
	});

	viewport.dispose();
	assert.equal(container.childElementCount, 0);
	model.applyEdits([{
		range: Range.fromPositions(model.positionAt(5)),
		text: " editor",
	}]);
	assert.equal(model.getText(), "alpha editor");

	dom.window.close();
});

test('View mounts content and overlay widgets through their VS Code owners', () => {
	const dom = new JSDOM('<!doctype html><body><main></main></body>');
	const container = requiredElement(dom.window.document, 'main');
	using model = new TextModel('alpha');
	using viewport = new View({ container, model, lineHeight: 20, textMeasurer: fixedTextMeasurer() });
	viewport.layout({ width: 300, height: 100 });
	const contentDomNode = h(dom.window.document, 'div');
	let contentPosition: IContentWidgetPosition = { position: { lineNumber: 1, column: 2 }, preference: [ContentWidgetPositionPreference.ABOVE, ContentWidgetPositionPreference.BELOW] };
	let renderedPosition: ContentWidgetPositionPreference | null = null;
	const contentWidget = {
		getId: () => 'content',
		getDomNode: () => contentDomNode,
		getPosition: () => contentPosition,
		beforeRender: () => ({ width: 60, height: 30 }),
		afterRender: (position: ContentWidgetPositionPreference | null) => { renderedPosition = position; },
	};
	const overlayDomNode = h(dom.window.document, 'div');
	const overlayWidget = {
		getId: () => 'overlay',
		getDomNode: () => overlayDomNode,
		getPosition: () => ({ preference: OverlayWidgetPositionPreference.TOP_RIGHT_CORNER }),
	};

	viewport.addContentWidget(contentWidget);
	viewport.addOverlayWidget(overlayWidget);

	assert.equal(contentDomNode.parentElement?.className, 'stanza-editor-content-widgets');
	assert.equal(contentDomNode.style.display, 'block');
	assert.equal(contentDomNode.style.visibility, 'inherit');
	assert.equal(contentDomNode.style.top, '20px');
	assert.equal(renderedPosition, ContentWidgetPositionPreference.BELOW);
	assert.equal(overlayDomNode.parentElement?.className, 'stanza-editor-overlay-widgets');
	assert.equal(overlayDomNode.style.display, 'block');
	contentPosition = { position: { lineNumber: 1, column: 1 }, preference: [ContentWidgetPositionPreference.EXACT], positionAffinity: PositionAffinity.LeftOfInjectedText };
	viewport.layoutContentWidget(contentWidget);
	assert.equal(contentDomNode.style.top, '0px');
	assert.equal(contentDomNode.style.left, '0px');
	assert.equal(renderedPosition, ContentWidgetPositionPreference.EXACT);
	viewport.removeContentWidget(contentWidget);
	viewport.removeOverlayWidget(overlayWidget);
	assert.equal(contentDomNode.isConnected, false);
	assert.equal(overlayDomNode.isConnected, false);
	dom.window.close();
});

test('View renders relative, interval, and custom line numbers', () => {
	const dom = new JSDOM('<!doctype html><body><main></main></body>');
	const container = requiredElement(dom.window.document, 'main');
	using model = new TextModel(lines(12).join('\n'));
	using selections = new CursorsController(model, SelectionSet.single(Selection.fromPositions(new Position((5) + 1, (0) + 1))));
	using relative = new View({
		container,
		model,
		selectionController: selections,
		lineHeight: 20,
		lineNumbers: 'relative',
		textMeasurer: fixedTextMeasurer(),
	});
	relative.layout({ width: 300, height: 240 });
	const relativeRows = lineElements(relative.element);
	assert.equal(lineNumber(relativeRows[0]!).textContent, '5');
	assert.equal(lineNumber(relativeRows[5]!).textContent, '6');
	assert.equal(lineNumber(relativeRows[9]!).textContent, '4');
	relative.dispose();

	using interval = new View({ container, model, lineHeight: 20, lineNumbers: 'interval', textMeasurer: fixedTextMeasurer() });
	interval.layout({ width: 300, height: 240 });
	assert.equal(lineNumber(lineElements(interval.element)[8]!).textContent, '');
	assert.equal(lineNumber(lineElements(interval.element)[9]!).textContent, '10');
	interval.dispose();

	using custom = new View({ container, model, lineHeight: 20, lineNumbers: line => `L${line}`, textMeasurer: fixedTextMeasurer() });
	custom.layout({ width: 300, height: 40 });
	assert.equal(lineNumber(lineElements(custom.element)[0]!).textContent, 'L1');
	dom.window.close();
});

test('View changes public view zones with content and margin ownership', () => {
	const dom = new JSDOM('<!doctype html><body><main></main></body>');
	const container = requiredElement(dom.window.document, 'main');
	using model = new TextModel('alpha\nbeta');
	using viewport = new View({ container, model, lineHeight: 20, textMeasurer: fixedTextMeasurer() });
	viewport.layout({ width: 300, height: 100 });
	const domNode = h(dom.window.document, 'button');
	const marginDomNode = h(dom.window.document, 'span');
	let computedHeight = 0;
	let relativeTop = 0;
	let id = '';
	let retainedAccessor: { layoutZone(id: string): void } | undefined;
	viewport.changeViewZones(accessor => {
		retainedAccessor = accessor;
		id = accessor.addZone({
			afterLineNumber: 1,
			heightInPx: 24,
			minWidthInPx: 450,
			suppressMouseDown: true,
			domNode,
			marginDomNode,
			onComputedHeight: height => { computedHeight = height; },
			onDomNodeTop: top => { relativeTop = top; },
		});
	});

	assert.equal(domNode.parentElement?.className, 'stanza-editor-view-zones');
	assert.equal(marginDomNode.parentElement?.className, 'stanza-editor-margin-view-zones');
	assert.equal(domNode.style.height, '24px');
	assert.equal(computedHeight, 24);
	assert.equal(relativeTop, 20);
	assert.equal(viewport.currentLayout.contentSize.width, 450);
	const mouseDown = new dom.window.MouseEvent('mousedown', { bubbles: true, cancelable: true });
	domNode.dispatchEvent(mouseDown);
	assert.equal(mouseDown.defaultPrevented, true);
	assert.throws(() => retainedAccessor?.layoutZone(id), /no longer valid/);
	viewport.changeViewZones(accessor => accessor.removeZone(id));
	assert.equal(domNode.isConnected, false);
	assert.equal(marginDomNode.isConnected, false);
	dom.window.close();
});

test('View resolves line-based and default view-zone heights after line-height changes', () => {
	const dom = new JSDOM('<!doctype html><body><main></main></body>');
	const container = requiredElement(dom.window.document, 'main');
	using model = new TextModel('alpha\nbeta');
	using viewport = new View({ container, model, lineHeight: 20, textMeasurer: fixedTextMeasurer() });
	viewport.layout({ width: 300, height: 100 });
	const lineHeightNode = h(dom.window.document, 'div');
	const defaultHeightNode = h(dom.window.document, 'div');
	const fixedHeightNode = h(dom.window.document, 'div');
	viewport.changeViewZones(accessor => {
		accessor.addZone({ afterLineNumber: 1, heightInLines: 1.5, domNode: lineHeightNode });
		accessor.addZone({ afterLineNumber: 1, domNode: defaultHeightNode });
		accessor.addZone({ afterLineNumber: 1, heightInPx: 18, heightInLines: 2, domNode: fixedHeightNode });
	});
	const before = { lineHeight: lineHeightNode.style.height, defaultHeight: defaultHeightNode.style.height, fixedHeight: fixedHeightNode.style.height };

	viewport.setLineHeight(24);

	assert.deepEqual({
		before,
		after: { lineHeight: lineHeightNode.style.height, defaultHeight: defaultHeightNode.style.height, fixedHeight: fixedHeightNode.style.height },
	}, {
		before: { lineHeight: '30px', defaultHeight: '20px', fixedHeight: '18px' },
		after: { lineHeight: '36px', defaultHeight: '24px', fixedHeight: '18px' },
	});
	dom.window.close();
});

test('View lays out overflowing and relayout-aware overlay widgets', () => {
	const dom = new JSDOM('<!doctype html><body><main></main></body>');
	const container = requiredElement(dom.window.document, 'main');
	using model = new TextModel('alpha');
	using viewport = new View({
		container,
		model,
		lineHeight: 20,
		textMeasurer: fixedTextMeasurer(),
		fixedOverflowWidgets: true,
		minimap: { enabled: false },
	});
	viewport.layout({ width: 300, height: 100 });
	const domNode = h(dom.window.document, 'div');
	let width = 40;
	domNode.getBoundingClientRect = () => ({ width, height: 20 }) as DOMRect;
	using layoutEmitter = new Emitter<void>();
	const widget = {
		allowEditorOverflow: true,
		onDidLayout: layoutEmitter.event,
		getId: () => 'overflow-overlay',
		getDomNode: () => domNode,
		getPosition: () => ({ preference: OverlayWidgetPositionPreference.TOP_RIGHT_CORNER }),
		getMinContentWidthInPx: () => 420,
	};

	viewport.addOverlayWidget(widget);
	const initialLeft = Number.parseFloat(domNode.style.left);
	assert.equal(domNode.parentElement?.className, 'stanza-editor-overflowing-overlay-widgets');
	assert.equal(domNode.style.position, 'fixed');
	assert.equal(viewport.currentLayout.contentSize.width, 420);
	width = 60;
	layoutEmitter.fire();
	assert.equal(Number.parseFloat(domNode.style.left), initialLeft - 20);

	viewport.removeOverlayWidget(widget);
	assert.equal(viewport.currentLayout.contentSize.width < 420, true);
	assert.equal(domNode.isConnected, false);
	dom.window.close();
});

test('View preserves focused overflow content widgets off screen and suppresses their mouse down', () => {
	const dom = new JSDOM('<!doctype html><body><main></main></body>');
	const container = requiredElement(dom.window.document, 'main');
	using model = new TextModel('alpha\nbeta');
	using viewport = new View({ container, model, lineHeight: 20, textMeasurer: fixedTextMeasurer(), fixedOverflowWidgets: true });
	viewport.layout({ width: 300, height: 20 });
	const contentDomNode = h(dom.window.document, 'button');
	let position: IContentWidgetPosition = { position: new Position((0) + 1, (1) + 1), preference: [ContentWidgetPositionPreference.EXACT] };
	let renderedPosition: ContentWidgetPositionPreference | null = null;
	const contentWidget = {
		allowEditorOverflow: true,
		suppressMouseDown: true,
		getId: () => 'overflow-content',
		getDomNode: () => contentDomNode,
		getPosition: () => position,
		beforeRender: () => ({ width: 80, height: 24 }),
		afterRender: (nextPosition: ContentWidgetPositionPreference | null) => { renderedPosition = nextPosition; },
	};

	viewport.addContentWidget(contentWidget);
	assert.equal(contentDomNode.parentElement?.className, 'stanza-editor-overflowing-content-widgets');
	assert.equal(contentDomNode.style.position, 'fixed');
	assert.equal(renderedPosition, ContentWidgetPositionPreference.EXACT);
	const mouseDown = new dom.window.MouseEvent('mousedown', { bubbles: true, cancelable: true });
	contentDomNode.dispatchEvent(mouseDown);
	assert.equal(mouseDown.defaultPrevented, true);

	contentDomNode.focus();
	position = { position: new Position((1) + 1, (0) + 1), preference: [ContentWidgetPositionPreference.BELOW] };
	viewport.layoutContentWidget(contentWidget);
	assert.equal(contentDomNode.style.top, '-1000px');
	assert.equal(contentDomNode.style.visibility, 'inherit');
	assert.equal(dom.window.document.activeElement, contentDomNode);
	assert.equal(renderedPosition, null);

	viewport.removeContentWidget(contentWidget);
	assert.equal(contentDomNode.isConnected, false);
	dom.window.close();
});

test('View projects configured whitespace through EditorWhitespaceOverlay', () => {
	const dom = new JSDOM('<!doctype html><body><main></main></body>');
	const container = requiredElement(dom.window.document, 'main');
	using model = new TextModel('a b\t');
	using viewport = new View({ container, model, lineHeight: 20, textMeasurer: fixedTextMeasurer(), renderWhitespace: 'all' });

	viewport.layout({ width: 300, height: 100 });

	assert.deepEqual([...viewport.element.querySelectorAll('.stanza-editor-whitespace')].map(element => element.textContent), ['·', '→']);
	dom.window.close();
});

test('View limits selection whitespace to the current selections', () => {
	const dom = new JSDOM('<!doctype html><body><main></main></body>');
	const container = requiredElement(dom.window.document, 'main');
	using model = new TextModel('a b c');
	using selections = new CursorsController(model, SelectionSet.single(Selection.fromPositions(new Position((0) + 1, (1) + 1), new Position((0) + 1, (2) + 1))));
	using viewport = new View({
		container,
		model,
		selectionController: selections,
		lineHeight: 20,
		textMeasurer: fixedTextMeasurer(),
		renderWhitespace: 'selection',
	});
	viewport.layout({ width: 300, height: 100 });

	const firstMarker = requiredElement<HTMLElement>(viewport.element, '.stanza-editor-whitespace');
	const firstLeft = firstMarker.style.left;
	assert.equal(viewport.element.querySelectorAll('.stanza-editor-whitespace').length, 1);
	selections.setSelections(SelectionSet.single(Selection.fromPositions(new Position((0) + 1, (3) + 1), new Position((0) + 1, (4) + 1))));
	const secondMarker = requiredElement<HTMLElement>(viewport.element, '.stanza-editor-whitespace');
	assert.equal(viewport.element.querySelectorAll('.stanza-editor-whitespace').length, 1);
	assert.notEqual(secondMarker.style.left, firstLeft);
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

function lineElements(container: ParentNode): HTMLDivElement[] {
	return [...container.querySelectorAll<HTMLDivElement>(
		".view-line",
	)];
}

function requiredLine(container: ParentNode, lineIndex: number): HTMLDivElement {
	return requiredElement<HTMLDivElement>(
		container,
		`[data-line-index="${lineIndex}"]`,
	);
}

function lineText(line: Element | undefined): HTMLSpanElement {
	assert.ok(line);
	return requiredElement<HTMLSpanElement>(
		line,
		".stanza-editor-line-text",
	);
}

function lineNumber(line: Element | undefined): HTMLSpanElement {
	assert.ok(line);
	const editor = line.closest(".stanza-editor");
	assert.ok(editor);
	return requiredElement<HTMLSpanElement>(
		editor,
		`.stanza-editor-line-margin[data-line-index="${line.getAttribute("data-line-index")}"] .line-numbers`,
	);
}

function requiredPartRow(container: ParentNode, className: string, lineIndex: number): HTMLElement {
	return requiredElement<HTMLElement>(container, `.${className}[data-line-index="${lineIndex}"]`);
}

function lines(count: number): string[] {
	return Array.from({ length: count }, (_, index) => `line ${index}`);
}

function testRectangle(left: number, top: number, width: number): DOMRect {
	return { left, top, width, height: 20, right: left + width, bottom: top + 20, x: left, y: top, toJSON: () => ({}) } as DOMRect;
}

function fixedTextMeasurer(
	characterWidth = 8,
	horizontalPadding = 24,
): TestTextMeasurer {
	return new TestTextMeasurer(characterWidth, horizontalPadding);
}

function tabTextMeasurer(): TextMeasurer {
	return {
		horizontalPadding: 24,
		contentLeftPadding: 12,
		refresh: () => false,
		measureLineWidth: text => {
			let columns = 0;
			for (const character of text) columns += character === '\t' ? 4 - columns % 4 : 1;
			return columns * 8;
		},
	};
}

class TestTextMeasurer implements TextMeasurer {
	private dirty = false;

	constructor(
		private characterWidth: number,
		readonly horizontalPadding: number,
	) {}

	get contentLeftPadding(): number {
		return this.horizontalPadding / 2;
	}

	setCharacterWidth(characterWidth: number): void {
		this.characterWidth = characterWidth;
		this.dirty = true;
	}

	refresh(): boolean {
		const changed = this.dirty;
		this.dirty = false;
		return changed;
	}

	measureLineWidth(text: string): number {
		return [...text].length * this.characterWidth;
	}
}
