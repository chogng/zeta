import assert from 'node:assert/strict';
import test from 'node:test';
import { isCompletionsEnabled, isCompletionsEnablement } from '../../common/services/completionsEnablement.js';

test('completion enablement resolves language overrides before the wildcard', () => {
	assert.equal(isCompletionsEnabled({ '*': true, markdown: false }, 'markdown'), false);
	assert.equal(isCompletionsEnabled({ '*': false, typescript: true }, 'typescript'), true);
	assert.equal(isCompletionsEnabled({ '*': true }, 'rust'), true);
	assert.equal(isCompletionsEnabled({}, 'rust'), false);
	assert.equal(isCompletionsEnabled(Object.create({ '*': true }) as Record<string, boolean>, 'rust'), false);
});

test('completion enablement accepts booleans and rejects malformed maps', () => {
	assert.equal(isCompletionsEnabled(true, 'rust'), true);
	assert.equal(isCompletionsEnabled(false, 'rust'), false);
	assert.equal(isCompletionsEnabled(undefined, 'rust'), false);
	assert.equal(isCompletionsEnablement({ '*': true, rust: false }), true);
	assert.equal(isCompletionsEnablement({ '*': 'yes' }), false);
	assert.equal(isCompletionsEnablement([]), false);
});
