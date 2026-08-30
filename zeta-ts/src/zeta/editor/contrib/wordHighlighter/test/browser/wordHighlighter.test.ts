import assert from 'node:assert/strict';
import test from 'node:test';
import { JSDOM } from 'jsdom';
import { URI } from '../../../../../base/common/uri.js';
import { type TextMeasurer } from '../../../../browser/config/fontMeasurements.js';
import { createStanzaDecorationSource } from '../../../../browser/viewparts/decorations/decorations.js';
import { Selection } from '../../../../common/core/selection.js';
import { SelectionSet } from '../../../../common/cursor/selectionSet.js';
import { Position } from '../../../../common/core/position.js';
import { Range } from '../../../../common/core/range.js';
import { CursorsController } from '../../../../common/cursor/cursor.js';
import { DocumentHighlightKind } from '../../../../common/languages.js';
import { TextDecorationCollection } from '../../../../common/model/decorationCollection.js';
import { TextModel } from '../../../../common/model/textModel.js';
import { TestLanguageFeaturesService as LanguageFeaturesService } from '../../../../test/common/testLanguageFeaturesService.js';

const browserEnvironment = new JSDOM('<!doctype html><body></body>');
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

const { EditorView, EditorViewport } = await import('../../../../browser/view.js');
const { resolveDocumentHighlightPresentation } = await import('../../browser/highlightDecorations.js');
const { TextualMultiDocumentHighlightFeature } = await import('../../browser/textualHighlightProvider.js');
const { WordHighlighterContribution } = await import('../../browser/wordHighlighter.contribution.js');

test('Word highlighter uses the textual provider for complete Unicode words', async () => {
	using languages = new LanguageFeaturesService();
	using harness = createHarness('café caféine café\nCafé', languages, URI.parse('file:///one.ts'), 'singleFile');

	harness.controller.restoreViewState(true);
	await settleHighlights();

	assert.deepEqual(decorationRanges(harness.decorations), [
		Range.fromPositions(new Position((0) + 1, (0) + 1), new Position((0) + 1, (4) + 1)),
		Range.fromPositions(new Position((0) + 1, (13) + 1), new Position((0) + 1, (17) + 1)),
	]);
});

test('Word highlighter prefers semantic providers and renders read and write kinds independently', async () => {
	using languages = new LanguageFeaturesService();
	using semanticProvider = languages.documentHighlightProvider.register({
		languageIds: ['typescript'],
		provideDocumentHighlights: () => [
			{ range: Range.fromPositions(new Position((0) + 1, (0) + 1), new Position((0) + 1, (4) + 1)), kind: DocumentHighlightKind.Read },
			{ range: Range.fromPositions(new Position((0) + 1, (5) + 1), new Position((0) + 1, (9) + 1)), kind: DocumentHighlightKind.Write },
		],
	});
	using harness = createHarness('item item', languages, URI.parse('file:///semantic.ts'), 'singleFile');

	harness.controller.restoreViewState(true);
	await settleHighlights();

	assert.deepEqual(harness.decorations.decorations.map(decoration => decoration.metadata), [DocumentHighlightKind.Read, DocumentHighlightKind.Write]);
	assert.equal(harness.viewport.element.querySelectorAll('.word-highlight').length, 1);
	assert.equal(harness.viewport.element.querySelectorAll('.word-highlight-strong').length, 1);
});

test('Multi-file word highlighting updates every open editor sharing the language service', async () => {
	using languages = new LanguageFeaturesService();
	using first = createHarness('item one item', languages, URI.parse('file:///one.ts'), 'multiFile');
	using second = createHarness('item two', languages, URI.parse('file:///two.ts'), 'multiFile');

	first.controller.restoreViewState(true);
	await settleHighlights();

	assert.deepEqual([first.decorations.size, second.decorations.size], [2, 1]);
});

test('Word highlighter cancels a stale provider request when the selection changes', async () => {
	using languages = new LanguageFeaturesService();
	let aborted = false;
	using semanticProvider = languages.documentHighlightProvider.register({
		languageIds: ['typescript'],
		provideDocumentHighlights: (_model, _position, token) => new Promise(resolve => {
			token.onCancellationRequested(() => {
				aborted = true;
				resolve([]);
			});
		}),
	});
	using harness = createHarness('item other', languages, URI.parse('file:///cancel.ts'), 'singleFile');

	harness.controller.restoreViewState(true);
	await new Promise(resolve => setTimeout(resolve, 1));
	harness.selections.setSelections(SelectionSet.single(Selection.fromPositions(new Position((0) + 1, (6) + 1))));
	await settleHighlights();

	assert.equal(aborted, true);
	assert.equal(harness.decorations.size, 0);
});

test('Word highlighter obeys the off mode and navigates existing highlights', async () => {
	using languages = new LanguageFeaturesService();
	using disabled = createHarness('item item', languages, URI.parse('file:///disabled.ts'), 'off');
	disabled.controller.restoreViewState(true);
	await settleHighlights();
	assert.equal(disabled.decorations.size, 0);

	using enabled = createHarness('item item', languages, URI.parse('file:///enabled.ts'), 'singleFile');
	enabled.controller.restoreViewState(true);
	await settleHighlights();
	enabled.controller.moveNext();
	assert.deepEqual(enabled.selections.selections.primary, Selection.fromPositions(new Position((0) + 1, (5) + 1)));
	enabled.controller.moveBack();
	assert.deepEqual(enabled.selections.selections.primary, Selection.fromPositions(new Position((0) + 1, (0) + 1)));
});

function createHarness(text: string, languages: LanguageFeaturesService, resource: URI, mode: 'off' | 'singleFile' | 'multiFile'): EditorHarness {
	const dom = new JSDOM('<!doctype html><body><main></main></body>');
	const container = dom.window.document.querySelector<HTMLElement>('main')!;
	const model = new TextModel(text);
	const selections = new CursorsController(model, SelectionSet.single(Selection.fromPositions(new Position((0) + 1, (1) + 1))));
	const textualProvider = new TextualMultiDocumentHighlightFeature(languages, { resource, model, wordPattern: () => undefined });
	const decorations = new TextDecorationCollection<DocumentHighlightKind | undefined>(model);
	const viewport = new EditorViewport({
		container,
		model,
		lineHeight: 20,
		textMeasurer: new FixedTextMeasurer(),
		selectionController: selections,
		decorationSources: [createStanzaDecorationSource(decorations, decoration => resolveDocumentHighlightPresentation(decoration.metadata))],
	});
	const view = new EditorView(viewport, selections);
	const controller = new WordHighlighterContribution(view, selections, decorations, {
		resource,
		languageId: 'typescript',
		languageFeaturesService: languages,
		mode,
		delay: 0,
	});
	view.layout({ width: 240, height: 60 });
	return new EditorHarness(dom, model, selections, decorations, viewport, view, controller, textualProvider);
}

class EditorHarness implements Disposable {
	constructor(
		private readonly dom: JSDOM,
		readonly model: TextModel,
		readonly selections: CursorsController,
		readonly decorations: TextDecorationCollection<DocumentHighlightKind | undefined>,
		readonly viewport: InstanceType<typeof EditorViewport>,
		readonly view: InstanceType<typeof EditorView>,
		readonly controller: InstanceType<typeof WordHighlighterContribution>,
		private readonly textualProvider: InstanceType<typeof TextualMultiDocumentHighlightFeature>,
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

function decorationRanges(decorations: TextDecorationCollection<DocumentHighlightKind | undefined>): readonly Range[] {
	return decorations.decorations.map(decoration => decoration.range);
}

async function settleHighlights(): Promise<void> {
	await new Promise(resolve => setTimeout(resolve, 10));
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
