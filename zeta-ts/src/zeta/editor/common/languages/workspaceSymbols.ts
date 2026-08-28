import { Disposable } from "../../../base/common/lifecycle.js";
import { type URI } from "../../../base/common/uri.js";
import { type TextRange } from "../core/text.js";
import { LanguageFeatureProviderRegistry, type LanguageFeatureProviderMetadata } from "../languageFeatureRegistry.js";

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
export class WorkspaceSymbolService extends Disposable {
	constructor(private readonly providers: LanguageFeatureProviderRegistry<LanguageWorkspaceSymbolProvider>) {
		super();
	}

	async provideWorkspaceSymbols(query: string, signal: AbortSignal = new AbortController().signal, onDidUpdate?: (symbols: readonly LanguageWorkspaceSymbol[]) => void): Promise<readonly LanguageWorkspaceSymbol[]> {
		const providers = this.providers.getProviders("*");
		const completed = new Array<readonly LanguageWorkspaceSymbol[] | undefined>(providers.length);
		await Promise.all(providers.map(async (provider, index) => {
			try {
				const symbols = await provider.provideWorkspaceSymbols(query, signal);
				completed[index] = Object.freeze(symbols.map(normalizeWorkspaceSymbol));
			} catch {
				completed[index] = Object.freeze([]);
			}
			if (!signal.aborted) onDidUpdate?.(mergeWorkspaceSymbols(completed));
		}));
		return signal.aborted ? Object.freeze([]) : mergeWorkspaceSymbols(completed);
	}

	async resolveWorkspaceSymbol(symbol: LanguageWorkspaceSymbol, signal: AbortSignal = new AbortController().signal): Promise<LanguageWorkspaceSymbol> {
		for (const provider of this.providers.getProviders("*")) {
			if (!provider.resolveWorkspaceSymbol) continue;
			return normalizeWorkspaceSymbol(await provider.resolveWorkspaceSymbol(symbol, signal));
		}
		return symbol;
	}
}

function mergeWorkspaceSymbols(providerResults: readonly (readonly LanguageWorkspaceSymbol[] | undefined)[]): readonly LanguageWorkspaceSymbol[] {
	const seen = new Set<string>();
	const merged: LanguageWorkspaceSymbol[] = [];
	for (const symbols of providerResults) {
		for (const symbol of symbols ?? []) {
			const key = `${symbol.resource.toString()}\0${symbol.name}\0${symbol.range.start.lineIndex}:${symbol.range.start.columnIndex}`;
			if (seen.has(key)) continue;
			seen.add(key);
			merged.push(symbol);
		}
	}
	return Object.freeze(merged);
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
