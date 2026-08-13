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
      empty.textContent = "No plugins are available.";
      this.element.replaceChildren(empty);
      return;
    }
    const fragment = this.document.createDocumentFragment();
    const available = marketplace.filter(candidate => !candidate.installed);
    if (available.length > 0) {
      const toolbar = this.document.createElement("div");
      toolbar.className = "zeta-marketplace-toolbar";
      const introduction = this.document.createElement("div");
      const title = this.document.createElement("h3");
      title.textContent = "Marketplace";
      const description = this.document.createElement("p");
      description.textContent = "Discover signed capabilities and review their access before installing.";
      introduction.append(title, description);
      const searchLabel = this.document.createElement("label");
      searchLabel.className = "zeta-marketplace-search";
      searchLabel.textContent = "Search Marketplace";
      const search = this.document.createElement("input");
      search.type = "search";
      search.placeholder = "Skills, extensions, publishers…";
      searchLabel.append(search);
      toolbar.append(introduction, searchLabel);
      const results = this.document.createElement("div");
      results.className = "zeta-marketplace-results";
      const showResults = () => {
        const query = search.value.trim().toLocaleLowerCase();
        const matches = available.filter(plugin => matchesSearch(plugin, query));
        if (matches.length === 0) {
          const empty = this.document.createElement("p");
          empty.className = "zeta-settings-message";
          empty.textContent = "No Marketplace plugins match this search.";
          results.replaceChildren(empty);
          return;
        }
        results.replaceChildren(...matches.map(plugin => this.marketplaceCard(catalog, plugin)));
      };
      this.rows.add(addDisposableListener(search, "input", showResults));
      showResults();
      fragment.append(toolbar, results);
    }
    if (catalog.packages.length > 0) {
      const title = this.document.createElement("h3");
      title.textContent = "Installed";
      fragment.append(title);
    }
    for (const plugin of catalog.packages) {
      const listing = marketplace.find(candidate => candidate.id === plugin.id && candidate.version === plugin.version && candidate.digest === plugin.digest);
      fragment.append(this.pluginCard(catalog, plugin, listing));
    }
    this.element.replaceChildren(fragment);
  }

  private marketplaceCard(catalog: PluginCatalogView, plugin: PluginMarketplacePackageView): HTMLElement {
    const card = this.document.createElement("section");
    card.className = "zeta-integration-card";
    const heading = this.document.createElement("div");
    heading.className = "zeta-integration-heading";
    const title = this.document.createElement("h4");
    title.textContent = plugin.displayName;
    const trust = this.document.createElement("span");
    trust.className = `zeta-marketplace-badge is-${plugin.marketplaceTrust}`;
    trust.textContent = marketplaceTrustLabel(plugin.marketplaceTrust);
    heading.append(title, trust);
    const source = this.document.createElement("p");
    source.className = "zeta-connector-description";
    source.textContent = `${plugin.id} · ${plugin.version} · ${plugin.publisher}`;
    const description = this.document.createElement("p");
    description.className = "zeta-marketplace-description";
    description.textContent = plugin.description ?? "No description provided.";
    const capabilities = capabilitySummary(this.document, plugin);
    const access = this.document.createElement("div");
    access.className = "zeta-marketplace-access";
    const accessTitle = this.document.createElement("strong");
    accessTitle.textContent = "Requested access";
    const accessSummary = this.document.createElement("p");
    accessSummary.textContent = permissionSummary(plugin);
    access.append(accessTitle, accessSummary);
    const details = this.document.createElement("p");
    details.className = "zeta-marketplace-details";
    details.textContent = `${plugin.marketplaceId} · Zeta ${plugin.compatibilityZeta} · ${plugin.license ?? "License not specified"} · ${formatBytes(plugin.packageSizeBytes)}`;
    const feedback = this.document.createElement("p");
    feedback.className = "zeta-integration-feedback";
    feedback.setAttribute("role", "status");
    const installedVersions = catalog.packages.filter(installed => installed.id === plugin.id);
    const newest = installedVersions.sort((left, right) => left.version.localeCompare(right.version, undefined, { numeric: true })).at(-1);
    const isUpdate = newest !== undefined && newest.version !== plugin.version;
    card.append(heading, source, description, capabilities, access, details);
    if (plugin.revoked) {
      const revoked = this.document.createElement("p");
      revoked.className = "zeta-integration-state is-unavailable";
      revoked.textContent = "This exact package has been revoked and cannot be installed.";
      card.append(revoked);
    } else {
      card.append(this.action(isUpdate ? "Stage update" : "Install", () => isUpdate ? this.plugins.update(plugin, catalog.revision) : this.plugins.install(plugin, catalog.revision), feedback));
    }
    card.append(feedback);
    return card;
  }

  private pluginCard(catalog: PluginCatalogView, plugin: PluginPackageView, listing?: PluginMarketplacePackageView): HTMLElement {
    const card = this.document.createElement("section");
    card.className = "zeta-integration-card";
    const heading = this.document.createElement("div");
    heading.className = "zeta-integration-heading";
    const title = this.document.createElement("h4");
    title.textContent = listing?.displayName ?? `${plugin.id} · ${plugin.version}`;
    const state = this.document.createElement("span");
    state.className = `zeta-integration-state is-${plugin.effective ? "connected" : "disconnected"}`;
    state.textContent = status(plugin);
    heading.append(title, state);
    const feedback = this.document.createElement("p");
    feedback.className = "zeta-integration-feedback";
    feedback.setAttribute("role", "status");
    card.append(heading);
    if (listing) {
      const source = this.document.createElement("p");
      source.className = "zeta-connector-description";
      source.textContent = `${plugin.id} · ${plugin.version} · ${listing.publisher}`;
      card.append(source, capabilitySummary(this.document, listing));
    }
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

function marketplaceTrustLabel(trust: PluginMarketplacePackageView["marketplaceTrust"]): string {
  switch (trust) {
    case "productManaged": return "Zeta managed";
    case "verifiedExternal": return "Verified external source";
    case "localDevelopment": return "Local development";
  }
}

function capabilitySummary(document: Document, plugin: PluginMarketplacePackageView): HTMLElement {
  const summary = document.createElement("div");
  summary.className = "zeta-marketplace-capabilities";
  for (const label of capabilityLabels(plugin)) {
    const badge = document.createElement("span");
    badge.textContent = label;
    summary.append(badge);
  }
  return summary;
}

function capabilityLabels(plugin: PluginMarketplacePackageView): string[] {
  const contributions = plugin.contributions;
  return [
    countLabel(contributions.skills, "Skill"),
    countLabel(contributions.editorExtensions, "Editor extension"),
    countLabel(contributions.mcpServers, "MCP server"),
    countLabel(contributions.connectors, "Connector"),
    countLabel(contributions.assets, "Asset"),
  ].filter((label): label is string => label !== undefined);
}

function countLabel(count: number, noun: string): string | undefined {
  if (count === 0) return undefined;
  return `${count} ${noun}${count === 1 ? "" : "s"}`;
}

function permissionSummary(plugin: PluginMarketplacePackageView): string {
  const permissions = plugin.permissions.map(permission => {
    switch (permission.type) {
      case "process": return `run ${permission.executable}`;
      case "workspace": return `${permission.access} workspace files`;
      case "network": return `connect to ${permission.hosts.join(", ")}`;
    }
  });
  if (plugin.credentialSlots.length > 0) permissions.push(`${plugin.credentialSlots.length} credential ${plugin.credentialSlots.length === 1 ? "slot" : "slots"}`);
  return permissions.length === 0 ? "None" : permissions.join(" · ");
}

function matchesSearch(plugin: PluginMarketplacePackageView, query: string): boolean {
  if (query.length === 0) return true;
  return [plugin.id, plugin.publisher, plugin.displayName, plugin.description ?? "", ...capabilityLabels(plugin)].some(value => value.toLocaleLowerCase().includes(query));
}

function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}
