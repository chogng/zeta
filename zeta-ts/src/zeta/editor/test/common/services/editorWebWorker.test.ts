import assert from 'node:assert/strict';
import test from 'node:test';
import { TextPosition, TextRange } from '../../../common/core/text.js';
import { DEFAULT_WORD_REGEXP } from '../../../common/core/wordHelper.js';
import { EDITOR_WORKER_MINIMAL_EDITS_LANE, EDITOR_WORKER_NAVIGATE_VALUE_LANE, EDITOR_WORKER_UNICODE_HIGHLIGHTS_LANE, type EditorWorkerLane, type EditorWorkerRequest } from '../../../common/services/editorWorker.js';
import { EditorWorker } from '../../../common/services/editorWebWorker.js';
import { TextModel } from '../../../common/model/textModel.js';

test('Editor worker computes Unicode highlights from the captured model version', async () => {
	using model = new TextModel('const a = 1;\u200b\nconst \u0430 = 2;\u202e');
	using worker = new EditorWorker();

	const result = await run(worker, model, 1, EDITOR_WORKER_UNICODE_HIGHLIGHTS_LANE, Object.freeze({}));

	assert.deepEqual((result as readonly { readonly kind: string }[]).map(highlight => highlight.kind), ['invisible', 'confusable', 'bidi']);
});

test('Editor worker reduces formatting replacements without changing their result', async () => {
	using model = new TextModel('This is line one');
	using worker = new EditorWorker();
	const range = TextRange.from(TextPosition.at(0, 0), TextPosition.at(0, model.length));

	const result = await run(worker, model, 1, EDITOR_WORKER_MINIMAL_EDITS_LANE, Object.freeze({ edits: [{ range, text: 'This is line One' }] }));

	assert.deepEqual(result, [{
		range: TextRange.from(TextPosition.at(0, 13), TextPosition.at(0, 14)),
		text: 'O',
	}]);
});

test('Editor worker navigates values at an empty selection through the enclosing word', async () => {
	using model = new TextModel('const enabled = true;');
	using worker = new EditorWorker();
	const start = model.getText().indexOf('true');

	const result = await run(worker, model, 1, EDITOR_WORKER_NAVIGATE_VALUE_LANE, Object.freeze({
		range: TextRange.emptyAt(TextPosition.at(0, start)),
		up: true,
		wordDefinition: DEFAULT_WORD_REGEXP,
	}));

	assert.deepEqual(result, {
		range: TextRange.from(TextPosition.at(0, start), TextPosition.at(0, start + 4)),
		value: 'false',
	});
});

test('Editor worker navigates an explicitly selected number without a matching word pattern', async () => {
	using model = new TextModel('version 2');
	using worker = new EditorWorker();
	const result = await run(worker, model, 1, EDITOR_WORKER_NAVIGATE_VALUE_LANE, Object.freeze({
		range: TextRange.from(TextPosition.at(0, 8), TextPosition.at(0, 9)),
		up: true,
		wordDefinition: /[A-Za-z]+/g,
	}));

	assert.deepEqual(result, {
		range: TextRange.from(TextPosition.at(0, 8), TextPosition.at(0, 9)),
		value: '3',
	});
});

function run(worker: EditorWorker, model: TextModel, requestId: number, lane: EditorWorkerLane, payload: EditorWorkerRequest): ReturnType<EditorWorker['run']> {
	return worker.run(Object.freeze({ requestId, lane, payload, snapshot: model.createSnapshot() }), new AbortController().signal);
}
