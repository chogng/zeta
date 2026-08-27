import { Disposable } from "../../../../base/common/lifecycle.js";
import { type TextPosition, type TextRange, type TextEdit } from "../../../common/core/text.js";
import { createLanguageFeatureRequest, isLanguageFeatureRequestCurrent, type LanguageFeatureRequest } from "../../../common/languages/languageFeatureRequest.js";
import { LanguageFeatureProviderRegistry, type LanguageFeatureProviderMetadata } from "../../../common/languages/languageFeatureRegistry.js";
import { type TextModel } from "../../../common/model/textModel.js";

export interface LanguageInlineCompletionItem {
	readonly insertText: string;
	readonly range?: TextRange;
	readonly filterText?: string;
	readonly commandId?: string;
	readonly additionalTextEdits?: readonly TextEdit[];
}

export interface LanguageInlineCompletionsRequest extends LanguageFeatureRequest {
	readonly position: TextPosition;
	readonly triggerKind: "automatic" | "explicit";
}

export interface LanguageInlineCompletionsProvider extends LanguageFeatureProviderMetadata {
	provideInlineCompletions(request: LanguageInlineCompletionsRequest, signal: AbortSignal): readonly LanguageInlineCompletionItem[] | Promise<readonly LanguageInlineCompletionItem[]>;
}

/** Owns ghost-text candidates and freshness; accepting a candidate is a cursor command. */
export class InlineCompletionsService extends Disposable {
	constructor(private readonly model: TextModel, private readonly providers: LanguageFeatureProviderRegistry<LanguageInlineCompletionsProvider>) {
		super();
	}

	async provideInlineCompletions(languageId: string, position: TextPosition, triggerKind: LanguageInlineCompletionsRequest["triggerKind"], signal: AbortSignal = new AbortController().signal): Promise<readonly LanguageInlineCompletionItem[]> {
		const request = { ...createLanguageFeatureRequest(this.model, languageId, signal), position, triggerKind };
		const result: LanguageInlineCompletionItem[] = [];
		for (const provider of this.providers.getProviders(languageId)) {
			if (!isLanguageFeatureRequestCurrent(request)) return Object.freeze([]);
			const items = await provider.provideInlineCompletions(request, signal);
			if (!isLanguageFeatureRequestCurrent(request)) return Object.freeze([]);
			result.push(...items.map(normalizeLanguageInlineCompletionItem));
		}
		return Object.freeze(result);
	}
}

function normalizeLanguageInlineCompletionItem(item: LanguageInlineCompletionItem): LanguageInlineCompletionItem {
	if (!item || typeof item !== "object" || typeof item.insertText !== "string") throw new TypeError("Inline completion must contain insert text");
	return Object.freeze({ insertText: item.insertText, ...(item.range ? { range: item.range } : {}), ...(item.filterText !== undefined ? { filterText: item.filterText } : {}), ...(item.commandId !== undefined ? { commandId: item.commandId } : {}), ...(item.additionalTextEdits ? { additionalTextEdits: Object.freeze([...item.additionalTextEdits]) } : {}) });
}
