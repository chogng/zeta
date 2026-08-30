import assert from 'node:assert/strict';
import test from 'node:test';
import { isCompletionsEnablementEnabled, isCompletionsEnablement } from '../../common/services/ownedCompletionsEnablement.js';

test('completion enablement resolves language overrides before the wildcard', () => {
	assert.equal(isCompletionsEnablementEnabled({ '*': true, markdown: false }, 'markdown'), false);
	assert.equal(isCompletionsEnablementEnabled({ '*': false, typescript: true }, 'typescript'), true);
	assert.equal(isCompletionsEnablementEnabled({ '*': true }, 'rust'), true);
	assert.equal(isCompletionsEnablementEnabled({}, 'rust'), false);
	assert.equal(isCompletionsEnablementEnabled(Object.create({ '*': true }) as Record<string, boolean>, 'rust'), false);
});

test('completion enablement accepts booleans and rejects malformed maps', () => {
	assert.equal(isCompletionsEnablementEnabled(true, 'rust'), true);
	assert.equal(isCompletionsEnablementEnabled(false, 'rust'), false);
	assert.equal(isCompletionsEnablementEnabled(undefined, 'rust'), false);
	assert.equal(isCompletionsEnablement({ '*': true, rust: false }), true);
	assert.equal(isCompletionsEnablement({ '*': 'yes' }), false);
	assert.equal(isCompletionsEnablement([]), false);
});
