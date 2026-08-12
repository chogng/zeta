import { DisposableOwner } from "../../../../base/common/lifecycle.js";
import { type TextRange } from "../../../common/core/text.js";
import { createLanguageFeatureRequest, isLanguageFeatureRequestCurrent, type LanguageFeatureRequest } from "../../../common/languages/languageFeatureRequest.js";
import { LanguageFeatureProviderRegistry, type LanguageFeatureProviderMetadata } from "../../../common/languages/languageFeatureRegistry.js";
import { type TextModel } from "../../../common/model/textModel.js";

export interface LanguageLinkedEditingRanges {
  readonly ranges: readonly TextRange[];
  readonly wordPattern?: RegExp;
}

export interface LanguageLinkedEditingRequest extends LanguageFeatureRequest {
  readonly range: TextRange;
}

export interface LanguageLinkedEditingProvider extends LanguageFeatureProviderMetadata {
  provideLinkedEditingRanges(request: LanguageLinkedEditingRequest, signal: AbortSignal): LanguageLinkedEditingRanges | undefined | Promise<LanguageLinkedEditingRanges | undefined>;
}

/** Calculates linked ranges; the browser controller later translates them into one model transaction. */
export class LinkedEditingService extends DisposableOwner {
  constructor(private readonly model: TextModel, private readonly providers: LanguageFeatureProviderRegistry<LanguageLinkedEditingProvider>) {
    super();
  }

  get textModel(): TextModel {
    return this.model;
  }

  async provideLinkedEditingRanges(languageId: string, range: TextRange, signal: AbortSignal = new AbortController().signal): Promise<LanguageLinkedEditingRanges | undefined> {
    const request = { ...createLanguageFeatureRequest(this.model, languageId, signal), range };
    for (const provider of this.providers.getProviders(languageId)) {
      const value = await provider.provideLinkedEditingRanges(request, signal);
      if (!isLanguageFeatureRequestCurrent(request)) return undefined;
      if (value) return Object.freeze({ ranges: Object.freeze([...value.ranges]), ...(value.wordPattern ? { wordPattern: value.wordPattern } : {}) });
    }
    return undefined;
  }
}
