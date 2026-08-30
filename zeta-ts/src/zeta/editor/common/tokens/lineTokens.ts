import type { ILanguageIdCodec } from '../languages.js';
import type { ColorId, ITokenPresentation, StandardTokenType } from '../encodedTokenAttributes.js';

/** Token access contract consumed by the view-line renderer. */
export interface IViewLineTokens {
	languageIdCodec: ILanguageIdCodec;
	equals(other: IViewLineTokens): boolean;
	getCount(): number;
	getStandardTokenType(tokenIndex: number): StandardTokenType;
	getForeground(tokenIndex: number): ColorId;
	getEndOffset(tokenIndex: number): number;
	getClassName(tokenIndex: number): string;
	getInlineStyle(tokenIndex: number, colorMap: string[]): string;
	getPresentation(tokenIndex: number): ITokenPresentation;
	findTokenIndexAtOffset(offset: number): number;
	getLineContent(): string;
	getMetadata(tokenIndex: number): number;
	getLanguageId(tokenIndex: number): string;
	getTokenText(tokenIndex: number): string;
	forEach(callback: (tokenIndex: number) => void): void;
}
