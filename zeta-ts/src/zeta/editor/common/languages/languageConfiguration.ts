export interface LanguageCharacterPair {
	readonly open: string;
	readonly close: string;
}

export type LanguageAutoClosingTokenContext = "string" | "comment";

export interface LanguageAutoClosingPair extends LanguageCharacterPair {
	readonly notIn?: readonly LanguageAutoClosingTokenContext[];
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
