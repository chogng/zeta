import assert from 'node:assert/strict';
import test from 'node:test';
import { commonPrefixLength, commonSuffixLength, escapeRegExpCharacters } from '../../common/strings.js';

test('escapeRegExpCharacters turns arbitrary text into a literal pattern', () => {
	const value = 'a.*(b)+[c]?\\d^$|{}';
	assert.equal(new RegExp(`^${escapeRegExpCharacters(value)}$`, 'u').test(value), true);
});

test('string common lengths use UTF-16 code-unit identity', () => {
	assert.equal(commonPrefixLength('prefix-left', 'prefix-right'), 7);
	assert.equal(commonSuffixLength('left-suffix', 'right-suffix'), 8);
	assert.equal(commonPrefixLength('', 'value'), 0);
	assert.equal(commonSuffixLength('value', ''), 0);
});
