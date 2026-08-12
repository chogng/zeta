import { DisposableOwner } from "../../../../base/common/lifecycle.js";
import { type TextPosition, type TextRange } from "../../../common/core/text.js";
import { createLanguageFeatureRequest, isLanguageFeatureRequestCurrent, type LanguageFeatureRequest } from "../../../common/languages/languageFeatureRequest.js";
import { LanguageFeatureProviderRegistry, type LanguageFeatureProviderMetadata } from "../../../common/languages/languageFeatureRegistry.js";
import { type TextModel } from "../../../common/model/textModel.js";

export type LanguageHoverContent = string | { readonly value: string; readonly language?: string };

export interface LanguageHover {
  readonly range?: TextRange;
  readonly contents: readonly LanguageHoverContent[];
}

export interface LanguageHoverRequest extends LanguageFeatureRequest {
  readonly position: TextPosition;
}

export interface LanguageHoverProvider extends LanguageFeatureProviderMetadata {
  provideHover(request: LanguageHoverRequest, signal: AbortSignal): LanguageHover | undefined | Promise<LanguageHover | undefined>;
}

/** Stores hover providers and exposes deterministic first-provider semantics. */
export class HoverService extends DisposableOwner {
  constructor(private readonly model: TextModel, private readonly providers: LanguageFeatureProviderRegistry<LanguageHoverProvider>) {
    super();
  }

  async provideHover(languageId: string, position: TextPosition, signal: AbortSignal = new AbortController().signal): Promise<LanguageHover | undefined> {
    const request = { ...createLanguageFeatureRequest(this.model, languageId, signal), position };
    for (const provider of this.providers.getProviders(languageId)) {
      if (!isLanguageFeatureRequestCurrent(request)) return undefined;
      const value = await provider.provideHover(request, signal);
      if (!isLanguageFeatureRequestCurrent(request)) return undefined;
      if (value) return normalizeLanguageHover(value);
    }
    return undefined;
  }
}

export function normalizeLanguageHover(value: LanguageHover): LanguageHover {
  if (!value || typeof value !== "object" || !Array.isArray(value.contents) || value.contents.length === 0) throw new TypeError("Language hover must contain content");
  const contents = value.contents.map(content => {
    if (typeof content === "string") return content;
    if (!content || typeof content !== "object" || typeof content.value !== "string") throw new TypeError("Language hover content must contain a string value");
    return Object.freeze({ value: content.value, ...(content.language !== undefined ? { language: content.language } : {}) });
  });
  return Object.freeze({ ...(value.range ? { range: value.range } : {}), contents: Object.freeze(contents) });
}
