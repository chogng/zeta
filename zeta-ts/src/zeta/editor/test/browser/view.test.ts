import assert from 'node:assert/strict';
import test from 'node:test';
import { JSDOM } from 'jsdom';
import { CursorsController } from '../../common/cursor/cursor.js';
import { TextSelection, TextSelectionSet } from '../../common/core/selection.js';
import { TextPosition } from '../../common/core/text.js';
import { TextModel } from '../../common/model/textModel.js';
import type { TextMeasurer } from '../../browser/config/fontMeasurements.js';

const browserEnvironment = new JSDOM('<!doctype html><body></body>');
for (const [name, value] of Object.entries({
	window: browserEnvironment.window,
	document: browserEnvironment.window.document,
	Node: browserEnvironment.window.Node,
	Element: browserEnvironment.window.Element,
	HTMLElement: browserEnvironment.window.HTMLElement,
	Event: browserEnvironment.window.Event,
	InputEvent: browserEnvironment.window.InputEvent,
	KeyboardEvent: browserEnvironment.window.KeyboardEvent,
})) {
	Object.defineProperty(globalThis, name, { configurable: true, value });
}

const { EditorView } = await import('../../browser/view.js');
const { TextAreaEditContextRegistry } = await import('../../browser/controller/editContext/textArea/textAreaEditContextRegistry.js');

test.after(() => browserEnvironment.window.close());

test('EditorView gives its input context a stable owner id and releases it', () => {
	const dom = new JSDOM('<!doctype html><body><main></main></body>');
	const container = dom.window.document.querySelector('main')!;
	using model = new TextModel('alpha');
	using selections = new CursorsController(
		model,
		TextSelectionSet.single(TextSelection.collapsedAt(TextPosition.at(0, 0))),
	);
	using view = new EditorView({
		container,
		model,
		selectionController: selections,
		lineHeight: 20,
		ownerId: 'view-test',
		viewport: { textMeasurer: new FixedTextMeasurer() },
	});

	assert.equal(view.ownerId, 'view-test');
	assert.equal(TextAreaEditContextRegistry.get(view.ownerId), view.editContext);
	assert.equal(TextAreaEditContextRegistry.get(view.element), view.editContext);
	view.dispose();
	assert.equal(TextAreaEditContextRegistry.get('view-test'), undefined);
	assert.equal(TextAreaEditContextRegistry.get(view.element), undefined);
	dom.window.close();
});

test('EditorView refreshes minimap canvas dimensions when the window pixel ratio changes', () => {
	const dom = new JSDOM('<!doctype html><body><main></main></body>');
	const container = dom.window.document.querySelector('main')!;
	Object.defineProperty(dom.window, 'devicePixelRatio', { configurable: true, value: 2 });
	using model = new TextModel('alpha\nbeta');
	using selections = new CursorsController(
		model,
		TextSelectionSet.single(TextSelection.collapsedAt(TextPosition.at(0, 0))),
	);
	using view = new EditorView({
		container,
		model,
		selectionController: selections,
		lineHeight: 20,
		viewport: { textMeasurer: new FixedTextMeasurer(), minimap: { size: 'proportional' } },
	});

	view.viewport.layout({ width: 800, height: 600 });
	const canvas = view.viewport.element.querySelector<HTMLCanvasElement>('.stanza-editor-minimap-canvas')!;
	const highDensityWidth = canvas.width;
	Object.defineProperty(dom.window, 'devicePixelRatio', { configurable: true, value: 1 });
	view.viewport.layout({ width: 800, height: 600 });

	assert.equal(highDensityWidth, canvas.width * 2);
	dom.window.close();
});

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
