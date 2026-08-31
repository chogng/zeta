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
	ResizeObserver: class TestResizeObserver { observe(): void {} unobserve(): void {} disconnect(): void {} },
})) Object.defineProperty(globalThis, name, { configurable: true, value });

const { CodeEditorWidget } = await import('../../../../browser/widget/codeEditor/codeEditorWidget.js');
const { EditorExtensionsRegistry } = await import('../../../../browser/editorExtensions.js');
const { CursorUndoRedoController } = await import('../../browser/cursorUndo.js');

test.after(() => environment.window.close());

test('CursorUndoRedoController restores and reapplies cursor-only history', () => {
	const dom = new JSDOM('<!doctype html><body><main></main></body>');
	dom.window.HTMLCanvasElement.prototype.getContext = () => null;
	const container = dom.window.document.querySelector<HTMLElement>('main')!;
	using model = new TextModel('one\ntwo');
	using editor = new CodeEditorWidget({ container, model, input: { resource: model.uri }, languageId: model.getLanguageId(), lineHeight: 20 });
	const original = [Selection.fromPositions(new Position(1, 1))];
	const multiple = [Selection.fromPositions(new Position(2, 1)), Selection.fromPositions(new Position(1, 1))];
	editor.selections.setCursorSelections(multiple);

	const undo = new dom.window.KeyboardEvent('keydown', { bubbles: true, cancelable: true, key: 'u', ctrlKey: true });
	editor.view.element.dispatchEvent(undo);
	assert.equal(undo.defaultPrevented, true);
	assert.deepEqual(editor.getSelections(), original);
	assert.ok(CursorUndoRedoController.get(editor));

	const redo = [...EditorExtensionsRegistry.getEditorActions()].find(action => action.id === 'cursorRedo');
	assert.ok(redo);
	editor.invokeWithinContext(accessor => redo.run(accessor, editor, {}));
	assert.deepEqual(editor.getSelections(), multiple);
	assert.equal(model.getText(), 'one\ntwo');
	dom.window.close();
});
