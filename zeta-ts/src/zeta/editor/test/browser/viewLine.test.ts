import assert from 'node:assert/strict';
import test from 'node:test';
import { JSDOM } from 'jsdom';

const browserEnvironment = new JSDOM('<!doctype html><body></body>');
for (const [name, value] of Object.entries({
	window: browserEnvironment.window,
	document: browserEnvironment.window.document,
	Node: browserEnvironment.window.Node,
	Element: browserEnvironment.window.Element,
	HTMLElement: browserEnvironment.window.HTMLElement,
	NodeFilter: browserEnvironment.window.NodeFilter,
})) {
	Object.defineProperty(globalThis, name, { configurable: true, value });
}

const { SemanticTokenPresentation } = await import('../../browser/viewparts/semanticTokens/semanticTokenPresentation.js');
const { ViewLine } = await import('../../browser/viewparts/viewLines/viewLine.js');
const { ViewLineTextDirection, ViewLineOptions } = await import('../../browser/viewparts/viewLines/viewLineOptions.js');

test('ViewLine owns rendering, character mapping, geometry, and DOM hit conversion', () => {
	const dom = new JSDOM('<!doctype html><body></body>');
	const line = new ViewLine(dom.window.document.body, 0, new ViewLineOptions({
		textDirection: ViewLineTextDirection.Auto,
		fontLigatures: false,
		useGpu: false,
		lineHeight: 20,
		tabSize: 4,
	}));
	assert.equal(line.domNode.domNode.children.length, 1);
	assert.equal(line.domNode.domNode.firstElementChild, line.textElement);
	line.renderText('ab😊cd', [{ startColumn: 2, endColumn: 4, presentation: SemanticTokenPresentation.String }], []);
	assert.equal(line.textElement.textContent, 'ab😊cd');
	assert.deepEqual([...line.textElement.children].map(child => child.textContent), ['ab', '😊', 'cd']);
	assert.equal(line.getColumnOfNodeOffset(line.textElement.children[2] as HTMLElement, 1), 5);

	const hitNode = line.textElement.children[1]!.firstChild;
	assert.ok(hitNode);
	Object.defineProperty(dom.window.document, 'caretPositionFromPoint', {
		configurable: true,
		value: () => ({ offsetNode: hitNode, offset: 2 }),
	});
	assert.equal(line.getOffsetAtClientPoint(20, 30), 4);

	Object.defineProperty(line.domNode.domNode, 'getBoundingClientRect', { configurable: true, value: () => rectangle(100, 0, 100) });
	Object.defineProperty(line.domNode.domNode, 'offsetWidth', { configurable: true, value: 100 });
	const createRange = dom.window.document.createRange.bind(dom.window.document);
	Object.defineProperty(dom.window.document, 'createRange', {
		configurable: true,
		value: () => {
			const range = createRange();
			Object.defineProperty(range, 'getClientRects', { configurable: true, value: () => [rectangle(130, 0, 0)] });
			return range;
		},
	});
	assert.equal(line.getCaretLeft(4), 30);
	dom.window.close();
});

function rectangle(left: number, top: number, width: number): DOMRect {
	return { left, top, width, height: 20, right: left + width, bottom: top + 20, x: left, y: top, toJSON: () => ({}) } as DOMRect;
}
