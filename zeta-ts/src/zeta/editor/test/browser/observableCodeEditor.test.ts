import assert from 'node:assert/strict';
import test from 'node:test';
import { JSDOM } from 'jsdom';
import { autorun } from '../../../base/common/observable.js';
import { EditorSelectionController } from '../../common/cursor/cursor.js';
import { TextSelection, TextSelectionSet } from '../../common/core/selection.js';
import { TextPosition } from '../../common/core/text.js';
import { TextModel } from '../../common/model/textModel.js';

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

const { observableCodeEditor } = await import('../../browser/observableCodeEditor.js');
const { CodeEditorWidget } = await import('../../browser/widget/codeEditor/codeEditorWidget.js');

test.after(() => browserEnvironment.window.close());

test('observable code editor tracks canonical model, selections, and layout', () => {
	const dom = new JSDOM('<!doctype html><body><main></main></body>');
	dom.window.HTMLCanvasElement.prototype.getContext = () => null;
	const container = dom.window.document.querySelector<HTMLElement>('main')!;
	using model = new TextModel('alpha');
	using selections = new EditorSelectionController(model, TextSelectionSet.single(TextSelection.collapsedAt(TextPosition.at(0, 0))));
	using editor = new CodeEditorWidget({ container, model, selectionController: selections, lineHeight: 20 });
	using observableEditor = observableCodeEditor(editor);

	assert.strictEqual(observableCodeEditor(editor), observableEditor);
	assert.equal(observableEditor.value.get(), 'alpha');
	assert.equal(observableEditor.cursorLineNumber.get(), 1);

	const observedValues: string[] = [];
	using reaction = autorun(reader => {
		observedValues.push(`${observableEditor.versionId.read(reader)}:${observableEditor.value.read(reader)}`);
	});

	model.reset('beta');
	assert.equal(observableEditor.value.get(), 'beta');
	assert.equal(observableEditor.valueIsEmpty.get(), false);

	selections.setSelections(TextSelectionSet.single(TextSelection.collapsedAt(TextPosition.at(0, 2))));
	assert.equal(observableEditor.cursorPosition.get().columnIndex, 2);

	observableEditor.value.set('');
	assert.equal(model.getText(), '');
	assert.equal(observableEditor.valueIsEmpty.get(), true);
	assert.equal(observedValues.at(-1)?.endsWith(':'), true);

	editor.layout({ width: 320, height: 80 });
	assert.equal(observableEditor.layoutInfoWidth.get(), 320);
	assert.equal(observableEditor.layoutInfoHeight.get(), 80);
	assert.equal(observableEditor.domNode.get(), editor.element);

	dom.window.close();
});
