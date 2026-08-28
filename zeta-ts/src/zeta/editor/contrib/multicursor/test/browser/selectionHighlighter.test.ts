import assert from 'node:assert/strict';
import test from 'node:test';
import { JSDOM } from 'jsdom';
import { type TextMeasurer } from '../../../../browser/config/fontMeasurements.js';
import { createStanzaDecorationSource } from '../../../../browser/viewparts/decorations/decorationPresentation.js';
import { TextSelection, TextSelectionSet } from '../../../../common/core/selection.js';
import { TextPosition, TextRange } from '../../../../common/core/text.js';
import { EditorSelectionController } from '../../../../common/cursor/editorSelectionController.js';
import { TextDecorationCollection } from '../../../../common/model/decorationCollection.js';
import { TextModel } from '../../../../common/model/textModel.js';
import { LanguageFeaturesService } from '../../../../common/services/languageService.js';

const browserEnvironment = new JSDOM('<!doctype html><body></body>');
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

const { EditorView, EditorViewport } = await import('../../../../browser/view.js');
const { getSelectionHighlightDecorationOptions } = await import('../../../wordHighlighter/browser/highlightDecorations.js');
const { TextualMultiDocumentHighlightFeature } = await import('../../../wordHighlighter/browser/textualHighlightProvider.js');
const { SelectionHighlighter } = await import('../../browser/selectionHighlighter.js');

test('Selection highlighter owns non-empty textual matches and excludes active selections', () => {
	using languages = new LanguageFeaturesService();
	using textualProvider = new TextualMultiDocumentHighlightFeature(languages);
	using harness = createHarness('item itemized item\nitem', languages, TextSelection.from(TextPosition.at(0, 0), TextPosition.at(0, 2)));

	assert.deepEqual(harness.decorations.decorations.map(decoration => decoration.range), [
		TextRange.from(TextPosition.at(0, 5), TextPosition.at(0, 7)),
		TextRange.from(TextPosition.at(0, 14), TextPosition.at(0, 16)),
		TextRange.from(TextPosition.at(1, 0), TextPosition.at(1, 2)),
	]);
	assert.equal(harness.viewport.element.querySelectorAll('.selection-highlight').length, 3);
});

test('Selection highlighter applies whole-word, whitespace, multiline, and maximum-length policy', () => {
	using languages = new LanguageFeaturesService();
	using textualProvider = new TextualMultiDocumentHighlightFeature(languages);
	using harness = createHarness('item itemized item\nitem', languages, TextSelection.from(TextPosition.at(0, 0), TextPosition.at(0, 4)));

	assert.deepEqual(harness.decorations.decorations.map(decoration => decoration.range), [
		TextRange.from(TextPosition.at(0, 14), TextPosition.at(0, 18)),
		TextRange.from(TextPosition.at(1, 0), TextPosition.at(1, 4)),
	]);
	harness.selections.setSelections(TextSelectionSet.single(TextSelection.from(TextPosition.at(0, 4), TextPosition.at(0, 5))));
	assert.equal(harness.decorations.size, 0);
	harness.selections.setSelections(TextSelectionSet.single(TextSelection.from(TextPosition.at(0, 0), TextPosition.at(1, 1))));
	assert.equal(harness.decorations.size, 0);
});

function createHarness(text: string, languages: LanguageFeaturesService, initialSelection: TextSelection): SelectionHarness {
	const dom = new JSDOM('<!doctype html><body><main></main></body>');
	const container = dom.window.document.querySelector<HTMLElement>('main')!;
	const model = new TextModel(text);
	const selections = new EditorSelectionController(model, TextSelectionSet.single(initialSelection));
	const decorations = new TextDecorationCollection<boolean>(model);
	const viewport = new EditorViewport({
		container,
		model,
		lineHeight: 20,
		textMeasurer: new FixedTextMeasurer(),
		selectionController: selections,
		decorationSources: [createStanzaDecorationSource(decorations, decoration => getSelectionHighlightDecorationOptions(decoration.metadata))],
	});
	const view = new EditorView(viewport, selections);
	const controller = new SelectionHighlighter(view, selections, decorations, {
		languageId: 'typescript',
		languageFeaturesService: languages,
	});
	view.layout({ width: 240, height: 60 });
	return new SelectionHarness(dom, model, selections, decorations, viewport, view, controller);
}

class SelectionHarness implements Disposable {
	constructor(
		private readonly dom: JSDOM,
		readonly model: TextModel,
		readonly selections: EditorSelectionController,
		readonly decorations: TextDecorationCollection<boolean>,
		readonly viewport: InstanceType<typeof EditorViewport>,
		readonly view: InstanceType<typeof EditorView>,
		readonly controller: InstanceType<typeof SelectionHighlighter>,
	) {}

	dispose(): void {
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
