import { type TextPosition, type TextRange, type TextEdit } from "../../../common/core/text.js";
import { type LanguageFeatureRequest } from "../../../common/languages/languageFeatureRequest.js";
import { type LanguageFeatureProviderMetadata } from "../../../common/languageFeatureRegistry.js";

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
