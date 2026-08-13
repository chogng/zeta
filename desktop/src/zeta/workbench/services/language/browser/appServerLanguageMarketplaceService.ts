import type { ILanguageApi } from "../../../../platform/language/common/languageApi.js";
import type { ILanguageMarketplaceService, LanguageMarketplaceCatalogView, LanguageMarketplaceEntryView } from "../../../../platform/language/common/languageMarketplaceService.js";

/** Mechanical adapter from App Server DTOs to frontend-owned Language Marketplace views. */
export class AppServerLanguageMarketplaceService implements ILanguageMarketplaceService {
  constructor(private readonly api: ILanguageApi) {}

  async list(): Promise<LanguageMarketplaceCatalogView> {
    const result = await this.api.listMarketplace();
    return {
      revision: result.catalogRevision,
      activationGeneration: result.activationGeneration,
      entries: result.entries,
    };
  }

  async install(entry: LanguageMarketplaceEntryView, expectedRevision: string): Promise<void> {
    await this.api.installMarketplace({
      expectedCatalogRevision: expectedRevision,
      marketplaceId: entry.marketplaceId,
      packageId: entry.packageId,
      version: entry.version,
      digest: entry.digest,
      serverId: entry.serverId,
    });
  }
}
