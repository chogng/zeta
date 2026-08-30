import assert from 'node:assert/strict';
import test from 'node:test';
import { clampOffset, createNativeTextWindow, isNativeTextUpdateEvent, NATIVE_TEXT_WINDOW_LENGTH } from '../../../browser/controller/editContext/native/nativeEditContextUtils.js';

test('native edit-context offsets are clamped and validated', () => {
	assert.equal(clampOffset(-4, 10), 0);
	assert.equal(clampOffset(20, 10), 10);
	assert.equal(isNativeTextUpdateEvent({ text: 'x', updateRangeStart: 0, updateRangeEnd: 1 }), true);
	assert.equal(isNativeTextUpdateEvent({ text: 'x', updateRangeStart: -1, updateRangeEnd: 1 }), false);
});

test('native text window remains bounded around the selection', () => {
	const text = 'a'.repeat(NATIVE_TEXT_WINDOW_LENGTH * 2);
	const window = createNativeTextWindow(text, NATIVE_TEXT_WINDOW_LENGTH, NATIVE_TEXT_WINDOW_LENGTH);
	assert.equal(window.endOffset - window.startOffset, NATIVE_TEXT_WINDOW_LENGTH);
	assert.ok(window.startOffset <= NATIVE_TEXT_WINDOW_LENGTH && window.endOffset >= NATIVE_TEXT_WINDOW_LENGTH);
});
