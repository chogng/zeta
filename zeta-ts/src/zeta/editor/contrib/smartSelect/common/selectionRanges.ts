import { Disposable } from '../../../../base/common/lifecycle.js';
import { type URI } from '../../../../base/common/uri.js';
import { type TextRange } from '../../../common/core/text.js';
import { createLanguageFeatureRequest, isLanguageFeatureRequestCurrent, type LanguageFeatureRequest } from '../../../common/languages/languageFeatureRequest.js';
import { LanguageFeatureProviderRegistry, type LanguageFeatureProviderMetadata } from '../../../common/languageFeatureRegistry.js';
import { type TextModel } from '../../../common/model/textModel.js';

export interface LanguageSelectionRangeRequest extends LanguageFeatureRequest {
	readonly resource?: URI;
	readonly ranges: readonly TextRange[];
}

export interface LanguageSelectionRangeProvider extends LanguageFeatureProviderMetadata {
	provideSelectionRanges(request: LanguageSelectionRangeRequest, signal: AbortSignal): readonly TextRange[] | Promise<readonly TextRange[]>;
}

/** Collects versioned structural selection candidates from registered language providers. */
export class SelectionRangeService extends Disposable {
	constructor(private readonly model: TextModel, private readonly providers: LanguageFeatureProviderRegistry<LanguageSelectionRangeProvider>, private readonly resource?: URI) {
		super();
	}

	async provideSelectionRanges(languageId: string, ranges: readonly TextRange[], signal: AbortSignal = new AbortController().signal): Promise<readonly TextRange[]> {
		const request: LanguageSelectionRangeRequest = Object.freeze({ ...createLanguageFeatureRequest(this.model, languageId, signal), ranges: Object.freeze([...ranges]), ...(this.resource ? { resource: this.resource } : {}) });
		const result: TextRange[] = [];
		for (const provider of this.providers.getProviders(languageId)) {
			if (!isLanguageFeatureRequestCurrent(request)) return Object.freeze([]);
			result.push(...await provider.provideSelectionRanges(request, signal));
			if (!isLanguageFeatureRequestCurrent(request)) return Object.freeze([]);
		}
		return Object.freeze(result);
	}
}
