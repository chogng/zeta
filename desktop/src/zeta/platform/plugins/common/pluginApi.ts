import type { PluginCommandResultDto, PluginListResult, PluginMarketplaceCommandParams, PluginMarketplaceListResult, PluginPackageCommandParams } from "../../../../../generated/app-server/types.js";

export interface IPluginApi {
  list(): Promise<PluginListResult>;
  listMarketplace(): Promise<PluginMarketplaceListResult>;
  install(params: PluginMarketplaceCommandParams): Promise<PluginCommandResultDto>;
  update(params: PluginMarketplaceCommandParams): Promise<PluginCommandResultDto>;
  rollback(params: PluginPackageCommandParams): Promise<PluginCommandResultDto>;
  enable(params: PluginPackageCommandParams): Promise<PluginCommandResultDto>;
  disable(params: PluginPackageCommandParams): Promise<PluginCommandResultDto>;
  grant(params: PluginPackageCommandParams): Promise<PluginCommandResultDto>;
  revokeGrant(params: PluginPackageCommandParams): Promise<PluginCommandResultDto>;
  uninstall(params: PluginPackageCommandParams): Promise<PluginCommandResultDto>;
}
