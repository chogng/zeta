import { ResourceMap } from '../../../../base/common/map.js';
import { Disposable, DisposableStore, toDisposable, type IDisposable } from '../../../../base/common/lifecycle.js';
import { CancellationToken } from '../../../../base/common/cancellation.js';
import { TextPosition } from '../../../common/core/text.js';
import { getTextWordSegments } from '../../../common/core/textSegmentation.js';
import { WordOperations } from '../../../common/cursor/cursorWordOperations.js';
import { DocumentHighlightKind, type DocumentHighlight, type DocumentHighlightProvider, type DocumentHighlightRequest, type DocumentHighlightTarget, type MultiDocumentHighlightProvider } from '../../../common/languages/documentHighlights.js';
import { findTextMatches } from '../../../common/model/textModelSearch.js';
import { type TextModel } from '../../../common/model/textModel.js';
import type { ILanguageFeaturesService } from '../../../common/services/languageFeatures.js';

const MAX_TEXTUAL_HIGHLIGHTS = 10_000;
const registrations = new WeakMap<ILanguageFeaturesService, TextualProviderRegistration>();

interface TextualProviderRegistration {
	count: number;
	readonly store: DisposableStore;
}

class TextualDocumentHighlightProvider implements DocumentHighlightProvider, MultiDocumentHighlightProvider {
	readonly languageIds = Object.freeze(['*']);
	readonly providerId = 'builtin.textualDocumentHighlights';

	provideDocumentHighlights(request: DocumentHighlightRequest, token: CancellationToken): readonly DocumentHighlight[] {
		return findHighlights(request.model, request.position, request.wordPattern, token);
	}

	provideMultiDocumentHighlights(request: DocumentHighlightRequest, targets: readonly DocumentHighlightTarget[], token: CancellationToken): ReadonlyMap<DocumentHighlightTarget['resource'], readonly DocumentHighlight[]> {
		const sourceRange = WordOperations.getWordSelectionRange(request.model, request.position, request.wordPattern);
		if (sourceRange.empty || !isWordRange(request.model, sourceRange.start.columnIndex, sourceRange.end.columnIndex, sourceRange.start.lineIndex, request.wordPattern)) return new ResourceMap();
		const text = request.model.getTextInRange(sourceRange);
		const result = new ResourceMap<readonly DocumentHighlight[]>();
		for (const target of targets) {
			if (token.isCancellationRequested) return new ResourceMap();
			result.set(target.resource, findText(target.model, text, target.wordPattern, token));
		}
		return result;
	}
}

export class TextualMultiDocumentHighlightFeature extends Disposable {
	constructor(languageFeaturesService: ILanguageFeaturesService) {
		super();
		this._register(acquireTextualHighlightProviders(languageFeaturesService));
	}
}

function acquireTextualHighlightProviders(service: ILanguageFeaturesService): IDisposable {
	let registration = registrations.get(service);
	if (!registration) {
		const provider = new TextualDocumentHighlightProvider();
		const store = new DisposableStore();
		store.add(service.documentHighlightProvider.register(provider));
		store.add(service.multiDocumentHighlightProvider.register(provider));
		registration = { count: 0, store };
		registrations.set(service, registration);
	}
	registration.count += 1;
	return toDisposable(() => {
		const current = registrations.get(service);
		if (!current) return;
		current.count -= 1;
		if (current.count > 0) return;
		registrations.delete(service);
		current.store.dispose();
	});
}

function findHighlights(model: TextModel, position: DocumentHighlightRequest['position'], wordPattern: RegExp | undefined, token: CancellationToken): readonly DocumentHighlight[] {
	if (token.isCancellationRequested) return Object.freeze([]);
	if (model.isDisposed) return Object.freeze([]);
	const range = WordOperations.getWordSelectionRange(model, position, wordPattern);
	if (range.empty || !isWordRange(model, range.start.columnIndex, range.end.columnIndex, range.start.lineIndex, wordPattern)) return Object.freeze([]);
	return findText(model, model.getTextInRange(range), wordPattern, token);
}

function findText(model: TextModel, text: string, wordPattern: RegExp | undefined, token: CancellationToken): readonly DocumentHighlight[] {
	const matches = findTextMatches(model, {
		pattern: text,
		matchCase: true,
		wholeWord: wordPattern === undefined,
	}, { resultLimit: MAX_TEXTUAL_HIGHLIGHTS });
	const highlights: DocumentHighlight[] = [];
	for (const match of matches) {
		if (token.isCancellationRequested) return Object.freeze([]);
		if (wordPattern && !isWordRange(model, match.range.start.columnIndex, match.range.end.columnIndex, match.range.start.lineIndex, wordPattern)) continue;
		highlights.push(Object.freeze({ range: match.range, kind: DocumentHighlightKind.Text }));
	}
	return Object.freeze(highlights);
}

function isWordRange(model: TextModel, startColumn: number, endColumn: number, lineIndex: number, wordPattern: RegExp | undefined): boolean {
	if (wordPattern) {
		const range = WordOperations.getWordSelectionRange(model, TextPosition.at(lineIndex, startColumn), wordPattern);
		return range.start.columnIndex === startColumn && range.end.columnIndex === endColumn;
	}
	return Boolean(getTextWordSegments(model.getLineContent(lineIndex)).find(segment => segment.wordLike && segment.start === startColumn && segment.end === endColumn));
}
