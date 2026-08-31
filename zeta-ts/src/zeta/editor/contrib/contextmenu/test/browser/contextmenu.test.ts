import assert from 'node:assert/strict';
import test from 'node:test';
import { JSDOM } from 'jsdom';
import { Position } from '../../../../common/core/position.js';
import { Selection } from '../../../../common/core/selection.js';
import { TextModel } from '../../../../common/model/textModel.js';

const environment = new JSDOM('<!doctype html><body></body>');
for (const [name, value] of Object.entries({
	window: environment.window,
	document: environment.window.document,
	Node: environment.window.Node,
	Element: environment.window.Element,
	HTMLElement: environment.window.HTMLElement,
	Event: environment.window.Event,
	InputEvent: environment.window.InputEvent,
	KeyboardEvent: environment.window.KeyboardEvent,
	MouseEvent: environment.window.MouseEvent,
	ResizeObserver: class TestResizeObserver { observe(): void {} unobserve(): void {} disconnect(): void {} },
})) Object.defineProperty(globalThis, name, { configurable: true, value });

const { CodeEditorWidget } = await import('../../../../browser/widget/codeEditor/codeEditorWidget.js');
const { ContextMenuController } = await import('../../browser/contextmenu.js');

test.after(() => environment.window.close());

test('ContextMenuController opens the host menu at the active cursor from Shift+F10', () => {
	const dom = new JSDOM('<!doctype html><body><main></main></body>');
	dom.window.HTMLCanvasElement.prototype.getContext = () => null;
	const container = dom.window.document.querySelector<HTMLElement>('main')!;
	using model = new TextModel('alpha\nbeta');
	const requests: Array<{ readonly position: Position; readonly clientX: number; readonly clientY: number }> = [];
	using editor = new CodeEditorWidget({
		container,
		model,
		input: { resource: model.uri },
		languageId: model.getLanguageId(),
		lineHeight: 20,
		onShowContextMenu: request => { requests.push(request); },
	});
	editor.layout({ width: 400, height: 100 });
	editor.setSelection(Selection.fromPositions(new Position(2, 3)));
	const event = new dom.window.KeyboardEvent('keydown', { bubbles: true, cancelable: true, key: 'F10', shiftKey: true });
	editor.view.element.dispatchEvent(event);

	assert.equal(event.defaultPrevented, true);
	assert.equal(requests.length, 1);
	assert.deepEqual(requests[0]!.position, new Position(2, 3));
	assert.equal(Number.isFinite(requests[0]!.clientX), true);
	assert.equal(Number.isFinite(requests[0]!.clientY), true);
	assert.ok(ContextMenuController.get(editor));
	dom.window.close();
});
