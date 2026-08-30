import assert from 'node:assert/strict';
import test from 'node:test';
import { EditorTextDirection, ViewLineOptions } from '../../browser/viewParts/viewLines/viewLineOptions.js';

test('ViewLineOptions snapshots the configuration shared by line renderers', () => {
	const options = new ViewLineOptions({
		textDirection: EditorTextDirection.RightToLeft,
		fontLigatures: true,
		useGpu: true,
		useMonospaceOptimizations: false,
		lineHeight: 22,
		tabSize: 4,
	});

	assert.deepEqual({
		textDirection: options.textDirection,
		fontLigatures: options.fontLigatures,
		useGpu: options.useGpu,
		lineHeight: options.lineHeight,
		tabSize: options.tabSize,
	}, {
		textDirection: 'rtl',
		fontLigatures: true,
		useGpu: true,
		lineHeight: 22,
		tabSize: 4,
	});
});

test('ViewLineOptions rejects invalid renderer configuration', () => {
	assert.throws(() => new ViewLineOptions({
		textDirection: 'diagonal' as EditorTextDirection,
		fontLigatures: false,
		useGpu: false,
		useMonospaceOptimizations: false,
		lineHeight: 20,
		tabSize: 4,
	}), /text direction/);
});

test('ViewLineOptions compares every renderer-owned field', () => {
	const options = new ViewLineOptions({ textDirection: EditorTextDirection.Auto, fontLigatures: false, useGpu: false, useMonospaceOptimizations: false, lineHeight: 20, tabSize: 4 });
	assert.equal(options.equals(new ViewLineOptions({ textDirection: EditorTextDirection.Auto, fontLigatures: false, useGpu: false, useMonospaceOptimizations: false, lineHeight: 20, tabSize: 4 })), true);
	assert.equal(options.equals(new ViewLineOptions({ textDirection: EditorTextDirection.Auto, fontLigatures: false, useGpu: false, useMonospaceOptimizations: false, lineHeight: 20, tabSize: 2 })), false);
});
