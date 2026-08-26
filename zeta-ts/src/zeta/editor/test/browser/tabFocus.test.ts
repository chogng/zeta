import assert from 'node:assert/strict';
import test from 'node:test';
import { TabFocus } from '../../browser/config/tabFocus.js';

test('TabFocus owns shared state and publishes only actual changes', () => {
	using tabFocus = new TabFocus();
	const changes: boolean[] = [];
	using subscription = tabFocus.onDidChange(enabled => changes.push(enabled));

	assert.equal(tabFocus.isEnabled, false);
	tabFocus.setEnabled(false);
	assert.equal(tabFocus.toggle(), true);
	tabFocus.setEnabled(true);
	tabFocus.setEnabled(false);

	assert.deepEqual(changes, [true, false]);
	assert.equal(tabFocus.isEnabled, false);
	assert.throws(() => tabFocus.setEnabled('true' as never), TypeError);
});
