import assert from 'node:assert/strict';
import test from 'node:test';
import { EditorTextDirection, EditorViewLineOptions } from '../../browser/viewParts/viewLines/viewLineOptions.js';

test('EditorViewLineOptions snapshots the configuration shared by line renderers', () => {
	const options = new EditorViewLineOptions({
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

test('EditorViewLineOptions rejects invalid renderer configuration', () => {
	assert.throws(() => new EditorViewLineOptions({
		textDirection: 'diagonal' as EditorTextDirection,
		fontLigatures: false,
		useGpu: false,
		useMonospaceOptimizations: false,
		lineHeight: 20,
		tabSize: 4,
	}), /text direction/);
});

test('EditorViewLineOptions compares every renderer-owned field', () => {
	const options = new EditorViewLineOptions({ textDirection: EditorTextDirection.Auto, fontLigatures: false, useGpu: false, useMonospaceOptimizations: false, lineHeight: 20, tabSize: 4 });
	assert.equal(options.equals(new EditorViewLineOptions({ textDirection: EditorTextDirection.Auto, fontLigatures: false, useGpu: false, useMonospaceOptimizations: false, lineHeight: 20, tabSize: 4 })), true);
	assert.equal(options.equals(new EditorViewLineOptions({ textDirection: EditorTextDirection.Auto, fontLigatures: false, useGpu: false, useMonospaceOptimizations: false, lineHeight: 20, tabSize: 2 })), false);
});
