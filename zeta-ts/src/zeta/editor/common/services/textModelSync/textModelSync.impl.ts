import { isNonEmptyArray } from '../../../../base/common/arrays.js';
import { CharCode } from '../../../../base/common/charCode.js';
import { isNonNegativeSafeInteger, isPositiveSafeInteger } from '../../../../base/common/numbers.js';
import { normalizeTextLineEndings, type TextSnapshot } from '../../core/textChange.js';
import type { TextBuffer } from '../../model/textBuffer.js';
import { createTextBuffer } from '../../model/textBufferFactory.js';
import type { LanguageWorkerDocumentChange } from './textModelSync.protocol.js';

/** Single-document Piece Tree mirror owned by one language-worker server. */
export class LanguageWorkerDocumentMirror {
	private readonly buffer: TextBuffer;
	private versionValue: number;

	constructor(snapshot: TextSnapshot) {
		assertPositiveSafeInteger(snapshot.version, 'Language worker mirror version');
		const text = snapshot.getText();
		if (text.length !== snapshot.length || countLines(text) !== snapshot.lineCount || normalizeTextLineEndings(text) !== text) {
			throw new Error('Language worker mirror snapshot metadata is inconsistent');
		}
		this.versionValue = snapshot.version;
		this.buffer = createTextBuffer(text);
	}

	public get version(): number {
		return this.versionValue;
	}

	public get length(): number {
		return this.buffer.length;
	}

	public get lineCount(): number {
		return this.buffer.lineCount;
	}

	public createSnapshot(): TextSnapshot {
		const version = this.versionValue;
		const snapshot = this.buffer.createSnapshot();
		return Object.freeze({
			version,
			length: snapshot.length,
			lineCount: snapshot.lineCount,
			getText: () => snapshot.getText(),
			getTextBetweenOffsets: (startOffset: number, endOffset: number) => snapshot.getTextBetweenOffsets(startOffset, endOffset),
		});
	}

	public synchronize(previousVersion: number, modelVersion: number, changes: readonly LanguageWorkerDocumentChange[]): void {
		if (previousVersion !== this.versionValue || modelVersion !== this.versionValue + 1) {
			throw new Error('Language worker sync version does not follow its document mirror');
		}
		if (!isNonEmptyArray(changes)) {
			throw new RangeError('Language worker sync must contain changes');
		}
		let previousStart = -1;
		let previousEnd = 0;
		for (const change of changes) {
			assertNonNegativeSafeInteger(change.rangeOffset, 'Language worker sync range offset');
			assertNonNegativeSafeInteger(change.rangeLength, 'Language worker sync range length');
			if (typeof change.text !== 'string' || normalizeTextLineEndings(change.text) !== change.text) {
				throw new TypeError('Language worker sync text must use normalized LF line endings');
			}
			const end = change.rangeOffset + change.rangeLength;
			const ambiguousSharedStart = change.rangeOffset === previousStart && (change.rangeLength === 0 || previousEnd === previousStart);
			if (change.rangeOffset < previousEnd || ambiguousSharedStart || end > this.buffer.length) {
				throw new RangeError('Language worker sync ranges must be ordered, non-overlapping, and inside the mirror');
			}
			previousStart = change.rangeOffset;
			previousEnd = end;
		}
		for (let index = changes.length - 1; index >= 0; index -= 1) {
			const change = changes[index]!;
			this.buffer.replace(change.rangeOffset, change.rangeOffset + change.rangeLength, change.text);
		}
		this.versionValue = modelVersion;
	}
}

function assertPositiveSafeInteger(value: unknown, owner: string): asserts value is number {
	if (!isPositiveSafeInteger(value)) {
		throw new RangeError(`${owner} must be a positive safe integer`);
	}
}

function assertNonNegativeSafeInteger(value: unknown, owner: string): asserts value is number {
	if (!isNonNegativeSafeInteger(value)) {
		throw new RangeError(`${owner} must be a non-negative safe integer`);
	}
}

function countLines(text: string): number {
	let result = 1;
	for (let index = 0; index < text.length; index += 1) {
		if (text.charCodeAt(index) === CharCode.LineFeed) result += 1;
	}
	return result;
}
