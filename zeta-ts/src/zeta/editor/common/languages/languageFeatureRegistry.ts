import { isNonEmptyArray } from "../../../base/common/arrays.js";
import { DisposableOwner, toDisposable, type IDisposable } from "../../../base/common/lifecycle.js";
import { assertLanguageId, assertLanguageSelector } from "./languageId.js";

/** Minimal metadata shared by all provider registries owned by the language layer. */
export interface LanguageFeatureProviderMetadata {
	readonly languageIds: readonly string[];
	readonly providerId?: string;
}

/** One caller-owned provider set that can be replaced without exposing the registry. */
export interface LanguageFeatureProviderRegistration<TProvider extends LanguageFeatureProviderMetadata> extends IDisposable {
	replace(providers: readonly TProvider[]): void;
}

interface OwnedLanguageFeatureProvider<TProvider> {
	readonly owner: object;
	readonly provider: TProvider;
}

/** Reusable registry for language providers without Workbench or transport dependencies. */
export class LanguageFeatureProviderRegistry<TProvider extends LanguageFeatureProviderMetadata> extends DisposableOwner {
	private readonly providers = new Map<number, OwnedLanguageFeatureProvider<TProvider>>();
	private nextProviderHandle = 1;

	constructor() {
		super();
		this.defer(() => this.providers.clear());
	}

	register(provider: TProvider): IDisposable {
		return this.registerGroup([provider]);
	}

	registerGroup(providers: readonly TProvider[]): LanguageFeatureProviderRegistration<TProvider> {
		const owner = Object.freeze({});
		this.replace(owner, providers);
		let disposed = false;
		const registration = toDisposable(() => {
			if (disposed) return;
			disposed = true;
			this.deleteOwner(owner);
		}) as LanguageFeatureProviderRegistration<TProvider>;
		registration.replace = replacement => {
			if (disposed) throw new ReferenceError("Language feature provider registration is already disposed");
			this.replace(owner, replacement);
		};
		return registration;
	}

	getProviders(languageId: string): readonly TProvider[] {
		if (languageId !== "*") assertLanguageId(languageId);
		return Object.freeze([...this.providers.values()].map(entry => entry.provider).filter(provider => languageId === "*" ? provider.languageIds.includes("*") : matchesLanguage(provider, languageId)));
	}

	private replace(owner: object, providers: readonly TProvider[]): void {
		if (!Array.isArray(providers)) throw new TypeError("Language feature providers must be an array");
		for (const provider of providers) validateProvider(provider);
		this.deleteOwner(owner);
		for (const provider of providers) this.providers.set(this.nextProviderHandle++, { owner, provider });
	}

	private deleteOwner(owner: object): void {
		for (const [handle, entry] of this.providers) if (entry.owner === owner) this.providers.delete(handle);
	}
}

function validateProvider<TProvider extends LanguageFeatureProviderMetadata>(provider: TProvider): void {
	if (!provider || typeof provider !== "object") throw new TypeError("Language feature provider must be an object");
	if (!isNonEmptyArray(provider.languageIds)) throw new TypeError("Language feature provider must declare language IDs");
	const languageIds = provider.languageIds.map(languageId => {
		assertLanguageSelector(languageId);
		return languageId;
	});
	if (new Set(languageIds).size !== languageIds.length) throw new RangeError("Language feature provider language IDs must be unique");
	if (provider.providerId !== undefined && (typeof provider.providerId !== "string" || provider.providerId.trim().length === 0)) throw new TypeError("Language feature provider ID must be a non-empty string");
}

function matchesLanguage(provider: LanguageFeatureProviderMetadata, languageId: string): boolean {
	return provider.languageIds.includes("*") || provider.languageIds.includes(languageId);
}
