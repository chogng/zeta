import { Disposable } from "../../../../base/common/lifecycle.js";
import { type URI } from "../../../../base/common/uri.js";
import { type Position } from "../../../common/core/position.js";
import { Range } from "../../../common/core/range.js";
import { createLanguageFeatureRequest, isLanguageFeatureRequestCurrent, type LanguageFeatureRequest } from "../../../common/languages/languageFeatureRequest.js";
import { OwnedLanguageFeatureProviderRegistry, type LanguageFeatureProviderMetadata } from "../../../common/ownedLanguageFeatureProviderRegistry.js";
import { type TextModel } from "../../../common/model/textModel.js";

/** One source or target position returned by a cross-resource language feature. */
export interface LanguageLocation {
	readonly resource: URI;
	readonly range: Range;
	/** The narrower symbol-name range to select after opening the target. */
	readonly selectionRange?: Range;
}

export interface LanguageLocationRequest extends LanguageFeatureRequest {
	readonly resource: URI;
	readonly position: Position;
}

export interface LanguageReferenceRequest extends LanguageLocationRequest {
	readonly includeDeclaration: boolean;
}

export interface LanguageDefinitionProvider extends LanguageFeatureProviderMetadata {
	provideDefinition(request: LanguageLocationRequest, signal: AbortSignal): readonly LanguageLocation[] | Promise<readonly LanguageLocation[]>;
}

export interface LanguageDeclarationProvider extends LanguageFeatureProviderMetadata {
	provideDeclaration(request: LanguageLocationRequest, signal: AbortSignal): readonly LanguageLocation[] | Promise<readonly LanguageLocation[]>;
}

export interface LanguageImplementationProvider extends LanguageFeatureProviderMetadata {
	provideImplementation(request: LanguageLocationRequest, signal: AbortSignal): readonly LanguageLocation[] | Promise<readonly LanguageLocation[]>;
}

export interface LanguageTypeDefinitionProvider extends LanguageFeatureProviderMetadata {
	provideTypeDefinition(request: LanguageLocationRequest, signal: AbortSignal): readonly LanguageLocation[] | Promise<readonly LanguageLocation[]>;
}

export interface LanguageReferenceProvider extends LanguageFeatureProviderMetadata {
	provideReferences(request: LanguageReferenceRequest, signal: AbortSignal): readonly LanguageLocation[] | Promise<readonly LanguageLocation[]>;
}

export interface LanguageNavigationProviderRegistries {
	readonly definitions: OwnedLanguageFeatureProviderRegistry<LanguageDefinitionProvider>;
	readonly declarations: OwnedLanguageFeatureProviderRegistry<LanguageDeclarationProvider>;
	readonly implementations: OwnedLanguageFeatureProviderRegistry<LanguageImplementationProvider>;
	readonly typeDefinitions: OwnedLanguageFeatureProviderRegistry<LanguageTypeDefinitionProvider>;
	readonly references: OwnedLanguageFeatureProviderRegistry<LanguageReferenceProvider>;
}

/** Coordinates cancellable cross-resource language requests for one source model. */
export class LanguageNavigationService extends Disposable {
	constructor(private readonly model: TextModel, private readonly resource: URI, private readonly providers: LanguageNavigationProviderRegistries) {
		super();
	}

	provideDefinition(languageId: string, position: Position, signal: AbortSignal = new AbortController().signal): Promise<readonly LanguageLocation[]> {
		return this.collect(languageId, position, signal, this.providers.definitions, (provider, request) => provider.provideDefinition(request, signal));
	}

	provideDeclaration(languageId: string, position: Position, signal: AbortSignal = new AbortController().signal): Promise<readonly LanguageLocation[]> {
		return this.collect(languageId, position, signal, this.providers.declarations, (provider, request) => provider.provideDeclaration(request, signal));
	}

	provideImplementation(languageId: string, position: Position, signal: AbortSignal = new AbortController().signal): Promise<readonly LanguageLocation[]> {
		return this.collect(languageId, position, signal, this.providers.implementations, (provider, request) => provider.provideImplementation(request, signal));
	}

	provideTypeDefinition(languageId: string, position: Position, signal: AbortSignal = new AbortController().signal): Promise<readonly LanguageLocation[]> {
		return this.collect(languageId, position, signal, this.providers.typeDefinitions, (provider, request) => provider.provideTypeDefinition(request, signal));
	}

	async provideReferences(languageId: string, position: Position, includeDeclaration: boolean, signal: AbortSignal = new AbortController().signal): Promise<readonly LanguageLocation[]> {
		const request = { ...createLanguageFeatureRequest(this.model, languageId, signal), resource: this.resource, position, includeDeclaration };
		return this.collectRequest(languageId, request, this.providers.references, (provider) => provider.provideReferences(request, signal));
	}

	private async collect<TProvider extends LanguageFeatureProviderMetadata>(languageId: string, position: Position, signal: AbortSignal, registry: OwnedLanguageFeatureProviderRegistry<TProvider>, provide: (provider: TProvider, request: LanguageLocationRequest) => readonly LanguageLocation[] | Promise<readonly LanguageLocation[]>): Promise<readonly LanguageLocation[]> {
		const request = { ...createLanguageFeatureRequest(this.model, languageId, signal), resource: this.resource, position };
		return this.collectRequest(languageId, request, registry, provider => provide(provider, request));
	}

	private async collectRequest<TProvider extends LanguageFeatureProviderMetadata>(languageId: string, request: LanguageLocationRequest, registry: OwnedLanguageFeatureProviderRegistry<TProvider>, provide: (provider: TProvider) => readonly LanguageLocation[] | Promise<readonly LanguageLocation[]>): Promise<readonly LanguageLocation[]> {
		const locations: LanguageLocation[] = [];
		for (const provider of registry.getProviders(languageId)) {
			if (!isLanguageFeatureRequestCurrent(request)) return Object.freeze([]);
			const provided = await provide(provider);
			if (!isLanguageFeatureRequestCurrent(request)) return Object.freeze([]);
			locations.push(...provided.map(normalizeLanguageLocation));
		}
		return deduplicateLocations(locations);
	}
}

function normalizeLanguageLocation(location: LanguageLocation): LanguageLocation {
	if (!location || typeof location !== "object" || !location.resource || !(location.range instanceof Object)) throw new TypeError("Language location requires a resource and range");
	const range = Range.fromPositions(location.range.getStartPosition(), location.range.getEndPosition());
	const selectionRange = location.selectionRange ? Range.fromPositions(location.selectionRange.getStartPosition(), location.selectionRange.getEndPosition()) : undefined;
	if (selectionRange && !range.containsRange(selectionRange)) throw new RangeError("Language location selection must be contained by its target range");
	return Object.freeze({ resource: location.resource, range, ...(selectionRange ? { selectionRange } : {}) });
}

function deduplicateLocations(locations: readonly LanguageLocation[]): readonly LanguageLocation[] {
	const keys = new Set<string>();
	const result: LanguageLocation[] = [];
	for (const location of locations) {
		const selection = location.selectionRange ?? location.range;
		const key = `${location.resource.toString()}\u0000${location.range.getStartPosition().lineNumber}:${location.range.getStartPosition().column}:${location.range.getEndPosition().lineNumber}:${location.range.getEndPosition().column}:${selection.getStartPosition().lineNumber}:${selection.getStartPosition().column}:${selection.getEndPosition().lineNumber}:${selection.getEndPosition().column}`;
		if (keys.has(key)) continue;
		keys.add(key);
		result.push(location);
	}
	return Object.freeze(result);
}
