import assert from 'node:assert/strict';
import test from 'node:test';
import { JSDOM } from 'jsdom';
import { applyEditorFontInfo } from '../../browser/config/domFontInfo.js';
import { resolveEditorConfiguration } from '../../browser/config/editorConfiguration.js';
import { EDITOR_FONT_DEFAULTS } from '../../common/config/fontInfo.js';

test('browser editor configuration resolves geometry defaults at the composition boundary', () => {
	assert.deepEqual(resolveEditorConfiguration({}), {
		fontFamily: undefined,
		fontSize: EDITOR_FONT_DEFAULTS.fontSize,
		lineHeight: 20,
		fontLigatures: false,
	});
	assert.equal(resolveEditorConfiguration({ fontSize: 12 }).lineHeight, 20);
	assert.equal(resolveEditorConfiguration({ fontSize: 20 }).lineHeight, 30);
	assert.throws(() => resolveEditorConfiguration({ fontSize: 7 }), RangeError);
	assert.throws(() => resolveEditorConfiguration({ lineHeight: 81 }), RangeError);
	assert.throws(() => resolveEditorConfiguration({ fontLigatures: 'on' as never }), TypeError);
});

test('DOM font info applies the shared editor font vocabulary', () => {
	const dom = new JSDOM('<!doctype html><body></body>');
	const element = dom.window.document.createElement('div');
	applyEditorFontInfo(element, {
		fontFamily: 'Stanza Mono',
		fontSize: 15,
		fontLigatures: true,
	});

	assert.equal(element.style.fontFamily, '"Stanza Mono"');
	assert.equal(element.style.fontSize, '15px');
	assert.equal(element.style.fontVariantLigatures, 'normal');
	dom.window.close();
});
