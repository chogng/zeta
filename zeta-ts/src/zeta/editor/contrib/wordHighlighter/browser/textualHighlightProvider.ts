import { ResourceMap } from '../../../../base/common/map.js';
import { Disposable, DisposableStore, toDisposable, type IDisposable } from '../../../../base/common/lifecycle.js';
import { CancellationToken } from '../../../../base/common/cancellation.js';
import { Position } from '../../../common/core/position.js';
import { getTextWordSegments } from '../../../common/core/textSegmentation.js';
import { getWordSelectionRange } from '../../../common/cursor/wordSelection.js';
import { DocumentHighlightKind, type DocumentHighlight, type DocumentHighlightProvider, type MultiDocumentHighlightProvider } from '../../../common/languages.js';
import { findTextMatches } from '../../../common/model/textModelSearch.js';
import { type TextModel } from '../../../common/model/textModel.js';
import type { IEditorLanguageFeaturesService } from '../../../common/services/languageFeatures.js';

const MAX_TEXTUAL_HIGHLIGHTS = 10_000;
const registrations = new WeakMap<IEditorLanguageFeaturesService, TextualProviderRegistration>();

interface TextualProviderRegistration {
	count: number;
	readonly store: DisposableStore;
	readonly targets: WeakMap<TextModel, TextualHighlightTarget>;
}

class TextualDocumentHighlightProvider implements DocumentHighlightProvider, MultiDocumentHighlightProvider {
	readonly languageIds = Object.freeze(['*']);
	readonly providerId = 'builtin.textualDocumentHighlights';
	readonly selector = Object.freeze({ language: '*' });

	constructor(private readonly targets: WeakMap<TextModel, TextualHighlightTarget>) {}

	provideDocumentHighlights(model: TextModel, position: Position, token: CancellationToken): DocumentHighlight[] {
		return findHighlights(model, position, this.targets.get(model)?.wordPattern(), token);
	}

	provideMultiDocumentHighlights(primaryModel: TextModel, position: Position, otherModels: TextModel[], token: CancellationToken): Map<TextualHighlightTarget['resource'], DocumentHighlight[]> {
		const primaryTarget = this.targets.get(primaryModel);
		const wordPattern = primaryTarget?.wordPattern();
		const sourceRange = getWordSelectionRange(primaryModel, position, wordPattern);
		if (sourceRange.isEmpty() || !isWordRange(primaryModel, sourceRange.getStartPosition().column, sourceRange.getEndPosition().column, sourceRange.getStartPosition().lineNumber, wordPattern)) return new ResourceMap();
		const text = primaryModel.getTextInRange(sourceRange);
		const result = new ResourceMap<DocumentHighlight[]>();
		for (const model of [primaryModel, ...otherModels]) {
			if (token.isCancellationRequested) return new ResourceMap();
			const target = this.targets.get(model);
			if (!target) continue;
			result.set(target.resource, findText(model, text, target.wordPattern(), token));
		}
		return result;
	}
}

export class TextualHighlightTargetRegistration extends Disposable {
	constructor(languageFeaturesService: IEditorLanguageFeaturesService, target: TextualHighlightTarget) {
		super();
		this._register(acquireTextualHighlightProviders(languageFeaturesService, target));
	}
}

interface TextualHighlightTarget {
	readonly resource: import('../../../../base/common/uri.js').URI;
	readonly model: TextModel;
	readonly wordPattern: () => RegExp | undefined;
}

function acquireTextualHighlightProviders(service: IEditorLanguageFeaturesService, target: TextualHighlightTarget): IDisposable {
	let registration = registrations.get(service);
	if (!registration) {
		const targets = new WeakMap<TextModel, TextualHighlightTarget>();
		const provider = new TextualDocumentHighlightProvider(targets);
		const store = new DisposableStore();
		store.add(service.documentHighlightProvider.register(provider));
		store.add(service.multiDocumentHighlightProvider.register(provider));
		registration = { count: 0, store, targets };
		registrations.set(service, registration);
	}
	registration.targets.set(target.model, target);
	registration.count += 1;
	return toDisposable(() => {
		const current = registrations.get(service);
		if (!current) return;
		current.targets.delete(target.model);
		current.count -= 1;
		if (current.count > 0) return;
		registrations.delete(service);
		current.store.dispose();
	});
}

function findHighlights(model: TextModel, position: Position, wordPattern: RegExp | undefined, token: CancellationToken): DocumentHighlight[] {
	if (token.isCancellationRequested) return [];
	if (model.isDisposed()) return [];
	const range = getWordSelectionRange(model, position, wordPattern);
	if (range.isEmpty() || !isWordRange(model, range.getStartPosition().column, range.getEndPosition().column, range.getStartPosition().lineNumber, wordPattern)) return [];
	return findText(model, model.getTextInRange(range), wordPattern, token);
}

function findText(model: TextModel, text: string, wordPattern: RegExp | undefined, token: CancellationToken): DocumentHighlight[] {
	const matches = findTextMatches(model, {
		pattern: text,
		matchCase: true,
		wholeWord: wordPattern === undefined,
	}, { resultLimit: MAX_TEXTUAL_HIGHLIGHTS });
	const highlights: DocumentHighlight[] = [];
	for (const match of matches) {
		if (token.isCancellationRequested) return [];
		if (wordPattern && !isWordRange(model, match.range.getStartPosition().column, match.range.getEndPosition().column, match.range.getStartPosition().lineNumber, wordPattern)) continue;
		highlights.push(Object.freeze({ range: match.range, kind: DocumentHighlightKind.Text }));
	}
	return highlights;
}

function isWordRange(model: TextModel, startColumn: number, endColumn: number, lineNumber: number, wordPattern: RegExp | undefined): boolean {
	if (wordPattern) {
		const range = getWordSelectionRange(model, new Position(lineNumber, startColumn), wordPattern);
		return range.getStartPosition().column === startColumn && range.getEndPosition().column === endColumn;
	}
	return Boolean(getTextWordSegments(model.getLineContent(lineNumber)).find(segment => segment.wordLike && segment.start === startColumn - 1 && segment.end === endColumn - 1));
}
