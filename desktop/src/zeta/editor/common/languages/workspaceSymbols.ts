import { DisposableOwner } from "../../../base/common/lifecycle.js";
import { type URI } from "../../../base/common/uri.js";
import { type TextRange } from "../core/text.js";
import { LanguageFeatureProviderRegistry, type LanguageFeatureProviderMetadata } from "./languageFeatureRegistry.js";

export type LanguageWorkspaceSymbolKind = string | number;

export interface LanguageWorkspaceSymbol {
  readonly name: string;
  readonly kind: LanguageWorkspaceSymbolKind;
  readonly resource: URI;
  readonly range: TextRange;
  readonly containerName?: string;
  readonly data?: unknown;
}

export interface LanguageWorkspaceSymbolProvider extends LanguageFeatureProviderMetadata {
  provideWorkspaceSymbols(query: string, signal: AbortSignal): readonly LanguageWorkspaceSymbol[] | Promise<readonly LanguageWorkspaceSymbol[]>;
  resolveWorkspaceSymbol?(symbol: LanguageWorkspaceSymbol, signal: AbortSignal): LanguageWorkspaceSymbol | Promise<LanguageWorkspaceSymbol>;
}

/** Aggregates workspace symbol providers independently from one editor model. */
export class WorkspaceSymbolService extends DisposableOwner {
  constructor(private readonly providers: LanguageFeatureProviderRegistry<LanguageWorkspaceSymbolProvider>) {
    super();
  }

  async provideWorkspaceSymbols(query: string, signal: AbortSignal = new AbortController().signal): Promise<readonly LanguageWorkspaceSymbol[]> {
    const symbols: LanguageWorkspaceSymbol[] = [];
    for (const provider of this.providers.getProviders("*")) {
      if (signal.aborted) return Object.freeze([]);
      symbols.push(...(await provider.provideWorkspaceSymbols(query, signal)).map(normalizeWorkspaceSymbol));
      if (signal.aborted) return Object.freeze([]);
    }
    return Object.freeze(symbols);
  }

  async resolveWorkspaceSymbol(symbol: LanguageWorkspaceSymbol, signal: AbortSignal = new AbortController().signal): Promise<LanguageWorkspaceSymbol> {
    for (const provider of this.providers.getProviders("*")) {
      if (!provider.resolveWorkspaceSymbol) continue;
      return normalizeWorkspaceSymbol(await provider.resolveWorkspaceSymbol(symbol, signal));
    }
    return symbol;
  }
}

function normalizeWorkspaceSymbol(symbol: LanguageWorkspaceSymbol): LanguageWorkspaceSymbol {
  if (!symbol || typeof symbol !== "object" || typeof symbol.name !== "string" || symbol.name.trim().length === 0 || !symbol.resource) throw new TypeError("Workspace symbol requires a name and resource");
  return Object.freeze({
    name: symbol.name,
    kind: symbol.kind,
    resource: symbol.resource,
    range: symbol.range,
    ...(symbol.containerName !== undefined ? { containerName: symbol.containerName } : {}),
    ...(symbol.data !== undefined ? { data: symbol.data } : {}),
  });
}
