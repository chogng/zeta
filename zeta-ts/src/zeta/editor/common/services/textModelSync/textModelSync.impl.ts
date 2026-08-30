import { CharCode } from '../../../../base/common/charCode.js';
import { Disposable } from '../../../../base/common/lifecycle.js';
import { isNonNegativeSafeInteger, isPositiveSafeInteger } from '../../../../base/common/numbers.js';
import { type TextSnapshot } from '../../core/textChange.js';
import { ValidAnnotatedEditOperation, type ITextBuffer } from '../../model.js';
import { createPieceTreeTextBuffer } from '../../model/textBufferFactory.js';
import type { LanguageWorkerDocumentChange } from './textModelSync.protocol.js';

/** Single-document Piece Tree mirror owned by one language-worker server. */
export class LanguageWorkerDocumentMirror extends Disposable {
	private readonly buffer: ITextBuffer;
	private versionValue: number;

	constructor(snapshot: TextSnapshot) {
		super();
		assertPositiveSafeInteger(snapshot.version, 'Language worker mirror version');
		const text = snapshot.getText();
		if (text.length !== snapshot.length || countLines(text) !== snapshot.lineCount) {
			throw new Error('Language worker mirror snapshot metadata is inconsistent');
		}
		this.versionValue = snapshot.version;
		this.buffer = this._register(createPieceTreeTextBuffer(text));
	}

	public get version(): number {
		return this.versionValue;
	}

	public get length(): number {
		return this.buffer.getLength();
	}

	public get lineCount(): number {
		return this.buffer.getLineCount();
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

	public synchronize(previousVersion: number, modelVersion: number, changes: readonly LanguageWorkerDocumentChange[], eol: '\n' | '\r\n' = this.buffer.getEOL()): void {
		if (previousVersion !== this.versionValue || modelVersion !== this.versionValue + 1) {
			throw new Error('Language worker sync version does not follow its document mirror');
		}
		if (eol !== '\n' && eol !== '\r\n') throw new TypeError('Language worker sync EOL must be LF or CRLF');
		let previousStart = -1;
		let previousEnd = 0;
		for (const change of changes) {
			assertNonNegativeSafeInteger(change.rangeOffset, 'Language worker sync range offset');
			assertNonNegativeSafeInteger(change.rangeLength, 'Language worker sync range length');
			if (typeof change.text !== 'string' || normalizeEOL(change.text, eol) !== change.text) {
				throw new TypeError('Language worker sync text must use the resulting document EOL');
			}
			const end = change.rangeOffset + change.rangeLength;
			const ambiguousSharedStart = change.rangeOffset === previousStart && (change.rangeLength === 0 || previousEnd === previousStart);
			if (change.rangeOffset < previousEnd || ambiguousSharedStart || end > this.buffer.getLength()) {
				throw new RangeError('Language worker sync ranges must be ordered, non-overlapping, and inside the mirror');
			}
			previousStart = change.rangeOffset;
			previousEnd = end;
		}
		this.buffer.applyEdits(changes.map(change => new ValidAnnotatedEditOperation(
			null,
			this.buffer.getRangeAt(change.rangeOffset, change.rangeLength),
			change.text,
			false,
			false,
			false,
		)), false, false);
		this.buffer.setEOL(eol);
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

function normalizeEOL(text: string, eol: '\n' | '\r\n'): string {
	return text.replace(/\r\n|\r|\n/g, eol);
}
