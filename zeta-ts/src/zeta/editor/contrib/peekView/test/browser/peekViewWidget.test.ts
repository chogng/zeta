import assert from 'node:assert/strict';
import test from 'node:test';
import { JSDOM } from 'jsdom';
import { h } from '../../../../../base/browser/dom.js';
import { type TextMeasurer } from '../../../../browser/config/fontMeasurements.js';
import { Position } from '../../../../common/core/position.js';
import { TextModel } from '../../../../common/model/textModel.js';

const browserEnvironment = new JSDOM('<!doctype html><body></body>');
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

const { EditorViewport } = await import('../../../../browser/view.js');
const { PeekViewWidget } = await import('../../browser/peekViewWidget.js');

test('PeekViewWidget renders inside reserved editor space', () => {
	const dom = new JSDOM('<!doctype html><body><main></main></body>');
	using model = new TextModel('alpha\nbeta\ngamma');
	using viewport = new EditorViewport({
		container: requiredElement<HTMLElement>(dom.window.document, 'main'),
		model,
		lineHeight: 20,
		textMeasurer: new FixedTextMeasurer(),
	});
	viewport.layout({ width: 240, height: 80 });
	using widget = new PeekViewWidget(viewport, new Position((0) + 1, (2) + 1), '2 definitions');
	const content = h(dom.window.document, 'button');
	content.textContent = 'result';
	widget.setBody(content);

	widget.show(new Position((0) + 1, (2) + 1), 2);

	assert.deepEqual({
		position: widget.position,
		parent: widget.element.parentElement,
		top: widget.element.style.top,
		height: widget.element.style.height,
		contentHeight: viewport.viewportLayout.contentSize.height,
		header: widget.element.querySelector('.stanza-editor-peek-view-header')?.textContent,
		body: widget.element.querySelector('.stanza-editor-peek-view-body')?.textContent,
		accessible: widget.element.hasAttribute('aria-hidden'),
	}, {
		position: new Position((0) + 1, (2) + 1),
		parent: requiredElement(viewport.element, '.stanza-editor-view-zones'),
		top: '20px',
		height: '40px',
		contentHeight: 100,
		header: '2 definitions',
		body: 'result',
		accessible: false,
	});

	widget.hide();
	assert.equal(widget.element.parentElement, null);
	assert.equal(viewport.viewportLayout.contentSize.height, 80);
	dom.window.close();
});

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
