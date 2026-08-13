import { addDisposableListener } from "../../../../base/browser/dom.js";
import { DisposableOwner, ResettableDisposableGroup } from "../../../../base/common/lifecycle.js";
import type { IPluginService, PluginCatalogView, PluginMarketplacePackageView, PluginPackageView } from "../../../../platform/plugins/common/pluginService.js";
import "./media/connectorSettings.css";

export class PluginSettingsPane extends DisposableOwner {
  readonly element: HTMLDivElement;
  private readonly rows = this.own(new ResettableDisposableGroup());
  private loadGeneration = 0;

  constructor(private readonly document: Document, private readonly plugins: IPluginService) {
    super();
    this.element = document.createElement("div");
    this.element.className = "zeta-integration-settings";
    this.own(plugins.onDidChange(() => void this.reload()));
    void this.reload();
    this.defer(() => this.element.remove());
  }

  private async reload(): Promise<void> {
    const loadGeneration = ++this.loadGeneration;
    const loading = this.document.createElement("p");
    loading.className = "zeta-settings-message";
    loading.textContent = "Loading plugins…";
    this.element.replaceChildren(loading);
    const loaded = await Promise.all([this.plugins.list(), this.plugins.listMarketplace()]).catch((error: unknown) => {
      if (loadGeneration !== this.loadGeneration) return undefined;
      loading.textContent = error instanceof Error ? `Unable to load plugins: ${error.message}` : "Unable to load plugins.";
      return undefined;
    });
    if (!loaded || loadGeneration !== this.loadGeneration) return;
    this.render(loaded[0], loaded[1]);
  }

  private render(catalog: PluginCatalogView, marketplace: readonly PluginMarketplacePackageView[]): void {
    this.rows.clear();
    if (catalog.packages.length === 0 && marketplace.length === 0) {
      const empty = this.document.createElement("p");
      empty.className = "zeta-settings-message";
      empty.textContent = "No plugins are installed.";
      this.element.replaceChildren(empty);
      return;
    }
    const fragment = this.document.createDocumentFragment();
    const available = marketplace.filter(candidate => !candidate.installed);
    if (available.length > 0) {
      const title = this.document.createElement("h3");
      title.textContent = "Marketplace";
      fragment.append(title);
      for (const plugin of available) fragment.append(this.marketplaceCard(catalog, plugin));
    }
    if (catalog.packages.length > 0) {
      const title = this.document.createElement("h3");
      title.textContent = "Installed";
      fragment.append(title);
    }
    for (const plugin of catalog.packages) fragment.append(this.pluginCard(catalog, plugin));
    this.element.replaceChildren(fragment);
  }

  private marketplaceCard(catalog: PluginCatalogView, plugin: PluginMarketplacePackageView): HTMLElement {
    const card = this.document.createElement("section");
    card.className = "zeta-integration-card";
    const title = this.document.createElement("h4");
    title.textContent = `${plugin.id} · ${plugin.version}`;
    const source = this.document.createElement("p");
    source.className = "zeta-connector-description";
    source.textContent = `${plugin.marketplaceId} · ${marketplaceModeLabel(plugin.marketplaceMode)}`;
    const feedback = this.document.createElement("p");
    feedback.className = "zeta-integration-feedback";
    feedback.setAttribute("role", "status");
    const installedVersions = catalog.packages.filter(installed => installed.id === plugin.id);
    const newest = installedVersions.sort((left, right) => left.version.localeCompare(right.version, undefined, { numeric: true })).at(-1);
    const isUpdate = newest !== undefined && newest.version !== plugin.version;
    card.append(title, source, this.action(isUpdate ? "Stage update" : "Install", () => isUpdate ? this.plugins.update(plugin, catalog.revision) : this.plugins.install(plugin, catalog.revision), feedback), feedback);
    return card;
  }

  private pluginCard(catalog: PluginCatalogView, plugin: PluginPackageView): HTMLElement {
    const card = this.document.createElement("section");
    card.className = "zeta-integration-card";
    const heading = this.document.createElement("div");
    heading.className = "zeta-integration-heading";
    const title = this.document.createElement("h4");
    title.textContent = `${plugin.id} · ${plugin.version}`;
    const state = this.document.createElement("span");
    state.className = `zeta-integration-state is-${plugin.effective ? "connected" : "disconnected"}`;
    state.textContent = status(plugin);
    heading.append(title, state);
    const feedback = this.document.createElement("p");
    feedback.className = "zeta-integration-feedback";
    feedback.setAttribute("role", "status");
    card.append(heading);
    if (!plugin.granted) card.append(this.action("Grant", () => this.plugins.grant(plugin, catalog.revision), feedback));
    if (plugin.granted) card.append(this.action("Revoke grant", () => this.plugins.revokeGrant(plugin, catalog.revision), feedback));
    const enabledVersion = catalog.packages.find(candidate => candidate.id === plugin.id && candidate.enabled);
    const rollback = plugin.granted && !plugin.enabled && enabledVersion !== undefined && plugin.version.localeCompare(enabledVersion.version, undefined, { numeric: true }) < 0;
    if (rollback) card.append(this.action("Rollback to this version", () => this.plugins.rollback(plugin, catalog.revision), feedback));
    if (!plugin.enabled && !rollback) card.append(this.action("Enable", () => this.plugins.enable(plugin, catalog.revision), feedback));
    if (plugin.enabled) card.append(this.action("Disable", () => this.plugins.disable(plugin, catalog.revision), feedback));
    if (!plugin.enabled && !plugin.granted) card.append(this.action("Uninstall", () => this.plugins.uninstall(plugin, catalog.revision), feedback, true));
    card.append(feedback);
    return card;
  }

  private action(label: string, invoke: () => Promise<void>, feedback: HTMLElement, danger = false): HTMLButtonElement {
    const button = this.document.createElement("button");
    button.type = "button";
    button.className = `zeta-theme-action${danger ? " is-danger" : ""}`;
    button.textContent = label;
    this.rows.add(addDisposableListener(button, "click", () => {
      button.disabled = true;
      feedback.textContent = `${label}…`;
      void invoke().then(() => this.reload()).catch((error: unknown) => {
        button.disabled = false;
        feedback.textContent = error instanceof Error ? `${label} failed: ${error.message}` : `${label} failed.`;
      });
    }));
    return button;
  }
}

function status(plugin: PluginPackageView): string {
  if (plugin.revoked) return "Revoked";
  if (plugin.effective) return "Active";
  if (plugin.enabled && !plugin.granted) return "Enabled · grant required";
  if (!plugin.enabled && plugin.granted) return "Granted · disabled";
  return "Installed";
}

function marketplaceModeLabel(mode: PluginMarketplacePackageView["marketplaceMode"]): string {
  switch (mode) {
    case "managed": return "Managed Marketplace";
    case "remoteManaged": return "Verified remote Marketplace";
    case "localDevelopment": return "Local development";
  }
}
