import assert from 'node:assert/strict';
import test from 'node:test';
import { JSDOM } from 'jsdom';
import { h } from '../../../base/browser/dom.js';
import { ElementSizeObserver } from '../../browser/config/elementSizeObserver.js';

test('ElementSizeObserver coalesces equal dimensions and exposes the measured width and height', () => {
	const dom = new JSDOM('<!doctype html><body></body>');
	const element = h(dom.window.document, 'div');
	const observer = new ElementSizeObserver(element, undefined);
	const sizes: Array<{ readonly width: number; readonly height: number }> = [];
	const subscription = observer.onDidChange(() => sizes.push({ width: observer.getWidth(), height: observer.getHeight() }));

	observer.observe({ width: 320, height: 180 });
	observer.observe({ width: 320, height: 180 });
	observer.observe({ width: 640, height: 360 });

	assert.deepEqual(sizes.map(size => ({ width: size.width, height: size.height })), [{ width: 320, height: 180 }, { width: 640, height: 360 }]);
	assert.deepEqual({ width: observer.getWidth(), height: observer.getHeight() }, { width: 640, height: 360 });
	observer.observe({ width: -1, height: 0 });
	assert.deepEqual({ width: observer.getWidth(), height: observer.getHeight() }, { width: 5, height: 5 });

	subscription.dispose();
	observer.dispose();
	dom.window.close();
});
