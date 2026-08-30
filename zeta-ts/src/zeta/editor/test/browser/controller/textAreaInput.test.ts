import assert from 'node:assert/strict';
import test from 'node:test';
import { JSDOM } from 'jsdom';
import { EditorTextAreaInput } from '../../../browser/controller/editContext/textArea/textAreaEditContextInput.js';

test('textarea input owns direction-aware value and selection state', () => {
	const dom = new JSDOM('<textarea></textarea>');
	const element = dom.window.document.querySelector('textarea')!;
	using input = new EditorTextAreaInput(element);
	input.setValue('test', 'abcd');
	input.setSelectionRange('test', 3, 1);
	assert.equal(input.getValue(), 'abcd');
	assert.deepEqual([input.getSelectionStart(), input.getSelectionEnd()], [3, 1]);
	dom.window.close();
});
