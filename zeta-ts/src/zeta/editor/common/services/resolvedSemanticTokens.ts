import type { Event } from '../../../base/common/event.js';
import type { IDisposable } from '../../../base/common/lifecycle.js';
import type { LanguageToken } from '../tokens/languageTokens.js';
import type { TextModel } from '../model/textModel.js';

export enum SemanticTokenPresentation {
	Comment = 'token-comment',
	Keyword = 'token-keyword',
	String = 'token-string',
	Number = 'token-number',
	Regexp = 'token-regexp',
	Type = 'token-type',
	Function = 'token-function',
	Variable = 'token-variable',
	Operator = 'token-operator',
}

export enum SemanticTokenModifier {
	Declaration = 'token-modifier-declaration',
	Readonly = 'token-modifier-readonly',
	Static = 'token-modifier-static',
	Deprecated = 'token-modifier-deprecated',
	Abstract = 'token-modifier-abstract',
	Async = 'token-modifier-async',
}

export interface SemanticTokenStyling {
	readonly presentation?: SemanticTokenPresentation;
	readonly modifiers: readonly SemanticTokenModifier[];
}

export interface SemanticTokenStylingResolver {
	resolve(token: LanguageToken): SemanticTokenStyling;
}

export interface ResolvedSemanticToken {
	readonly startColumn: number;
	readonly endColumn: number;
	readonly presentation?: SemanticTokenPresentation;
	readonly modifiers?: readonly SemanticTokenModifier[];
	readonly syntaxPresentation?: LanguageToken['presentation'];
}

export interface SemanticTokenLine {
	readonly lineIndex: number;
	readonly tokens: readonly ResolvedSemanticToken[];
}

export interface SemanticTokenSource {
	readonly textModel: TextModel;
	readonly onDidChange: Event<void>;
	readonly lines: readonly SemanticTokenLine[];
	getLineTokens(lineIndex: number): readonly ResolvedSemanticToken[];
}

export interface SemanticTokenModelSource {
	readonly textModel: TextModel;
	readonly onDidChange: (listener: (...args: any[]) => void) => IDisposable;
	readonly lines: readonly { readonly lineIndex: number; readonly tokens: readonly LanguageToken[] }[];
	getLineTokens(lineIndex: number): readonly LanguageToken[];
}

export interface IResolvedSemanticTokensService extends IDisposable {
	createSource(source: SemanticTokenModelSource, styling?: SemanticTokenStylingResolver): SemanticTokenSource;
	createOverlay(base: SemanticTokenSource, overlay: SemanticTokenSource): SemanticTokenSource;
}
