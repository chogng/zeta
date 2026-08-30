import assert from 'node:assert/strict';
import test from 'node:test';
import { JSDOM } from 'jsdom';
import { type TextMeasurer } from '../../browser/config/fontMeasurements.js';
import { CursorsController } from '../../common/cursor/cursor.js';
import { Position } from '../../common/core/position.js';
import { Selection } from '../../common/core/selection.js';
import { SelectionSet } from '../../common/cursor/selectionSet.js';
import { TextModel } from '../../common/model/textModel.js';

const browserEnvironment = new JSDOM('<!doctype html><body></body>');
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

const { View } = await import('../../browser/view.js');
const { ViewStableEditorBottomScrollState, ViewStableEditorScrollState } = await import('../../browser/stableEditorScroll.js');

test('StableEditorScrollState preserves the first visible row offset', () => {
	const dom = new JSDOM('<!doctype html><body><main></main></body>');
	const container = requiredElement(dom.window.document, 'main');
	using model = new TextModel([
		'x'.repeat(100),
		'one',
		'two',
		'three',
		'four',
		'five',
		'six',
	].join('\n'));
	using viewport = new View({
		container,
		model,
		lineHeight: 20,
		textMeasurer: fixedTextMeasurer(),
	});
	viewport.layout({ width: 80, height: 40 });
	viewport.scrollTo({ left: 100, top: 45 });

	const initialLayout = viewport.currentLayout;
	const initialVisibleLine = viewport.getVisualLineProjection().lineAt(
		initialLayout.visibleLines.startLineIndex,
	);
	assert.ok(initialVisibleLine);
	const anchor = new Position((initialVisibleLine.logicalLineIndex) + 1, (initialVisibleLine.startColumn) + 1);
	const initialAnchorTop = viewport.getPositionContentCoordinates(anchor).top;
	const initialDelta = initialLayout.scrollPosition.top - initialAnchorTop;
	const initialLeft = initialLayout.scrollPosition.left;

	const state = ViewStableEditorScrollState.capture(viewport);
	viewport.setLineHeight(30);
	state.restore(viewport);

	const restoredLayout = viewport.currentLayout;
	assert.equal(restoredLayout.scrollPosition.left, initialLeft);
	assert.equal(
		restoredLayout.scrollPosition.top - viewport.getPositionContentCoordinates(anchor).top,
		initialDelta,
	);
	dom.window.close();
});

test('StableEditorBottomScrollState preserves the last visible row offset', () => {
	const dom = new JSDOM('<!doctype html><body><main></main></body>');
	const container = requiredElement(dom.window.document, 'main');
	using model = new TextModel([
		'one',
		'two',
		'three',
		'four',
		'five',
		'six',
		'seven',
		'eight',
	].join('\n'));
	using viewport = new View({
		container,
		model,
		lineHeight: 20,
		textMeasurer: fixedTextMeasurer(),
	});
	viewport.layout({ width: 200, height: 60 });
	viewport.scrollTo({ left: 0, top: 45 });

	const initialLayout = viewport.currentLayout;
	const initialVisibleLine = viewport.getVisualLineProjection().lineAt(
		initialLayout.visibleLines.endLineIndexExclusive - 1,
	);
	assert.ok(initialVisibleLine);
	const anchor = new Position((initialVisibleLine.logicalLineIndex) + 1, (initialVisibleLine.startColumn) + 1);
	const initialCoordinates = viewport.getPositionContentCoordinates(anchor);
	const initialDelta = initialCoordinates.top + initialCoordinates.height - initialLayout.scrollPosition.top;

	const state = ViewStableEditorBottomScrollState.capture(viewport);
	viewport.setLineHeight(30);
	state.restore(viewport);

	const restoredLayout = viewport.currentLayout;
	const restoredCoordinates = viewport.getPositionContentCoordinates(anchor);
	assert.equal(
		restoredCoordinates.top + restoredCoordinates.height - restoredLayout.scrollPosition.top,
		initialDelta,
	);
	dom.window.close();
});

test('StableEditorScrollState restores the cursor relative to the viewport', () => {
	const dom = new JSDOM('<!doctype html><body><main></main></body>');
	const container = requiredElement(dom.window.document, 'main');
	using model = new TextModel([
		'one',
		'two',
		'three',
		'four',
		'five',
		'six',
		'seven',
		'eight',
	].join('\n'));
	using selections = new CursorsController(
		model,
		SelectionSet.single(Selection.fromPositions(new Position((1) + 1, (0) + 1))),
	);
	using viewport = new View({
		container,
		model,
		lineHeight: 20,
		selectionController: selections,
		textMeasurer: fixedTextMeasurer(),
	});
	viewport.layout({ width: 200, height: 60 });
	viewport.scrollTo({ left: 0, top: 40 });

	const state = ViewStableEditorScrollState.capture(viewport, selections);
	selections.setSelections(SelectionSet.single(
		Selection.fromPositions(new Position((4) + 1, (0) + 1)),
	));
	viewport.setLineHeight(30);
	state.restoreRelativeVerticalPositionOfCursor(viewport, selections);

	assert.equal(viewport.currentLayout.scrollPosition.top, 150);
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

function fixedTextMeasurer(): TextMeasurer {
	return {
		horizontalPadding: 24,
		contentLeftPadding: 12,
		refresh: () => false,
		measureLineWidth: text => [...text].length * 8,
	};
}
