import { type CancellationToken } from '../../base/common/cancellation.js';
import { type URI } from '../../base/common/uri.js';
import { EditOperation, type ISingleEditOperation } from './core/editOperation.js';
import { type Position } from './core/position.js';
import { type IRange, Range } from './core/range.js';
import { type TextSnapshot } from './core/textChange.js';
import { type LanguageId } from './encodedTokenAttributes.js';
import { type LanguageSelector } from './languageSelector.js';
import * as model from './model.js';
import { type TextModel } from './model/textModel.js';
import { type LanguageTokenResult } from './tokens/languageTokens.js';

type Thenable<T> = PromiseLike<T>;

export type ProviderResult<T> = T | undefined | null | Thenable<T | undefined | null>;

export interface LanguageSemanticTokensRequest {
	readonly requestId: number;
	readonly model: TextModel;
	readonly snapshot: TextSnapshot;
	readonly languageId: string;
	readonly resource?: URI;
}

export interface LanguageSemanticTokensProvider {
	provideSemanticTokens(request: LanguageSemanticTokensRequest, signal: AbortSignal): LanguageTokenResult | undefined | PromiseLike<LanguageTokenResult | undefined>;
}

/** @internal */
export interface ILanguageIdCodec {
	encodeLanguageId(languageId: string): LanguageId;
	decodeLanguageId(languageId: LanguageId): string;
}

export interface TextEdit {
	range: IRange;
	text: string;
	eol?: model.EndOfLineSequence;
}

/** Options supplied to document and range formatting providers. */
export interface FormattingOptions {
	tabSize: number;
	insertSpaces: boolean;
}

/** @internal */
export interface IInplaceReplaceSupportResult {
	value: string;
	range: IRange;
}

/** @internal */
export abstract class TextEdit {
	static asEditOperation(edit: TextEdit): ISingleEditOperation {
		const range = Range.lift(edit.range);
		return range.isEmpty()
			? EditOperation.insert(range.getStartPosition(), edit.text)
			: EditOperation.replace(range, edit.text);
	}

	static isTextEdit(thing: unknown): thing is TextEdit {
		const possibleTextEdit = thing as TextEdit;
		return typeof possibleTextEdit?.text === 'string' && Range.isIRange(possibleTextEdit.range);
	}
}

/** A color in RGBA format. */
export interface IColor {
	readonly red: number;
	readonly green: number;
	readonly blue: number;
	readonly alpha: number;
}

/** String representations for a color. */
export interface IColorPresentation {
	label: string;
	textEdit?: TextEdit;
	additionalTextEdits?: TextEdit[];
}

/** A color range in a text model. */
export interface IColorInformation {
	range: IRange;
	color: IColor;
}

/** A provider of colors for editor models. */
export interface DocumentColorProvider {
	provideDocumentColors(model: model.ITextModel, token: CancellationToken): ProviderResult<IColorInformation[]>;
	provideColorPresentations(model: model.ITextModel, colorInfo: IColorInformation, token: CancellationToken): ProviderResult<IColorPresentation[]>;
}

export enum DocumentHighlightKind {
	Text,
	Read,
	Write,
}

export interface DocumentHighlight {
	range: IRange;
	kind?: DocumentHighlightKind;
}

export interface MultiDocumentHighlight {
	uri: URI;
	highlights: DocumentHighlight[];
}

export interface DocumentHighlightProvider {
	provideDocumentHighlights(model: model.ITextModel, position: Position, token: CancellationToken): ProviderResult<DocumentHighlight[]>;
}

export interface MultiDocumentHighlightProvider {
	readonly selector: LanguageSelector;
	provideMultiDocumentHighlights(primaryModel: model.ITextModel, position: Position, otherModels: model.ITextModel[], token: CancellationToken): ProviderResult<Map<URI, DocumentHighlight[]>>;
}

/** @internal */
export class ProviderId {
	public static fromExtensionId(extensionId: string | undefined): ProviderId {
		return new ProviderId(extensionId, undefined, undefined);
	}

	constructor(
		public readonly extensionId: string | undefined,
		public readonly extensionVersion: string | undefined,
		public readonly providerId: string | undefined
	) {
	}

	toString(): string {
		let result = '';
		if (this.extensionId) {
			result += this.extensionId;
		}
		if (this.extensionVersion) {
			result += `@${this.extensionVersion}`;
		}
		if (this.providerId) {
			result += `:${this.providerId}`;
		}
		if (result.length === 0) {
			result = 'unknown';
		}
		return result;
	}

	toStringWithoutVersion(): string {
		let result = '';
		if (this.extensionId) {
			result += this.extensionId;
		}
		if (this.providerId) {
			result += `:${this.providerId}`;
		}
		return result;
	}
}

/** @internal */
export class VersionedExtensionId {
	public static tryCreate(extensionId: string | undefined, version: string | undefined): VersionedExtensionId | undefined {
		if (!extensionId || !version) {
			return undefined;
		}
		return new VersionedExtensionId(extensionId, version);
	}

	constructor(
		public readonly extensionId: string,
		public readonly version: string,
	) { }

	toString(): string {
		return `${this.extensionId}@${this.version}`;
	}
}
