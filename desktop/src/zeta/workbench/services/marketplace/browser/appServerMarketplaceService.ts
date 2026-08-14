import type { IMarketplaceApi } from "../../../../platform/marketplace/common/marketplaceApi.js";
import type { IMarketplaceService, MarketplaceAcquiredCapability, MarketplaceInstalledPackage, MarketplacePackageDetails, MarketplacePackageSummary } from "../../../../platform/marketplace/common/marketplaceService.js";

/** Mechanical adapter from App Server DTOs to frontend-owned Marketplace values. */
export class AppServerMarketplaceService implements IMarketplaceService {
  constructor(private readonly api: IMarketplaceApi) {}

  async search(query: string, packageType?: string, limit?: number): Promise<readonly MarketplacePackageSummary[]> {
    return (await this.api.search({ query, packageType: packageType ?? null, limit: limit ?? null })).packages;
  }

  get(packageId: string, version?: string): Promise<MarketplacePackageDetails> {
    return this.api.get({ packageId, version: version ?? null });
  }

  download(packageId: string, version?: string) {
    return this.api.download({ packageId, version: version ?? null });
  }

  install(packageId: string, version?: string): Promise<MarketplaceInstalledPackage> {
    return this.api.install({ packageId, version: version ?? null });
  }

  update(installationId: string, version?: string): Promise<MarketplaceInstalledPackage> {
    return this.api.update({ installationId, version: version ?? null });
  }

  uninstall(installationId: string, mode: "ifUnused" | "whenUnused" = "whenUnused"): Promise<void> {
    return this.api.uninstall({ installationId, mode });
  }

  async listInstalled(): Promise<readonly MarketplaceInstalledPackage[]> {
    return (await this.api.listInstalled()).packages;
  }

  acquireCapability(capabilityId: string): Promise<MarketplaceAcquiredCapability> {
    return this.api.acquireCapability({ capability: { id: capabilityId } });
  }

  releaseCapability(leaseId: string): Promise<void> {
    return this.api.releaseCapability({ leaseId });
  }

  openResource(leaseId: string, resourceId: string) {
    return this.api.openResource({ leaseId, resource: { id: resourceId } });
  }
}
