import type { MarketplaceAcquiredCapabilityDto, MarketplaceArtifactHandleDto, MarketplaceInstalledPackageDto, MarketplaceListInstalledResult, MarketplacePackageDetailsDto, MarketplaceResourceContentDto, MarketplaceSearchResult } from "../../../../../generated/app-server/types.js";
import { invoke } from "../../ipc/electron-browser/rendererIpc.js";
import type { IMarketplaceApi } from "../common/marketplaceApi.js";

export function createMarketplaceApi(): IMarketplaceApi {
  return {
    search: params => invoke<MarketplaceSearchResult>("zeta:marketplace:search", params),
    get: params => invoke<MarketplacePackageDetailsDto>("zeta:marketplace:get", params),
    download: params => invoke<MarketplaceArtifactHandleDto>("zeta:marketplace:download", params),
    install: params => invoke<MarketplaceInstalledPackageDto>("zeta:marketplace:install", params),
    update: params => invoke<MarketplaceInstalledPackageDto>("zeta:marketplace:update", params),
    uninstall: params => invoke<void>("zeta:marketplace:uninstall", params),
    listInstalled: () => invoke<MarketplaceListInstalledResult>("zeta:marketplace:list-installed"),
    acquireCapability: params => invoke<MarketplaceAcquiredCapabilityDto>("zeta:marketplace:acquire-capability", params),
    releaseCapability: params => invoke<void>("zeta:marketplace:release-capability", params),
    openResource: params => invoke<MarketplaceResourceContentDto>("zeta:marketplace:open-resource", params),
  };
}
