import { DisposableOwner } from "../../../../../base/common/lifecycle.js";
import { type TextRange } from "../../../common/core/text.js";
import { createLanguageFeatureRequest, isLanguageFeatureRequestCurrent, type LanguageFeatureRequest } from "../../../common/languages/languageFeatureRequest.js";
import { LanguageFeatureProviderRegistry, type LanguageFeatureProviderMetadata } from "../../../common/languages/languageFeatureRegistry.js";
import { type TextModel } from "../../../common/model/textModel.js";

export type LanguageSymbolKind = string | number;

export interface LanguageDocumentSymbol {
  readonly name: string;
  readonly detail?: string;
  readonly kind: LanguageSymbolKind;
  readonly range: TextRange;
  readonly selectionRange: TextRange;
  readonly children?: readonly LanguageDocumentSymbol[];
}

export interface LanguageDocumentSymbolRequest extends LanguageFeatureRequest {}

export interface LanguageDocumentSymbolProvider extends LanguageFeatureProviderMetadata {
  provideDocumentSymbols(request: LanguageDocumentSymbolRequest, signal: AbortSignal): readonly LanguageDocumentSymbol[] | Promise<readonly LanguageDocumentSymbol[]>;
}

/** Provider-backed document symbol service used by outline and symbol navigation. */
export class DocumentSymbolService extends DisposableOwner {
  constructor(private readonly model: TextModel, private readonly providers: LanguageFeatureProviderRegistry<LanguageDocumentSymbolProvider>) {
    super();
  }

  get textModel(): TextModel {
    return this.model;
  }

  async provideDocumentSymbols(languageId: string, signal: AbortSignal = new AbortController().signal): Promise<readonly LanguageDocumentSymbol[]> {
    const request = createLanguageFeatureRequest(this.model, languageId, signal);
    for (const provider of this.providers.getProviders(languageId)) {
      if (!isLanguageFeatureRequestCurrent(request)) return Object.freeze([]);
      const symbols = await provider.provideDocumentSymbols(request, signal);
      if (!isLanguageFeatureRequestCurrent(request)) return Object.freeze([]);
      if (symbols.length > 0) return normalizeLanguageDocumentSymbols(symbols);
    }
    return Object.freeze([]);
  }
}

export function normalizeLanguageDocumentSymbols(symbols: readonly LanguageDocumentSymbol[]): readonly LanguageDocumentSymbol[] {
  if (!Array.isArray(symbols)) throw new TypeError("Document symbols must be an array");
  return Object.freeze(symbols.map(symbol => normalizeLanguageDocumentSymbol(symbol)));
}

function normalizeLanguageDocumentSymbol(symbol: LanguageDocumentSymbol): LanguageDocumentSymbol {
  if (!symbol || typeof symbol !== "object" || typeof symbol.name !== "string" || (typeof symbol.kind !== "string" && typeof symbol.kind !== "number")) throw new TypeError("Document symbol has invalid identity");
  return Object.freeze({
    name: symbol.name,
    ...(symbol.detail !== undefined ? { detail: symbol.detail } : {}),
    kind: symbol.kind,
    range: symbol.range,
    selectionRange: symbol.selectionRange,
    ...(symbol.children ? { children: normalizeLanguageDocumentSymbols(symbol.children) } : {}),
  });
}
