import assert from 'node:assert/strict';
import test from 'node:test';
import { Selection } from '../../../../common/core/selection.js';
import { Position } from '../../../../common/core/position.js';
import { TextModel } from '../../../../common/model/textModel.js';
import { contentOffsetAtModelOffset, modelOffsetAtContentOffset, SimplePagedScreenReaderStrategy } from '../../../../browser/controller/editContext/screenReaderUtils.js';
import { TextAreaState, type ITextAreaWrapper } from '../../../../browser/controller/editContext/textArea/textAreaEditContextState.js';

test('screen-reader pages preserve endpoint text and model mappings', () => {
	using model = new TextModel('zero\none\ntwo\nthree');
	const selection = Selection.fromPositions(
		new Position((0) + 1, (2) + 1),
		new Position((3) + 1, (3) + 1),
	);
	const state = new SimplePagedScreenReaderStrategy().fromEditorSelection(model, selection, 1, false);

	assert.equal(state.value, 'zero\n…three');
	assert.equal(modelOffsetAtContentOffset(state, state.selectionStart), model.offsetAt(selection.getStartPosition()));
	assert.equal(modelOffsetAtContentOffset(state, state.selectionEnd, 'end'), model.offsetAt(selection.getEndPosition()));
	const omittedOffset = model.offsetAt(new Position((2) + 1, (0) + 1));
	assert.equal(
		modelOffsetAtContentOffset(state, contentOffsetAtModelOffset(state, omittedOffset, 'start'), 'start'),
		model.offsetAt(new Position((1) + 1, (0) + 1)),
	);
	assert.equal(
		modelOffsetAtContentOffset(state, contentOffsetAtModelOffset(state, omittedOffset, 'end'), 'end'),
		model.offsetAt(new Position((3) + 1, (0) + 1)),
	);

});

test('textarea state preserves direction and deduces replacement input', () => {
	const previous = new TextAreaState('ab', 2, 2, null, 0);
	const current = new TextAreaState('abX', 3, 3, null, 0);
	assert.deepEqual(TextAreaState.deduceInput(previous, current, false), {
		text: 'X',
		replacePrevCharCnt: 0,
		replaceNextCharCnt: 0,
		positionDelta: 0,
	});

	const wrapper = new MemoryTextArea();
	const backward = new TextAreaState('abcd', 4, 1, null, undefined);
	backward.writeToTextArea('test', wrapper, true);
	assert.deepEqual({
		selectionStart: wrapper.getSelectionStart(),
		selectionEnd: wrapper.getSelectionEnd(),
	}, { selectionStart: 4, selectionEnd: 1 });
});

class MemoryTextArea implements ITextAreaWrapper {
	private value = '';
	private selectionStart = 0;
	private selectionEnd = 0;

	getValue(): string {
		return this.value;
	}

	setValue(_reason: string, value: string): void {
		this.value = value;
	}

	getSelectionStart(): number {
		return this.selectionStart;
	}

	getSelectionEnd(): number {
		return this.selectionEnd;
	}

	setSelectionRange(_reason: string, selectionStart: number, selectionEnd: number): void {
		this.selectionStart = selectionStart;
		this.selectionEnd = selectionEnd;
	}
}
