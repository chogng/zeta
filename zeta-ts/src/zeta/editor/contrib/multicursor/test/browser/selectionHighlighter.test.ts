import assert from 'node:assert/strict';
import test from 'node:test';
import { JSDOM } from 'jsdom';
import { URI } from '../../../../../base/common/uri.js';
import { type TextMeasurer } from '../../../../browser/config/fontMeasurements.js';
import { createStanzaDecorationSource } from '../../../../browser/viewParts/decorations/decorations.js';
import { Selection } from '../../../../common/core/selection.js';
import { SelectionSet } from '../../../../common/cursor/selectionSet.js';
import { Position } from '../../../../common/core/position.js';
import { Range } from '../../../../common/core/range.js';
import { CursorsController } from '../../../../common/cursor/cursor.js';
import { TextDecorationCollection } from '../../../../common/model/decorationCollection.js';
import { TextModel } from '../../../../common/model/textModel.js';
import { TestLanguageFeaturesService as EditorLanguageFeaturesService } from '../../../../test/common/testLanguageFeaturesService.js';

const browserEnvironment = new JSDOM('<!doctype html><body></body>');
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
	ResizeObserver: TestResizeObserver,
})) {
	Object.defineProperty(globalThis, name, { configurable: true, value });
}

const { EditorView } = await import('../../../../browser/editorView.js');
const { View } = await import('../../../../browser/view.js');
const { resolveSelectionHighlightPresentation } = await import('../../../wordHighlighter/browser/highlightDecorations.js');
const { TextualHighlightTargetRegistration } = await import('../../../wordHighlighter/browser/textualHighlightProvider.js');
const { EditorSelectionHighlighter } = await import('../../browser/multicursor.js');

test('Selection highlighter owns non-empty textual matches and excludes active selections', () => {
	using languages = new EditorLanguageFeaturesService();
	using harness = createHarness('item itemized item\nitem', languages, Selection.fromPositions(new Position((0) + 1, (0) + 1), new Position((0) + 1, (2) + 1)));

	assert.deepEqual(harness.decorations.decorations.map(decoration => decoration.range), [
		Range.fromPositions(new Position((0) + 1, (5) + 1), new Position((0) + 1, (7) + 1)),
		Range.fromPositions(new Position((0) + 1, (14) + 1), new Position((0) + 1, (16) + 1)),
		Range.fromPositions(new Position((1) + 1, (0) + 1), new Position((1) + 1, (2) + 1)),
	]);
	assert.equal(harness.viewport.element.querySelectorAll('.selection-highlight').length, 3);
});

test('Selection highlighter applies whole-word, whitespace, multiline, and maximum-length policy', () => {
	using languages = new EditorLanguageFeaturesService();
	using harness = createHarness('item itemized item\nitem', languages, Selection.fromPositions(new Position((0) + 1, (0) + 1), new Position((0) + 1, (4) + 1)));

	assert.deepEqual(harness.decorations.decorations.map(decoration => decoration.range), [
		Range.fromPositions(new Position((0) + 1, (14) + 1), new Position((0) + 1, (18) + 1)),
		Range.fromPositions(new Position((1) + 1, (0) + 1), new Position((1) + 1, (4) + 1)),
	]);
	harness.selections.setSelections(SelectionSet.single(Selection.fromPositions(new Position((0) + 1, (4) + 1), new Position((0) + 1, (5) + 1))));
	assert.equal(harness.decorations.size, 0);
	harness.selections.setSelections(SelectionSet.single(Selection.fromPositions(new Position((0) + 1, (0) + 1), new Position((1) + 1, (1) + 1))));
	assert.equal(harness.decorations.size, 0);
});

function createHarness(text: string, languages: EditorLanguageFeaturesService, initialSelection: Selection): SelectionHarness {
	const dom = new JSDOM('<!doctype html><body><main></main></body>');
	const container = dom.window.document.querySelector<HTMLElement>('main')!;
	const model = new TextModel(text);
	const selections = new CursorsController(model, SelectionSet.single(initialSelection));
	const textualProvider = new TextualHighlightTargetRegistration(languages, { resource: URI.parse('file:///selection.ts'), model, wordPattern: () => undefined });
	const decorations = new TextDecorationCollection<boolean>(model);
	const viewport = new View({
		container,
		model,
		lineHeight: 20,
		textMeasurer: new FixedTextMeasurer(),
		selectionController: selections,
		decorationSources: [createStanzaDecorationSource(decorations, decoration => resolveSelectionHighlightPresentation(decoration.metadata))],
	});
	const view = new EditorView(viewport, selections);
	const controller = new EditorSelectionHighlighter(view, selections, decorations, {
		languageId: 'typescript',
		languageFeaturesService: languages,
	});
	view.layout({ width: 240, height: 60 });
	return new SelectionHarness(dom, model, selections, decorations, viewport, view, controller, textualProvider);
}

class SelectionHarness implements Disposable {
	constructor(
		private readonly dom: JSDOM,
		readonly model: TextModel,
		readonly selections: CursorsController,
		readonly decorations: TextDecorationCollection<boolean>,
		readonly viewport: InstanceType<typeof View>,
		readonly view: InstanceType<typeof EditorView>,
		readonly controller: InstanceType<typeof EditorSelectionHighlighter>,
		private readonly textualProvider: InstanceType<typeof TextualHighlightTargetRegistration>,
	) {}

	dispose(): void {
		this.textualProvider.dispose();
		this.controller.dispose();
		this.view.dispose();
		this.viewport.dispose();
		this.decorations.dispose();
		this.selections.dispose();
		this.model.dispose();
		this.dom.window.close();
	}

	[Symbol.dispose](): void {
		this.dispose();
	}
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
