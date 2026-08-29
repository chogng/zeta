import assert from 'node:assert/strict';
import test from 'node:test';
import { JSDOM } from 'jsdom';
import { readEditorClipboardText, readEditorHtmlText } from '../../../browser/controller/editContext/clipboardUtils.js';

test('clipboard HTML is sanitized before deterministic text extraction', () => {
	const environment = new JSDOM('<!doctype html><body></body>');
	try {
		const text = readEditorHtmlText('<div onclick="run()">one<script>bad()</script><br>two</div><style>bad</style><p>three&nbsp;four</p>', environment.window.document);
		assert.equal(text, 'one\ntwo\nthree four');
	} finally {
		environment.window.close();
	}
});

test('clipboard prefers plain text and safely converts HTML when it is absent', () => {
	const environment = new JSDOM('<!doctype html><body></body>');
	try {
		assert.equal(readEditorClipboardText({
			types: ['text/plain', 'text/html'],
			files: [],
			getData: type => type === 'text/plain' ? 'plain' : '<p>html</p>',
		}, environment.window.document), 'plain');
		assert.equal(readEditorClipboardText({
			types: ['text/html'],
			files: [],
			getData: type => type === 'text/html' ? '<p>safe<script>bad()</script></p>' : '',
		}, environment.window.document), 'safe');
	} finally {
		environment.window.close();
	}
});
