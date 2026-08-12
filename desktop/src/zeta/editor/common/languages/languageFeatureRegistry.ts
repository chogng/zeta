import { DisposableOwner, toDisposable, type IDisposable } from "../../../base/common/lifecycle.js";
import { assertLanguageId, assertLanguageSelector } from "./languageId.js";

/** Minimal metadata shared by all provider registries owned by the language layer. */
export interface LanguageFeatureProviderMetadata {
  readonly languageIds: readonly string[];
  readonly providerId?: string;
}

/** Reusable registry for language providers without Workbench or transport dependencies. */
export class LanguageFeatureProviderRegistry<TProvider extends LanguageFeatureProviderMetadata> extends DisposableOwner {
  private readonly providers = new Map<number, TProvider>();
  private nextProviderHandle = 1;

  constructor() {
    super();
    this.defer(() => this.providers.clear());
  }

  register(provider: TProvider): IDisposable {
    validateProvider(provider);
    const handle = this.nextProviderHandle++;
    this.providers.set(handle, provider);
    return toDisposable(() => this.providers.delete(handle));
  }

  getProviders(languageId: string): readonly TProvider[] {
    assertLanguageId(languageId);
    return Object.freeze([...this.providers.values()].filter(provider => matchesLanguage(provider, languageId)));
  }

}

function validateProvider<TProvider extends LanguageFeatureProviderMetadata>(provider: TProvider): void {
  if (!provider || typeof provider !== "object") throw new TypeError("Language feature provider must be an object");
  if (!Array.isArray(provider.languageIds) || provider.languageIds.length === 0) throw new TypeError("Language feature provider must declare language IDs");
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
