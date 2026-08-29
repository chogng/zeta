import { type URI } from '../../../base/common/uri.js';
import { type CancellationToken } from '../../../base/common/cancellation.js';
import { type Position } from '../core/position.js';
import { type Range } from '../core/range.js';
import { type TextSnapshot } from '../core/textChange.js';
import { type TextModel } from '../model/textModel.js';
import { type LanguageFeatureProviderMetadata } from '../languageFeatureRegistry.js';

export enum DocumentHighlightKind {
	Text = 'text',
	Read = 'read',
	Write = 'write',
}

export interface DocumentHighlight {
	readonly range: Range;
	readonly kind?: DocumentHighlightKind;
}

export interface DocumentHighlightRequest {
	readonly resource: URI;
	readonly model: TextModel;
	readonly snapshot: TextSnapshot;
	readonly languageId: string;
	readonly position: Position;
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
