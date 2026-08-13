import type { Event } from "../../../base/common/event.js";
import { createServiceIdentifier } from "../../instantiation/common/instantiation.js";

export interface PluginPackageView {
  readonly id: string;
  readonly version: string;
  readonly digest: string;
  readonly enabled: boolean;
  readonly granted: boolean;
  readonly effective: boolean;
  readonly revoked: boolean;
}

export interface PluginCatalogView {
  readonly revision: number;
  readonly activationGeneration: number;
  readonly packages: readonly PluginPackageView[];
}

export interface PluginContributionSummaryView {
  readonly skills: number;
  readonly mcpServers: number;
  readonly connectors: number;
  readonly assets: number;
  readonly editorExtensions: number;
}

export type PluginPermissionView =
  | { readonly type: "process"; readonly executable: string }
  | { readonly type: "workspace"; readonly access: "read" | "write" }
  | { readonly type: "network"; readonly hosts: readonly string[] };

export interface PluginCredentialSlotView {
  readonly name: string;
  readonly kind: "secretText";
  readonly requiredFor: readonly string[];
}

export interface PluginMarketplacePackageView {
  readonly marketplaceId: string;
  readonly marketplaceMode: "managed" | "remoteManaged" | "localDevelopment";
  readonly marketplaceTrust: "productManaged" | "verifiedExternal" | "localDevelopment";
  readonly marketplaceRevision: string;
  readonly id: string;
  readonly publisher: string;
  readonly version: string;
  readonly digest: string;
  readonly displayName: string;
  readonly description: string | null;
  readonly license: string | null;
  readonly compatibilityZeta: string;
  readonly contributions: PluginContributionSummaryView;
  readonly permissions: readonly PluginPermissionView[];
  readonly credentialSlots: readonly PluginCredentialSlotView[];
  readonly packageFileCount: number;
  readonly packageSizeBytes: number;
  readonly installed: boolean;
  readonly enabled: boolean;
  readonly granted: boolean;
  readonly effective: boolean;
  readonly revoked: boolean;
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
