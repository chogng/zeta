import assert from 'node:assert/strict';
import test from 'node:test';
import { JSDOM } from 'jsdom';
import { createClipboardPasteEvent, InMemoryClipboardMetadataManager, readEditorClipboardText, readEditorHtmlText, type ClipboardStoredMetadata } from '../../../browser/controller/editContext/clipboardUtils.js';

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

test('paste event reads VS Code metadata and exposes an external data transfer', async () => {
	const metadata: ClipboardStoredMetadata = {
		version: 1,
		id: 'copy-id',
		isFromEmptySelection: false,
		multicursorText: ['one', 'two'],
		mode: 'typescript',
	};
	const values = new Map([
		['text/plain', 'one\ntwo'],
		['vscode-editor-data', JSON.stringify(metadata)],
		['ResourceURLs', '["file:///internal"]'],
		['application/vnd.code.uri-list', 'file:///visible'],
	]);
	const items = [...values].map(([type, value]) => ({
		kind: 'string',
		type,
		getAsString: (callback: (text: string) => void) => callback(value),
		getAsFile: () => null,
	})) as unknown as DataTransferItemList;
	const transfer = {
		types: [...values.keys()],
		files: [],
		items,
		getData: (type: string) => values.get(type) ?? '',
	} as unknown as DataTransfer;
	const browserEvent = {
		clipboardData: transfer,
		preventDefault() {},
		stopImmediatePropagation() {},
	} as ClipboardEvent;
	const event = createClipboardPasteEvent(browserEvent);
	assert.deepEqual(event.metadata, metadata);
	assert.equal(event.text, 'one\ntwo');
	const external = event.toExternalVSDataTransfer();
	assert.ok(external);
	assert.equal(await external.get('text/uri-list')?.asString(), 'file:///visible');
	assert.equal(external.has('ResourceURLs'), false);
});

test('in-memory metadata is single-source and cleared by a text mismatch', () => {
	const metadata: ClipboardStoredMetadata = {
		version: 1,
		id: undefined,
		isFromEmptySelection: true,
		multicursorText: null,
		mode: null,
	};
	const manager = new InMemoryClipboardMetadataManager();
	manager.set('copied', metadata);
	assert.equal(manager.get('other'), null);
	assert.equal(manager.get('copied'), null);
	manager.set('copied', metadata);
	assert.equal(manager.get('copied'), metadata);
});
