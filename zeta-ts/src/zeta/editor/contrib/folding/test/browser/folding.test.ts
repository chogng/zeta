import assert from "node:assert/strict";
import test from "node:test";
import { JSDOM } from "jsdom";
import { OperatingSystem } from "../../../../../base/common/platform.js";
import { type TextMeasurer } from "../../../../browser/config/fontMeasurements.js";
import { EditorSelectionController } from "../../../../common/cursor/cursor.js";
import { EditorFoldingModel } from "../../browser/foldingModel.js";
import { EditorHiddenRangeModel } from "../../browser/hiddenRangeModel.js";
import { FoldingDecorationProvider } from "../../browser/foldingDecorations.js";
import { TextSelection, TextSelectionSet } from "../../../../common/core/selection.js";
import { TextPosition } from "../../../../common/core/text.js";
import { TextModel } from "../../../../common/model/textModel.js";
import { h } from "../../../../../base/browser/dom.js";
import { type TextEditorContributionContext } from "../../../../browser/editorExtensions.js";

const browserEnvironment = new JSDOM("<!doctype html><body></body>");
for (const [name, value] of Object.entries({
	window: browserEnvironment.window,
	document: browserEnvironment.window.document,
	Node: browserEnvironment.window.Node,
	Element: browserEnvironment.window.Element,
	HTMLElement: browserEnvironment.window.HTMLElement,
	Event: browserEnvironment.window.Event,
	KeyboardEvent: browserEnvironment.window.KeyboardEvent,
	MouseEvent: browserEnvironment.window.MouseEvent,
})) {
	Object.defineProperty(globalThis, name, { configurable: true, value });
}

const { EditorViewport } = await import("../../../../browser/view.js");
const { FoldingCommand, FoldingController, resolveStanzaFoldingCommand } = await import("../../browser/folding.js");

test.after(() => browserEnvironment.window.close());

test("Folding controller routes platform chords and gutter toggles through the front-end folding model", () => {
	const dom = new JSDOM("<!doctype html><body><main></main></body>");
	const container = requiredElement<HTMLElement>(dom.window.document, "main");
	using model = new TextModel("header\nbody\nend\nafter");
	using selections = new EditorSelectionController(model, TextSelectionSet.single(TextSelection.collapsedAt(TextPosition.at(1, 1))));
	using folding = new EditorFoldingModel(model);
	using hiddenRanges = new EditorHiddenRangeModel(model, folding);
	folding.setRanges([{ startLineIndex: 0, endLineIndex: 2 }]);
	using decorations = new FoldingDecorationProvider(folding);
	using viewport = new EditorViewport({
		container,
		model,
		lineHeight: 20,
		textMeasurer: new FixedTextMeasurer(),
		selectionController: selections,
		lineVisibilitySource: hiddenRanges,
		decorationSources: [decorations],
	});
	viewport.layout({ width: 300, height: 60 });
	const input = h(dom.window.document, "textarea") as unknown as HTMLTextAreaElement;
	viewport.element.append(input);
	using controller = new FoldingController(foldingControllerContext(input, viewport, selections, folding), { operatingSystem: OperatingSystem.Windows });

	const collapse = keyboardEvent(dom.window, "[", { ctrlKey: true, shiftKey: true });
	input.dispatchEvent(collapse);
	assert.equal(collapse.defaultPrevented, true);
	assert.equal(folding.regions[0]?.collapsed, true);
	assert.deepEqual(selections.selections.primary.active, TextPosition.at(0, 6));
	assert.deepEqual(renderedLogicalLines(viewport.element), ["0", "3"]);

	const expand = keyboardEvent(dom.window, "]", { ctrlKey: true, shiftKey: true });
	input.dispatchEvent(expand);
	assert.equal(folding.regions[0]?.collapsed, false);
	const toggle = requiredElement<HTMLButtonElement>(viewport.element, ".stanza-editor-fold-toggle");
	const gutterToggle = new dom.window.MouseEvent("pointerdown", { bubbles: true, cancelable: true });
	toggle.dispatchEvent(gutterToggle);
	assert.equal(gutterToggle.defaultPrevented, true);
	assert.equal(folding.regions[0]?.collapsed, true);

	input.remove();
	dom.window.close();
});

test("Folding chord resolution follows Windows/Linux and macOS conventions", () => {
	assert.equal(resolveStanzaFoldingCommand({ key: "[", ctrlKey: true, shiftKey: true, altKey: false, metaKey: false }, OperatingSystem.Linux), FoldingCommand.Collapse);
	assert.equal(resolveStanzaFoldingCommand({ key: "]", ctrlKey: false, shiftKey: false, altKey: true, metaKey: true }, OperatingSystem.Macintosh), FoldingCommand.Expand);
	assert.equal(resolveStanzaFoldingCommand({ key: "[", ctrlKey: true, shiftKey: false, altKey: false, metaKey: false }, OperatingSystem.Windows), undefined);
});

test("Folding controller routes macOS Command+K chords without accepting Control", () => {
	const dom = new JSDOM("<!doctype html><body><main></main></body>");
	const container = requiredElement<HTMLElement>(dom.window.document, "main");
	using model = new TextModel("first\n  nested\nsecond\n  nested");
	using selections = new EditorSelectionController(model, TextSelectionSet.single(TextSelection.collapsedAt(TextPosition.at(1, 1))));
	using folding = new EditorFoldingModel(model);
	using hiddenRanges = new EditorHiddenRangeModel(model, folding);
	folding.setRanges([{ startLineIndex: 0, endLineIndex: 1 }, { startLineIndex: 2, endLineIndex: 3 }]);
	using decorations = new FoldingDecorationProvider(folding);
	using viewport = new EditorViewport({ container, model, lineHeight: 20, textMeasurer: new FixedTextMeasurer(), selectionController: selections, lineVisibilitySource: hiddenRanges, decorationSources: [decorations] });
	const input = h(dom.window.document, "textarea") as unknown as HTMLTextAreaElement;
	viewport.element.append(input);
	using controller = new FoldingController(foldingControllerContext(input, viewport, selections, folding), { operatingSystem: OperatingSystem.Macintosh });

	input.dispatchEvent(keyboardEvent(dom.window, "k", { metaKey: true }));
	const collapseAll = keyboardEvent(dom.window, "0", { metaKey: true });
	input.dispatchEvent(collapseAll);
	assert.equal(collapseAll.defaultPrevented, true);
	assert.equal(folding.regions.every(region => region.collapsed), true);

	const controlPrefix = keyboardEvent(dom.window, "k", { ctrlKey: true });
	input.dispatchEvent(controlPrefix);
	assert.equal(controlPrefix.defaultPrevented, false);
	dom.window.close();
});

test("Folding controller collapses and expands every range through Ctrl+K chords", () => {
	const dom = new JSDOM("<!doctype html><body><main></main></body>");
	const container = requiredElement<HTMLElement>(dom.window.document, "main");
	using model = new TextModel("first\n  nested\nsecond\n  nested");
	using selections = new EditorSelectionController(model, TextSelectionSet.single(TextSelection.collapsedAt(TextPosition.at(1, 1))));
	using folding = new EditorFoldingModel(model);
	using hiddenRanges = new EditorHiddenRangeModel(model, folding);
	folding.setRanges([{ startLineIndex: 0, endLineIndex: 1 }, { startLineIndex: 2, endLineIndex: 3 }]);
	using decorations = new FoldingDecorationProvider(folding);
	using viewport = new EditorViewport({ container, model, lineHeight: 20, textMeasurer: new FixedTextMeasurer(), selectionController: selections, lineVisibilitySource: hiddenRanges, decorationSources: [decorations] });
	const input = h(dom.window.document, "textarea") as unknown as HTMLTextAreaElement;
	viewport.element.append(input);
	using controller = new FoldingController(foldingControllerContext(input, viewport, selections, folding), { operatingSystem: OperatingSystem.Windows });
	input.dispatchEvent(keyboardEvent(dom.window, "k", { ctrlKey: true }));
	const collapseAll = keyboardEvent(dom.window, "0", { ctrlKey: true });
	input.dispatchEvent(collapseAll);
	assert.equal(collapseAll.defaultPrevented, true);
	assert.equal(folding.regions.every(region => region.collapsed), true);
	input.dispatchEvent(keyboardEvent(dom.window, "k", { ctrlKey: true }));
	input.dispatchEvent(keyboardEvent(dom.window, "j", { ctrlKey: true }));
	assert.equal(folding.regions.every(region => !region.collapsed), true);
	dom.window.close();
});

test("Folding controller recursively folds nested regions through platform prefix chords", () => {
	const dom = new JSDOM("<!doctype html><body><main></main></body>");
	const container = requiredElement<HTMLElement>(dom.window.document, "main");
	using model = new TextModel("outer\nchild\ngrandchild\nend child\nend outer");
	using selections = new EditorSelectionController(model, TextSelectionSet.single(TextSelection.collapsedAt(TextPosition.at(1, 0))));
	using folding = new EditorFoldingModel(model);
	using hiddenRanges = new EditorHiddenRangeModel(model, folding);
	folding.setRanges([{ startLineIndex: 0, endLineIndex: 4 }, { startLineIndex: 1, endLineIndex: 3 }, { startLineIndex: 2, endLineIndex: 3 }]);
	using decorations = new FoldingDecorationProvider(folding);
	using viewport = new EditorViewport({ container, model, lineHeight: 20, textMeasurer: new FixedTextMeasurer(), selectionController: selections, lineVisibilitySource: hiddenRanges, decorationSources: [decorations] });
	const input = h(dom.window.document, "textarea") as unknown as HTMLTextAreaElement;
	viewport.element.append(input);
	using controller = new FoldingController(foldingControllerContext(input, viewport, selections, folding), { operatingSystem: OperatingSystem.Macintosh });

	input.dispatchEvent(keyboardEvent(dom.window, "k", { metaKey: true }));
	const fold = keyboardEvent(dom.window, "[", { metaKey: true });
	input.dispatchEvent(fold);
	assert.equal(fold.defaultPrevented, true);
	assert.deepEqual(folding.regions.map(region => region.collapsed), [false, true, true]);

	selections.setSelections(TextSelectionSet.single(TextSelection.collapsedAt(TextPosition.at(1, 0))));
	input.dispatchEvent(keyboardEvent(dom.window, "k", { metaKey: true }));
	input.dispatchEvent(keyboardEvent(dom.window, "]", { metaKey: true }));
	assert.deepEqual(folding.regions.map(region => region.collapsed), [false, false, false]);
	dom.window.close();
});

test("Folding controller creates and removes manual ranges through macOS prefix chords", () => {
	const dom = new JSDOM("<!doctype html><body><main></main></body>");
	const container = requiredElement<HTMLElement>(dom.window.document, "main");
	using model = new TextModel("first\nmanual start\nbody\nmanual end\nlast");
	using selections = new EditorSelectionController(model, TextSelectionSet.single(TextSelection.from(TextPosition.at(1, 0), TextPosition.at(4, 0))));
	using folding = new EditorFoldingModel(model);
	using hiddenRanges = new EditorHiddenRangeModel(model, folding);
	using decorations = new FoldingDecorationProvider(folding);
	using viewport = new EditorViewport({ container, model, lineHeight: 20, textMeasurer: new FixedTextMeasurer(), selectionController: selections, lineVisibilitySource: hiddenRanges, decorationSources: [decorations] });
	const input = h(dom.window.document, "textarea") as unknown as HTMLTextAreaElement;
	viewport.element.append(input);
	using controller = new FoldingController(foldingControllerContext(input, viewport, selections, folding), { operatingSystem: OperatingSystem.Macintosh });

	input.dispatchEvent(keyboardEvent(dom.window, "k", { metaKey: true }));
	input.dispatchEvent(keyboardEvent(dom.window, ",", { metaKey: true }));
	assert.deepEqual(folding.regions.map(region => [region.startLineIndex, region.endLineIndex, region.source]), [[1, 3, "manual"]]);

	selections.setSelections(TextSelectionSet.single(TextSelection.collapsedAt(TextPosition.at(2, 0))));
	input.dispatchEvent(keyboardEvent(dom.window, "k", { metaKey: true }));
	input.dispatchEvent(keyboardEvent(dom.window, ".", { metaKey: true }));
	assert.deepEqual(folding.regions, []);
	dom.window.close();
});

test("Folding controller collapses macOS prefix levels without hiding shallower headers", () => {
	const dom = new JSDOM("<!doctype html><body><main></main></body>");
	const container = requiredElement<HTMLElement>(dom.window.document, "main");
	using model = new TextModel("outer\nchild\ngrandchild\nend grandchild\nend child\nend outer");
	using selections = new EditorSelectionController(model, TextSelectionSet.single(TextSelection.collapsedAt(TextPosition.at(0, 0))));
	using folding = new EditorFoldingModel(model);
	using hiddenRanges = new EditorHiddenRangeModel(model, folding);
	folding.setRanges([{ startLineIndex: 0, endLineIndex: 5 }, { startLineIndex: 1, endLineIndex: 4 }, { startLineIndex: 2, endLineIndex: 3 }]);
	using decorations = new FoldingDecorationProvider(folding);
	using viewport = new EditorViewport({ container, model, lineHeight: 20, textMeasurer: new FixedTextMeasurer(), selectionController: selections, lineVisibilitySource: hiddenRanges, decorationSources: [decorations] });
	const input = h(dom.window.document, "textarea") as unknown as HTMLTextAreaElement;
	viewport.element.append(input);
	using controller = new FoldingController(foldingControllerContext(input, viewport, selections, folding), { operatingSystem: OperatingSystem.Macintosh });

	input.dispatchEvent(keyboardEvent(dom.window, "k", { metaKey: true }));
	const level = keyboardEvent(dom.window, "2", { metaKey: true });
	input.dispatchEvent(level);
	assert.equal(level.defaultPrevented, true);
	assert.deepEqual(folding.regions.map(region => region.collapsed), [false, true, true]);
	dom.window.close();
});

function renderedLogicalLines(root: ParentNode): readonly string[] {
	return [...root.querySelectorAll<HTMLElement>(".stanza-editor-line")].map(line => line.dataset.logicalLineIndex!);
}

function foldingControllerContext(
	input: HTMLTextAreaElement,
	viewport: InstanceType<typeof EditorViewport>,
	selections: EditorSelectionController,
	folding: EditorFoldingModel,
): TextEditorContributionContext {
	return {
		model: viewport.textModel,
		view: { element: input },
		viewport,
		selections,
		getCapability: () => folding,
	} as unknown as TextEditorContributionContext;
}

function keyboardEvent(targetWindow: typeof browserEnvironment.window, key: string, options: KeyboardEventInit): KeyboardEvent {
	return new targetWindow.KeyboardEvent("keydown", { bubbles: true, cancelable: true, key, ...options }) as unknown as KeyboardEvent;
}

function requiredElement<T extends Element = HTMLElement>(root: ParentNode, selector: string): T {
	const element = root.querySelector<T>(selector);
	assert.ok(element);
	return element;
}

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
