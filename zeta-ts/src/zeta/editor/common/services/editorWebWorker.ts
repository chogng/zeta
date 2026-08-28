import { AbstractDisposable } from '../../../base/common/lifecycle.js';
import { TextPosition, TextRange, normalizeTextLineEndings, type TextEdit, type TextSnapshot } from '../core/text.js';
import { StringText } from '../core/text/abstractText.js';
import { TextReplacement } from '../core/edits/textEdit.js';
import { getWordAtText } from '../core/wordHelper.js';
import { BasicInplaceReplace } from '../languages/inplaceReplaceSupport.js';
import { type LanguageWorkerRequest } from '../languages/languageRequestCoordinator.js';
import { EDITOR_WORKER_MINIMAL_EDITS_LANE, EDITOR_WORKER_NAVIGATE_VALUE_LANE, EDITOR_WORKER_UNICODE_HIGHLIGHTS_LANE, type EditorWorkerImplementation, type EditorWorkerLane, type EditorWorkerMinimalEditsRequest, type EditorWorkerNavigateValueRequest, type EditorWorkerRequest, type EditorWorkerResult } from './editorWorker.js';
import { computeUnicodeHighlights } from './unicodeTextModelHighlighter.js';

const MINIMAL_EDIT_LIMIT = 100_000;

/** Executes model-versioned editor computations inside a dedicated Worker or in-process host. */
export class EditorWorker extends AbstractDisposable implements EditorWorkerImplementation {
	public async run(request: LanguageWorkerRequest<EditorWorkerLane, EditorWorkerRequest>, signal: AbortSignal): Promise<EditorWorkerResult> {
		this.assertNotDisposed();
		signal.throwIfAborted();
		switch (request.lane) {
			case EDITOR_WORKER_UNICODE_HIGHLIGHTS_LANE:
				return computeUnicodeHighlights(request.snapshot, signal);
			case EDITOR_WORKER_MINIMAL_EDITS_LANE:
				return computeMoreMinimalEdits(request.snapshot, request.payload as EditorWorkerMinimalEditsRequest, signal);
			case EDITOR_WORKER_NAVIGATE_VALUE_LANE:
				return navigateValueSet(request.snapshot, request.payload as EditorWorkerNavigateValueRequest);
		}
	}

	protected disposeCore(): void {}
}

function computeMoreMinimalEdits(snapshot: TextSnapshot, request: EditorWorkerMinimalEditsRequest, signal: AbortSignal): readonly TextEdit[] {
	const document = new StringText(snapshot.getText());
	const edits = mergeAdjacentEdits(request.edits);
	const result: TextEdit[] = [];
	for (const edit of edits) {
		signal.throwIfAborted();
		const text = normalizeTextLineEndings(edit.text);
		const original = document.getValueOfRange(edit.range);
		if (original === text) continue;
		if (Math.max(original.length, text.length) > MINIMAL_EDIT_LIMIT) {
			result.push(Object.freeze({ range: edit.range, text }));
			continue;
		}
		const replacement = new TextReplacement(edit.range, text).removeCommonPrefixAndSuffix(document);
		if (!replacement.isEmpty) result.push(Object.freeze({ range: replacement.range, text: replacement.text }));
	}
	return Object.freeze(result);
}

function mergeAdjacentEdits(edits: readonly TextEdit[]): readonly TextEdit[] {
	const sorted = edits.map(edit => Object.freeze({ range: TextRange.from(edit.range.start, edit.range.end), text: edit.text })).sort((left, right) => left.range.start.compareTo(right.range.start));
	const result: TextEdit[] = [];
	for (const edit of sorted) {
		const previous = result.at(-1);
		if (previous && previous.range.end.equals(edit.range.start)) {
			result[result.length - 1] = Object.freeze({ range: previous.range.plusRange(edit.range), text: previous.text + edit.text });
			continue;
		}
		if (previous && previous.range.end.isAfter(edit.range.start)) throw new RangeError('Editor worker edits must not overlap');
		result.push(edit);
	}
	return result;
}

function navigateValueSet(snapshot: TextSnapshot, request: EditorWorkerNavigateValueRequest): EditorWorkerResult {
	const document = new StringText(snapshot.getText());
	const range = validateRange(document, request.range);
	const selectionRange = range.empty && range.end.columnIndex < document.getLineLength(range.end.lineIndex)
		? TextRange.from(range.start, TextPosition.at(range.end.lineIndex, range.end.columnIndex + 1))
		: range;
	const selectionText = document.getValueOfRange(selectionRange);
	const line = document.getLineAt(range.start.lineIndex);
	const word = getWordAtText(range.start.columnIndex, request.wordDefinition, line)
		?? (range.start.columnIndex > 0 ? getWordAtText(range.start.columnIndex - 1, request.wordDefinition, line) : undefined);
	const wordRange = word ? TextRange.from(
		TextPosition.at(range.start.lineIndex, word.startColumnIndex),
		TextPosition.at(range.start.lineIndex, word.endColumnIndexExclusive),
	) : undefined;
	return BasicInplaceReplace.instance.navigateValueSet(selectionRange, selectionText, wordRange, word?.word, request.up);
}

function validateRange(document: StringText, range: TextRange): TextRange {
	const lifted = TextRange.from(range.start, range.end);
	document.getValueOfRange(lifted);
	return lifted;
}
