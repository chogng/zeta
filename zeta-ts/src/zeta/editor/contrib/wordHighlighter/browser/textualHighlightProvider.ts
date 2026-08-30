import { ResourceMap } from '../../../../base/common/map.js';
import { Disposable, DisposableStore, toDisposable, type IDisposable } from '../../../../base/common/lifecycle.js';
import { CancellationToken } from '../../../../base/common/cancellation.js';
import { Position } from '../../../common/core/position.js';
import { USUAL_WORD_SEPARATORS } from '../../../common/core/wordHelper.js';
import { DocumentHighlightKind, type DocumentHighlight, type DocumentHighlightProvider, type MultiDocumentHighlightProvider } from '../../../common/languages.js';
import { type TextModel } from '../../../common/model/textModel.js';
import type { ILanguageFeaturesService } from '../../../common/services/languageFeatures.js';

const MAX_TEXTUAL_HIGHLIGHTS = 10_000;
const registrations = new WeakMap<ILanguageFeaturesService, TextualProviderRegistration>();

interface TextualProviderRegistration {
	count: number;
	readonly store: DisposableStore;
	readonly targets: WeakMap<TextModel, TextualHighlightTarget>;
}

class TextualDocumentHighlightProvider implements DocumentHighlightProvider, MultiDocumentHighlightProvider {
	readonly selector = Object.freeze({ language: '*' });

	constructor(private readonly targets: WeakMap<TextModel, TextualHighlightTarget>) {}

	provideDocumentHighlights(model: TextModel, position: Position, token: CancellationToken): DocumentHighlight[] {
		return findHighlights(model, position, token);
	}

	provideMultiDocumentHighlights(primaryModel: TextModel, position: Position, otherModels: TextModel[], token: CancellationToken): Map<TextualHighlightTarget['resource'], DocumentHighlight[]> {
		const word = primaryModel.getWordAtPosition(position);
		if (!word) return new ResourceMap();
		const result = new ResourceMap<DocumentHighlight[]>();
		for (const model of [primaryModel, ...otherModels]) {
			if (token.isCancellationRequested) return new ResourceMap();
			const target = this.targets.get(model);
			if (!target) continue;
			result.set(target.resource, findText(model, word.word, token));
		}
		return result;
	}
}

export class TextualHighlightTargetRegistration extends Disposable {
	constructor(languageFeaturesService: ILanguageFeaturesService, target: TextualHighlightTarget) {
		super();
		this._register(acquireTextualHighlightProviders(languageFeaturesService, target));
	}
}

interface TextualHighlightTarget {
	readonly resource: import('../../../../base/common/uri.js').URI;
	readonly model: TextModel;
}

function acquireTextualHighlightProviders(service: ILanguageFeaturesService, target: TextualHighlightTarget): IDisposable {
	let registration = registrations.get(service);
	if (!registration) {
		const targets = new WeakMap<TextModel, TextualHighlightTarget>();
		const provider = new TextualDocumentHighlightProvider(targets);
		const store = new DisposableStore();
		store.add(service.documentHighlightProvider.register(provider.selector, provider));
		store.add(service.multiDocumentHighlightProvider.register(provider.selector, provider));
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

function findHighlights(model: TextModel, position: Position, token: CancellationToken): DocumentHighlight[] {
	if (token.isCancellationRequested) return [];
	if (model.isDisposed()) return [];
	const word = model.getWordAtPosition(position);
	return word ? findText(model, word.word, token) : [];
}

function findText(model: TextModel, text: string, token: CancellationToken): DocumentHighlight[] {
	const matches = model.findMatches(text, true, false, true, USUAL_WORD_SEPARATORS, false, MAX_TEXTUAL_HIGHLIGHTS);
	const highlights: DocumentHighlight[] = [];
	for (const match of matches) {
		if (token.isCancellationRequested) return [];
		highlights.push(Object.freeze({ range: match.range, kind: DocumentHighlightKind.Text }));
	}
	return highlights;
}
