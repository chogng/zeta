import { Emitter, type Event } from '../../../base/common/event.js';
import { Disposable, toDisposable } from '../../../base/common/lifecycle.js';
import { Position } from '../core/position.js';
import { Range } from '../core/range.js';
import { type TextModel } from '../model/textModel.js';
import { type LanguageLexicalBracketEvent } from './languageLexicalLineScanner.js';
import { type LanguageStructuralBracketSource } from './languageLexicalContext.js';

export interface LanguageBracketPair {
	readonly opening: Range;
	readonly closing: Range;
}

export interface LanguageBracketInfo {
	readonly range: Range;
	readonly token: string;
	readonly action: 'open' | 'close';
	readonly nestingLevel: number;
	readonly nestingLevelOfEqualBracketType: number;
	readonly isInvalid: boolean;
	readonly pair?: LanguageBracketPair;
}

interface MutableBracketInfo {
	readonly range: Range;
	readonly token: string;
	readonly action: 'open' | 'close';
	readonly nestingLevel: number;
	readonly nestingLevelOfEqualBracketType: number;
	isInvalid: boolean;
	pair?: LanguageBracketPair;
}

interface OpenBracket {
	readonly event: LanguageLexicalBracketEvent;
	readonly info: MutableBracketInfo;
}

interface BracketPairState {
	readonly modelVersion: number;
	readonly brackets: readonly LanguageBracketInfo[];
	readonly bracketsByLine: readonly (readonly LanguageBracketInfo[])[];
}

/** Owns the model-wide structural bracket index used by matching and colorization. */
export class LanguageBracketPairs extends Disposable {
	private readonly changeEmitter = this._register(new Emitter<void>());
	private state: BracketPairState | undefined;

	readonly onDidChange: Event<void> = this.changeEmitter.event;

	constructor(readonly textModel: TextModel, private readonly source: LanguageStructuralBracketSource) {
		super();
		if (source.textModel !== textModel) {
			this.dispose();
			throw new TypeError('Language bracket pairs require a structural source for their text model');
		}
		this._register(source.onDidChange(() => {
			this.state = undefined;
			this.changeEmitter.fire();
		}));
		this._register(toDisposable(() => {
			this.state = undefined;
		}));
	}

	matchBracket(position: Position): LanguageBracketPair | undefined {
		this.assertNotDisposed();
		this.textModel.offsetAt(position);
		return this.findBracketAt(position)?.pair;
	}

	findEnclosingBrackets(position: Position): LanguageBracketPair | undefined {
		this.assertNotDisposed();
		this.textModel.offsetAt(position);
		let result: LanguageBracketPair | undefined;
		for (const bracket of this.ensureState().brackets) {
			if (bracket.action !== 'open' || !bracket.pair) continue;
			if (Position.compare(bracket.pair.opening.getStartPosition(), position) >= 0 || Position.compare(bracket.pair.closing.getEndPosition(), position) <= 0) continue;
			if (!result || Position.compare(bracket.pair.opening.getStartPosition(), result.opening.getStartPosition()) > 0) result = bracket.pair;
		}
		return result;
	}

	findNextBracket(position: Position): LanguageBracketInfo | undefined {
		this.assertNotDisposed();
		this.textModel.offsetAt(position);
		return this.ensureState().brackets.find(bracket => Position.compare(bracket.range.getStartPosition(), position) >= 0);
	}

	getLineBrackets(lineIndex: number): readonly LanguageBracketInfo[] {
		this.assertNotDisposed();
		if (!Number.isSafeInteger(lineIndex) || lineIndex < 0 || lineIndex >= this.textModel.lineCount) {
			throw new RangeError('Language bracket line is outside the text model');
		}
		return this.ensureState().bracketsByLine[lineIndex]!;
	}

	getBracketPairsInLineRange(startLineIndex: number, endLineIndexInclusive: number): readonly LanguageBracketInfo[] {
		this.assertNotDisposed();
		if (!Number.isSafeInteger(startLineIndex) || !Number.isSafeInteger(endLineIndexInclusive) || startLineIndex < 0 || endLineIndexInclusive < startLineIndex || endLineIndexInclusive >= this.textModel.lineCount) {
			throw new RangeError('Language bracket pair range is outside the text model');
		}
		return Object.freeze(this.ensureState().brackets.filter(bracket => bracket.action === 'open' && bracket.pair
			&& bracket.pair.opening.startLineNumber - 1 <= endLineIndexInclusive
			&& bracket.pair.closing.endLineNumber - 1 >= startLineIndex));
	}

	private findBracketAt(position: Position): LanguageBracketInfo | undefined {
		const brackets = this.ensureState().bracketsByLine[position.lineNumber - 1]!;
		const contained = brackets.find(bracket => bracket.range.getStartPosition().column <= position.column && position.column < bracket.range.getEndPosition().column);
		if (contained) return contained;
		for (let index = brackets.length - 1; index >= 0; index -= 1) {
			if (brackets[index]!.range.getEndPosition().column === position.column) return brackets[index];
		}
		return undefined;
	}

	private ensureState(): BracketPairState {
		if (this.state?.modelVersion === this.textModel.version) return this.state;
		const mutableBrackets: MutableBracketInfo[] = [];
		const mutableByLine: MutableBracketInfo[][] = Array.from({ length: this.textModel.lineCount }, () => []);
		const stack: OpenBracket[] = [];
		for (let lineIndex = 0; lineIndex < this.textModel.lineCount; lineIndex += 1) {
			for (const event of this.source.getStructuralBracketEvents(lineIndex)) {
				const range = Range.fromPositions(new Position((lineIndex) + 1, (event.startColumn) + 1), new Position((lineIndex) + 1, (event.endColumn) + 1));
				if (event.action === 'open') {
					const info: MutableBracketInfo = {
						range,
						token: event.token,
						action: event.action,
						nestingLevel: stack.length,
						nestingLevelOfEqualBracketType: stack.filter(open => open.event.token === event.token).length,
						isInvalid: false,
					};
					mutableBrackets.push(info);
					mutableByLine[lineIndex]!.push(info);
					stack.push({ event, info });
					continue;
				}
				let openingIndex = -1;
				for (let index = stack.length - 1; index >= 0; index -= 1) {
					if (stack[index]!.event.matchingToken === event.token) {
						openingIndex = index;
						break;
					}
				}
				const opening = openingIndex >= 0 ? stack[openingIndex]! : undefined;
				const info: MutableBracketInfo = {
					range,
					token: event.token,
					action: event.action,
					nestingLevel: opening ? opening.info.nestingLevel : stack.length,
					nestingLevelOfEqualBracketType: opening ? opening.info.nestingLevelOfEqualBracketType : stack.filter(open => open.event.matchingToken === event.token).length,
					isInvalid: opening === undefined,
				};
				mutableBrackets.push(info);
				mutableByLine[lineIndex]!.push(info);
				if (!opening) continue;
				for (let index = stack.length - 1; index > openingIndex; index -= 1) stack[index]!.info.isInvalid = true;
				stack.length = openingIndex;
				const pair = Object.freeze({ opening: opening.info.range, closing: info.range });
				opening.info.pair = pair;
				info.pair = pair;
			}
		}
		const frozenByMutable = new Map<MutableBracketInfo, LanguageBracketInfo>();
		for (const bracket of mutableBrackets) frozenByMutable.set(bracket, Object.freeze({ ...bracket }));
		this.state = Object.freeze({
			modelVersion: this.textModel.version,
			brackets: Object.freeze(mutableBrackets.map(bracket => frozenByMutable.get(bracket)!)),
			bracketsByLine: Object.freeze(mutableByLine.map(line => Object.freeze(line.map(bracket => frozenByMutable.get(bracket)!)))),
		});
		return this.state;
	}
}
