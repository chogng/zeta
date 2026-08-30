export interface LanguageCharacterPair {
	readonly open: string;
	readonly close: string;
}

export type LanguageAutoClosingTokenContext = "string" | "comment";

export interface LanguageAutoClosingPair extends LanguageCharacterPair {
	readonly notIn?: readonly LanguageAutoClosingTokenContext[];
}

/** Normalized auto-closing pair consumed by cursor editing operations. */
export class StandardAutoClosingPairConditional {
	public readonly open: string;
	public readonly close: string;
	public readonly notIn: readonly LanguageAutoClosingTokenContext[];

	constructor(source: LanguageAutoClosingPair) {
		this.open = source.open;
		this.close = source.close;
		this.notIn = Object.freeze([...(source.notIn ?? [])]);
	}
}

/** Indexes auto-closing pairs by both sides of their opening and closing text. */
export class AutoClosingPairs {
	public readonly autoClosingPairsOpenByStart = new Map<string, StandardAutoClosingPairConditional[]>();
	public readonly autoClosingPairsOpenByEnd = new Map<string, StandardAutoClosingPairConditional[]>();
	public readonly autoClosingPairsCloseByStart = new Map<string, StandardAutoClosingPairConditional[]>();
	public readonly autoClosingPairsCloseByEnd = new Map<string, StandardAutoClosingPairConditional[]>();
	public readonly autoClosingPairsCloseSingleChar = new Map<string, StandardAutoClosingPairConditional[]>();

	constructor(autoClosingPairs: readonly LanguageAutoClosingPair[]) {
		for (const source of autoClosingPairs) {
			const pair = new StandardAutoClosingPairConditional(source);
			appendPair(this.autoClosingPairsOpenByStart, pair.open.charAt(0), pair);
			appendPair(this.autoClosingPairsOpenByEnd, pair.open.charAt(pair.open.length - 1), pair);
			appendPair(this.autoClosingPairsCloseByStart, pair.close.charAt(0), pair);
			appendPair(this.autoClosingPairsCloseByEnd, pair.close.charAt(pair.close.length - 1), pair);
			if (pair.open.length === 1 && pair.close.length === 1) appendPair(this.autoClosingPairsCloseSingleChar, pair.close, pair);
		}
	}
}

function appendPair(target: Map<string, StandardAutoClosingPairConditional[]>, key: string, pair: StandardAutoClosingPairConditional): void {
	const current = target.get(key);
	if (current) current.push(pair);
	else target.set(key, [pair]);
}

export interface LanguageCommentConfiguration {
	readonly lineComment?: string | null;
	readonly blockComment?: LanguageCharacterPair | null;
}

export enum LanguageIndentAction {
	None = "none",
	Indent = "indent",
	IndentOutdent = "indentOutdent",
	Outdent = "outdent",
}

export interface LanguageEnterAction {
	readonly indentAction: LanguageIndentAction;
	readonly appendText?: string;
	readonly removeText?: number;
}

export interface LanguageOnEnterRule {
	readonly beforeText: RegExp;
	readonly afterText?: RegExp;
	readonly previousLineText?: RegExp;
	readonly action: LanguageEnterAction;
}

export interface LanguageIndentationRules {
	readonly decreaseIndentPattern: RegExp;
	readonly increaseIndentPattern: RegExp;
	readonly indentNextLinePattern?: RegExp | null;
	readonly unIndentedLinePattern?: RegExp | null;
}

export interface LanguageFoldingMarkers {
	readonly start: RegExp;
	readonly end: RegExp;
}

export interface FoldingMarkers {
	start: RegExp;
	end: RegExp;
}

export interface FoldingRules {
	offSide?: boolean;
	markers?: FoldingMarkers;
}

/** DOM-free editing rules contributed for one language. */
export interface LanguageConfigurationInput {
	readonly comments?: LanguageCommentConfiguration | null;
	readonly brackets?: readonly LanguageCharacterPair[] | null;
	readonly autoClosingPairs?: readonly LanguageAutoClosingPair[] | null;
	readonly surroundingPairs?: readonly LanguageCharacterPair[] | null;
	readonly autoCloseBefore?: string | null;
	readonly indentationRules?: LanguageIndentationRules | null;
	readonly foldingMarkers?: LanguageFoldingMarkers | null;
	readonly onEnterRules?: readonly LanguageOnEnterRule[] | null;
	readonly wordPattern?: RegExp | null;
}

export const DEFAULT_LANGUAGE_AUTO_CLOSE_BEFORE = "\"'`;:.,=}])> \n\t";
