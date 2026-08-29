import assert from 'node:assert/strict';
import test from 'node:test';
import { JSDOM } from 'jsdom';
import { h } from '../../../base/browser/dom.js';
import { applyEditorFontInfo } from '../../browser/config/domFontInfo.js';
import { resolveEditorConfiguration } from '../../browser/config/editorConfiguration.js';
import { migrateOptions } from '../../browser/config/migrateOptions.js';
import { EditorLineWrapping } from '../../common/config/editorOptions.js';
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
	const element = h(dom.window.document, 'div');
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

test('browser editor option migration converts only supported legacy shapes without mutating the caller', () => {
	const legacy = {
		wordWrap: true,
		showLineNumbers: false,
		activeLineHighlight: 'on',
		renderIndentGuides: false,
		renderWhitespace: true,
		matchBrackets: false,
		occurrencesHighlight: true,
		defaultColorDecorators: false,
	};

	assert.deepEqual(migrateOptions(legacy), {
		lineWrapping: EditorLineWrapping.On,
		lineNumbers: 'off',
		renderLineHighlight: 'line',
		guides: { indentation: false },
		renderWhitespace: 'boundary',
		matchBrackets: 'never',
		occurrencesHighlight: 'singleFile',
		defaultColorDecorators: 'never',
	});
	assert.equal(legacy.wordWrap, true);
});

test('browser editor option migration preserves current values and rejects invalid legacy values', () => {
	assert.deepEqual(migrateOptions({ wordWrap: false, lineWrapping: EditorLineWrapping.On }), {
		lineWrapping: EditorLineWrapping.On,
	});
	assert.deepEqual(migrateOptions({ lineNumbers: 'relative' }), { lineNumbers: 'relative' });
	assert.deepEqual(migrateOptions({ renderLineHighlight: 'gutter' }), { renderLineHighlight: 'gutter' });
	assert.throws(() => migrateOptions({ activeLineHighlight: 'blink' }), /highlight option is invalid/);
});
