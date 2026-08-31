import assert from 'node:assert/strict';
import test from 'node:test';
import { JSDOM } from 'jsdom';
import { TextAreaInput } from '../../../browser/controller/editContext/textArea/textAreaEditContextInput.js';

test('textarea input owns direction-aware value and selection state', () => {
	const dom = new JSDOM('<textarea></textarea>');
	const element = dom.window.document.querySelector('textarea')!;
	using input = new TextAreaInput(element);
	input.setValue('test', 'abcd');
	input.setSelectionRange('test', 3, 1);
	assert.equal(input.getValue(), 'abcd');
	assert.deepEqual([input.getSelectionStart(), input.getSelectionEnd()], [3, 1]);
	dom.window.close();
});

test('textarea input connects each DOM event once and disconnects on dispose', () => {
	const dom = new JSDOM('<textarea></textarea>');
	const element = dom.window.document.querySelector('textarea')!;
	const input = new TextAreaInput(element);
	const keys: string[] = [];
	let copies = 0;
	input.onDidKeydown(event => keys.push(`down:${event.key}`));
	input.onDidKeyup(event => keys.push(`up:${event.key}`));
	input.onDidCopy(() => copies++);
	input.connect();
	input.connect();

	element.dispatchEvent(new dom.window.KeyboardEvent('keydown', { key: 'A' }));
	element.dispatchEvent(new dom.window.KeyboardEvent('keyup', { key: 'A' }));
	element.dispatchEvent(new dom.window.Event('copy'));
	assert.deepEqual(keys, ['down:A', 'up:A']);
	assert.equal(copies, 1);

	input.dispose();
	element.dispatchEvent(new dom.window.KeyboardEvent('keydown', { key: 'B' }));
	assert.deepEqual(keys, ['down:A', 'up:A']);
	dom.window.close();
});

test('textarea input publishes normalized composition state', () => {
	const dom = new JSDOM('<textarea></textarea>');
	const element = dom.window.document.querySelector('textarea')!;
	using input = new TextAreaInput(element);
	let composition: { readonly data: string; readonly text: string } | undefined;
	input.onDidCompositionUpdate(event => composition = event);
	input.connect();
	input.setValue('composition', 'first\nsecond');
	input.setSelectionRange('composition', 5, 5);
	element.dispatchEvent(new dom.window.CompositionEvent('compositionupdate', { data: 'x' }));
	assert.deepEqual(composition && { data: composition.data, text: composition.text }, {
		data: 'x',
		text: 'first\nsecond',
	});
	dom.window.close();
});

test('textarea input owns focus and reports the resulting focus state', () => {
	const dom = new JSDOM('<body><textarea></textarea></body>');
	const element = dom.window.document.querySelector('textarea')!;
	using input = new TextAreaInput(element);
	input.connect();
	input.focusTextArea();
	assert.equal(dom.window.document.activeElement, element);
	assert.equal(input.isFocused(), true);
	dom.window.close();
});
