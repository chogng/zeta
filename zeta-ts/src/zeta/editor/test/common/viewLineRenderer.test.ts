import assert from 'node:assert/strict';
import test from 'node:test';
import { Range } from '../../common/core/range.js';
import { InlineDecorationType } from '../../common/viewModel/inlineDecorations.js';
import { LineDecoration } from '../../common/viewLayout/lineDecorations.js';
import { DomPosition, RenderLineInput, renderViewLine } from '../../common/viewLayout/viewLineRenderer.js';

test('line decoration filtering uses one-based Range lines and columns', () => {
	assert.deepEqual(LineDecoration.filter([{
		range: new Range(2, 2, 3, 4),
		inlineClassName: 'detected-link',
		type: InlineDecorationType.Regular,
	}], 3, 1, 10), [
		new LineDecoration(1, 4, 'detected-link', InlineDecorationType.Regular),
	]);
});

test('line renderer maps source columns through decorated spans and tabs', () => {
	const output = renderViewLine(new RenderLineInput({
		lineContent: 'a\tb',
		tabSize: 4,
		lineDecorations: [new LineDecoration(1, 2, 'token-keyword', InlineDecorationType.Regular)],
	}));

	assert.equal(output.html, '<span class="stanza-editor-line-text"><span class="token-keyword">a</span><span>&nbsp;&nbsp;&nbsp;b</span></span>');
	assert.deepEqual([
		output.characterMapping.getDomPosition(1),
		output.characterMapping.getDomPosition(2),
		output.characterMapping.getDomPosition(3),
		output.characterMapping.getDomPosition(4),
	], [
		new DomPosition(0, 0),
		new DomPosition(1, 0),
		new DomPosition(1, 3),
		new DomPosition(1, 4),
	]);
	assert.deepEqual([1, 2, 3, 4].map(column => output.characterMapping.getHorizontalOffset(column)), [0, 1, 4, 5]);
	assert.equal(output.characterMapping.getColumn(new DomPosition(1, 3), 4), 3);
});
