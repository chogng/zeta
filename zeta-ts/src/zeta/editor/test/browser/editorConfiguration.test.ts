import assert from 'node:assert/strict';
import test from 'node:test';
import { JSDOM } from 'jsdom';
import { h } from '../../../base/browser/dom.js';
import { applyEditorFontInfo } from '../../browser/config/domFontInfo.js';
import { resolveEditorGeometryConfiguration } from '../../browser/config/resolvedEditorGeometryConfiguration.js';
import { migrateOptions } from '../../browser/config/migrateOptions.js';
import { EDITOR_FONT_DEFAULTS } from '../../common/config/fontInfo.js';

test('browser editor configuration resolves geometry defaults at the composition boundary', () => {
	assert.deepEqual(resolveEditorGeometryConfiguration({}), {
		fontFamily: undefined,
		fontSize: EDITOR_FONT_DEFAULTS.fontSize,
		lineHeight: 20,
		fontLigatures: false,
	});
	assert.equal(resolveEditorGeometryConfiguration({ fontSize: 12 }).lineHeight, 20);
	assert.equal(resolveEditorGeometryConfiguration({ fontSize: 20 }).lineHeight, 30);
	assert.throws(() => resolveEditorGeometryConfiguration({ fontSize: 7 }), RangeError);
	assert.throws(() => resolveEditorGeometryConfiguration({ lineHeight: 81 }), RangeError);
	assert.throws(() => resolveEditorGeometryConfiguration({ fontLigatures: 'on' as never }), TypeError);
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

test('browser editor option migration updates supported legacy shapes in place', () => {
	const legacy = {
		wordWrap: true,
		lineNumbers: false,
		renderLineHighlight: true,
		renderIndentGuides: false,
		renderWhitespace: true,
		matchBrackets: false,
		occurrencesHighlight: true,
		defaultColorDecorators: false,
	};

	migrateOptions(legacy as never);
	assert.deepEqual(legacy, {
		wordWrap: 'on',
		lineNumbers: 'off',
		renderLineHighlight: 'line',
		renderIndentGuides: undefined,
		guides: { indentation: false },
		renderWhitespace: 'boundary',
		matchBrackets: 'never',
		occurrencesHighlight: 'singleFile',
		defaultColorDecorators: 'never',
	});
});

test('browser editor option migration preserves current values', () => {
	const options = { lineNumbers: 'relative', renderLineHighlight: 'gutter' };
	migrateOptions(options as never);
	assert.deepEqual(options, { lineNumbers: 'relative', renderLineHighlight: 'gutter' });
});
