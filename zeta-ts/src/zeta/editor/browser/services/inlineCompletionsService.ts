import { Disposable } from '../../../base/common/lifecycle.js';
import { type Position } from '../../common/core/position.js';
import { LanguageFeatureProviderRegistry } from '../../common/languageFeatureRegistry.js';
import { createLanguageFeatureRequest, isLanguageFeatureRequestCurrent } from '../../common/languages/languageFeatureRequest.js';
import { type TextModel } from '../../common/model/textModel.js';
import { type LanguageInlineCompletionItem, type LanguageInlineCompletionsProvider, type LanguageInlineCompletionsRequest } from '../../contrib/inlineCompletions/common/inlineCompletions.js';

/** Owns ghost-text candidates and freshness; accepting a candidate remains a cursor command. */
export class InlineCompletionsService extends Disposable {
	constructor(private readonly model: TextModel, private readonly providers: LanguageFeatureProviderRegistry<LanguageInlineCompletionsProvider>) {
		super();
	}

	public async provideInlineCompletions(languageId: string, position: Position, triggerKind: LanguageInlineCompletionsRequest['triggerKind'], signal: AbortSignal = new AbortController().signal): Promise<readonly LanguageInlineCompletionItem[]> {
		const request = { ...createLanguageFeatureRequest(this.model, languageId, signal), position, triggerKind };
		const result: LanguageInlineCompletionItem[] = [];
		for (const provider of this.providers.getProviders(languageId)) {
			if (!isLanguageFeatureRequestCurrent(request)) {
				return Object.freeze([]);
			}
			const items = await provider.provideInlineCompletions(request, signal);
			if (!isLanguageFeatureRequestCurrent(request)) {
				return Object.freeze([]);
			}
			result.push(...items.map(normalizeLanguageInlineCompletionItem));
		}
		return Object.freeze(result);
	}
}

function normalizeLanguageInlineCompletionItem(item: LanguageInlineCompletionItem): LanguageInlineCompletionItem {
	if (!item || typeof item !== 'object' || typeof item.insertText !== 'string') {
		throw new TypeError('Inline completion must contain insert text');
	}
	return Object.freeze({
		insertText: item.insertText,
		...(item.range ? { range: item.range } : {}),
		...(item.filterText !== undefined ? { filterText: item.filterText } : {}),
		...(item.commandId !== undefined ? { commandId: item.commandId } : {}),
		...(item.additionalTextEdits ? { additionalTextEdits: Object.freeze([...item.additionalTextEdits]) } : {}),
	});
}
