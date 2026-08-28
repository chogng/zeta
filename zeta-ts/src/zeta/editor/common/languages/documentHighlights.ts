import { type URI } from '../../../base/common/uri.js';
import { type CancellationToken } from '../../../base/common/cancellation.js';
import { type TextPosition, type TextRange, type TextSnapshot } from '../core/text.js';
import { type TextModel } from '../model/textModel.js';
import { type LanguageFeatureProviderMetadata } from './languageFeatureRegistry.js';

export enum DocumentHighlightKind {
	Text = 'text',
	Read = 'read',
	Write = 'write',
}

export interface DocumentHighlight {
	readonly range: TextRange;
	readonly kind?: DocumentHighlightKind;
}

export interface DocumentHighlightRequest {
	readonly resource: URI;
	readonly model: TextModel;
	readonly snapshot: TextSnapshot;
	readonly languageId: string;
	readonly position: TextPosition;
	readonly wordPattern?: RegExp;
}

export interface DocumentHighlightTarget {
	readonly resource: URI;
	readonly model: TextModel;
	readonly snapshot: TextSnapshot;
	readonly languageId: string;
	readonly wordPattern?: RegExp;
}

export interface DocumentHighlightProvider extends LanguageFeatureProviderMetadata {
	provideDocumentHighlights(request: DocumentHighlightRequest, token: CancellationToken): readonly DocumentHighlight[] | null | undefined | PromiseLike<readonly DocumentHighlight[] | null | undefined>;
}

export interface MultiDocumentHighlightProvider extends LanguageFeatureProviderMetadata {
	provideMultiDocumentHighlights(request: DocumentHighlightRequest, targets: readonly DocumentHighlightTarget[], token: CancellationToken): ReadonlyMap<URI, readonly DocumentHighlight[]> | null | undefined | PromiseLike<ReadonlyMap<URI, readonly DocumentHighlight[]> | null | undefined>;
}
