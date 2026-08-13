import { createServiceIdentifier } from "../../instantiation/common/instantiation.js";

export type LanguageMarketplaceCompatibilityView =
  | { readonly status: "compatible" }
  | { readonly status: "incompatible"; readonly reason: string };

export interface LanguageMarketplaceEntryView {
  readonly marketplaceId: string;
  readonly packageId: string;
  readonly version: string;
  readonly digest: string;
  readonly displayName: string;
  readonly description: string;
  readonly license: string;
  readonly serverId: string;
  readonly languages: readonly string[];
  readonly fileExtensions: readonly string[];
  readonly compatibility: LanguageMarketplaceCompatibilityView;
  readonly installed: boolean;
  readonly active: boolean;
}

export interface LanguageMarketplaceCatalogView {
  readonly revision: string;
  readonly activationGeneration: number;
  readonly entries: readonly LanguageMarketplaceEntryView[];
}

/** Frontend-owned language package discovery and confirmed installation contract. */
export interface ILanguageMarketplaceService {
  list(): Promise<LanguageMarketplaceCatalogView>;
  install(entry: LanguageMarketplaceEntryView, expectedRevision: string): Promise<void>;
}

export const ILanguageMarketplaceService = createServiceIdentifier<ILanguageMarketplaceService>("languageMarketplaceService");
