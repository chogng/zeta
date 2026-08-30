import assert from "node:assert/strict";
import test from "node:test";

import { URI } from "../../../base/common/uri.js";
import { Position } from "../../common/core/position.js";
import { Range } from "../../common/core/range.js";
import { OwnedLanguageFeatureProviderRegistry } from "../../common/ownedLanguageFeatureProviderRegistry.js";
import { type LanguageWorkspaceSymbol, type LanguageWorkspaceSymbolProvider, WorkspaceSymbolService } from "../../common/languages/workspaceSymbols.js";

const RANGE = Range.fromPositions(new Position((0) + 1, (0) + 1), new Position((0) + 1, (4) + 1));

test("workspace symbols publish fast providers before deterministic final fusion", async () => {
	using providers = new OwnedLanguageFeatureProviderRegistry<LanguageWorkspaceSymbolProvider>();
	let releaseSlow!: (symbols: readonly LanguageWorkspaceSymbol[]) => void;
	providers.register(provider("slow", () => new Promise(resolve => { releaseSlow = resolve; })));
	providers.register(provider("fast", async () => [symbol("fast", "fast.ts")]));
	using service = new WorkspaceSymbolService(providers);
	const updates: (readonly LanguageWorkspaceSymbol[])[] = [];

	const final = service.provideWorkspaceSymbols("f", new AbortController().signal, symbols => updates.push(symbols));
	await new Promise(resolve => setTimeout(resolve, 0));
	assert.deepEqual(updates.at(-1)?.map(symbol => symbol.name), ["fast"]);

	releaseSlow([symbol("slow", "slow.ts")]);
	assert.deepEqual((await final).map(symbol => symbol.name), ["slow", "fast"]);
});

test("workspace symbol fusion deduplicates locations and survives provider failure", async () => {
	using providers = new OwnedLanguageFeatureProviderRegistry<LanguageWorkspaceSymbolProvider>();
	providers.register(provider("preferred", async () => [symbol("same", "same.ts", "preferred")]));
	providers.register(provider("failed", async () => { throw new Error("unavailable"); }));
	providers.register(provider("duplicate", async () => [symbol("same", "same.ts", "duplicate"), symbol("other", "other.ts")]));
	using service = new WorkspaceSymbolService(providers);

	const result = await service.provideWorkspaceSymbols("same");

	assert.deepEqual(result.map(symbol => symbol.name), ["same", "other"]);
	assert.equal(result[0]?.containerName, "preferred");
});

function provider(providerId: string, provide: LanguageWorkspaceSymbolProvider["provideWorkspaceSymbols"]): LanguageWorkspaceSymbolProvider {
	return { languageIds: ["*"], providerId, provideWorkspaceSymbols: provide };
}

function symbol(name: string, path: string, containerName?: string): LanguageWorkspaceSymbol {
	return { name, kind: "function", resource: URI.file(`/workspace/${path}`), range: RANGE, ...(containerName ? { containerName } : {}) };
}
