import type { Event } from "../../../base/common/event.js";
import { createServiceIdentifier } from "../../instantiation/common/instantiation.js";

export interface PluginPackageView {
  readonly id: string;
  readonly version: string;
  readonly digest: string;
  readonly enabled: boolean;
  readonly granted: boolean;
  readonly effective: boolean;
}

export interface PluginCatalogView {
  readonly revision: number;
  readonly activationGeneration: number;
  readonly packages: readonly PluginPackageView[];
}

export interface PluginMarketplacePackageView {
  readonly marketplaceId: string;
  readonly marketplaceMode: "managed" | "localDevelopment";
  readonly id: string;
  readonly version: string;
  readonly digest: string;
  readonly installed: boolean;
}

export interface IPluginService {
  readonly onDidChange: Event<number>;
  list(): Promise<PluginCatalogView>;
  listMarketplace(): Promise<readonly PluginMarketplacePackageView[]>;
  install(plugin: PluginMarketplacePackageView, revision: number): Promise<void>;
  update(plugin: PluginMarketplacePackageView, revision: number): Promise<void>;
  rollback(plugin: PluginPackageView, revision: number): Promise<void>;
  enable(plugin: PluginPackageView, revision: number): Promise<void>;
  disable(plugin: PluginPackageView, revision: number): Promise<void>;
  grant(plugin: PluginPackageView, revision: number): Promise<void>;
  revokeGrant(plugin: PluginPackageView, revision: number): Promise<void>;
  uninstall(plugin: PluginPackageView, revision: number): Promise<void>;
}

export const IPluginService = createServiceIdentifier<IPluginService>("pluginService");
