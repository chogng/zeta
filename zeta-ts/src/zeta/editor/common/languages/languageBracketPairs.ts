import { Emitter, type Event } from '../../../base/common/event.js';
import { Disposable, toDisposable } from '../../../base/common/lifecycle.js';
import { TextPosition, TextRange } from '../core/text.js';
import { type TextModel } from '../model/textModel.js';
import { type LanguageLexicalBracketEvent } from './languageLexicalLineScanner.js';
import { type LanguageStructuralBracketSource } from './languageLexicalContext.js';

export interface LanguageBracketPair {
	readonly opening: TextRange;
	readonly closing: TextRange;
}

export interface LanguageBracketInfo {
	readonly range: TextRange;
	readonly token: string;
	readonly action: 'open' | 'close';
	readonly nestingLevel: number;
	readonly nestingLevelOfEqualBracketType: number;
	readonly isInvalid: boolean;
	readonly pair?: LanguageBracketPair;
}

interface MutableBracketInfo {
	readonly range: TextRange;
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

	matchBracket(position: TextPosition): LanguageBracketPair | undefined {
		this.assertNotDisposed();
		this.textModel.offsetAt(position);
		return this.findBracketAt(position)?.pair;
	}

	findEnclosingBrackets(position: TextPosition): LanguageBracketPair | undefined {
		this.assertNotDisposed();
		this.textModel.offsetAt(position);
		let result: LanguageBracketPair | undefined;
		for (const bracket of this.ensureState().brackets) {
			if (bracket.action !== 'open' || !bracket.pair) continue;
			if (bracket.pair.opening.start.compareTo(position) >= 0 || bracket.pair.closing.end.compareTo(position) <= 0) continue;
			if (!result || bracket.pair.opening.start.compareTo(result.opening.start) > 0) result = bracket.pair;
		}
		return result;
	}

	findNextBracket(position: TextPosition): LanguageBracketInfo | undefined {
		this.assertNotDisposed();
		this.textModel.offsetAt(position);
		return this.ensureState().brackets.find(bracket => bracket.range.start.compareTo(position) >= 0);
	}

	getLineBrackets(lineIndex: number): readonly LanguageBracketInfo[] {
		this.assertNotDisposed();
		if (!Number.isSafeInteger(lineIndex) || lineIndex < 0 || lineIndex >= this.textModel.lineCount) {
			throw new RangeError('Language bracket line is outside the text model');
		}
		return this.ensureState().bracketsByLine[lineIndex]!;
	}

	private findBracketAt(position: TextPosition): LanguageBracketInfo | undefined {
		const brackets = this.ensureState().bracketsByLine[position.lineIndex]!;
		const contained = brackets.find(bracket => bracket.range.start.columnIndex <= position.columnIndex && position.columnIndex < bracket.range.end.columnIndex);
		if (contained) return contained;
		for (let index = brackets.length - 1; index >= 0; index -= 1) {
			if (brackets[index]!.range.end.columnIndex === position.columnIndex) return brackets[index];
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
				const range = TextRange.from(TextPosition.at(lineIndex, event.startColumn), TextPosition.at(lineIndex, event.endColumn));
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
