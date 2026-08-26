import assert from 'node:assert/strict';
import test from 'node:test';
import { JSDOM } from 'jsdom';
import { EditorDom } from '../../browser/editorDom.js';

test('EditorDom owns stable roots, layout writes, and attachment lifecycle', () => {
	const dom = new JSDOM('<!doctype html><body><main></main></body>');
	const parent = dom.window.document.querySelector<HTMLElement>('main');
	assert.ok(parent);
	const editorDom = new EditorDom({
		rootClassName: 'editor-root',
		contentClassName: 'editor-content',
	});
	editorDom.attach(parent);
	editorDom.layout({ width: -10, height: 240 });

	assert.equal(parent.firstElementChild, editorDom.domNode);
	assert.equal(editorDom.domNode.className, 'editor-root');
	assert.equal(editorDom.contentDomNode.className, 'editor-content');
	assert.equal(editorDom.domNode.style.width, '0px');
	assert.equal(editorDom.domNode.style.height, '240px');
	assert.equal(editorDom.contentDomNode.style.width, '0px');
	assert.equal(editorDom.contentDomNode.style.height, '240px');
	assert.throws(() => editorDom.attach(parent), ReferenceError);

	editorDom.dispose();
	assert.equal(parent.contains(editorDom.domNode), false);
	dom.window.close();
});
