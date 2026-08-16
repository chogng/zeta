import "./media/marketplaceSettings.css";
import { addDisposableListener } from "../../../../base/browser/dom.js";
import { appendIcon } from "../../../../base/browser/ui/icon/icon.js";
import { TabList } from "../../../../base/browser/ui/tablist/tabList.js";
import { lxiconsLibrary } from "../../../../base/common/lxiconsLibrary.js";
import { DisposableOwner } from "../../../../base/common/lifecycle.js";
import type { IMarketplaceService, MarketplaceBrowseSnapshot, MarketplaceInstalledPackage, MarketplacePackageDetails, MarketplacePackageSummary } from "../../../../platform/marketplace/common/marketplaceService.js";

const packageTypeFilters = [
  { label: "All", packageType: undefined },
  { label: "Plugins", packageType: "plugin" },
  { label: "MCPs", packageType: "mcp" },
  { label: "Skills", packageType: "skill" },
  { label: "Languages", packageType: "language" },
  { label: "Themes", packageType: "theme" },
] as const;

/** Generic package discovery and installation surface backed only by Marketplace business APIs. */
export class MarketplaceSettingsPane extends DisposableOwner {
  readonly element: HTMLElement;
  private readonly input: HTMLInputElement;
  private readonly status: HTMLParagraphElement;
  private readonly results: HTMLDivElement;
  private readonly filterTabs: TabList<string> | undefined;
  private query = "";
  private selectedPackageType: string | undefined;
  private reloadGeneration = 0;
  private isDisposed = false;

  constructor(private readonly document: Document, private readonly marketplace: IMarketplaceService, private readonly fixedPackageType?: string) {
    super();
    this.selectedPackageType = fixedPackageType;
    this.element = document.createElement("section");
    this.element.className = "zeta-package-marketplace";
    const toolbar = document.createElement("form");
    toolbar.className = "zeta-package-marketplace-toolbar";
    const searchControl = document.createElement("div");
    searchControl.className = "zeta-package-marketplace-search-control";
    appendIcon(lxiconsLibrary.search, searchControl);
    this.input = document.createElement("input");
    this.input.type = "search";
    this.input.placeholder = fixedPackageType === "language" ? "Search language extensions" : "Search Plugins, Skills, MCPs…";
    this.input.setAttribute("aria-label", "Search Marketplace");
    searchControl.append(this.input);
    const search = document.createElement("button");
    search.type = "submit";
    search.textContent = "Browse Marketplace";
    toolbar.append(searchControl, search);
    const filters = document.createElement("div");
    filters.className = "zeta-package-marketplace-filters";
    this.filterTabs = fixedPackageType ? undefined : this.own(new TabList<string>({
      ownerDocument: document,
      ariaLabel: "Marketplace package types",
      onActivate: (packageType) => {
        const nextPackageType = packageType || undefined;
        if (this.selectedPackageType === nextPackageType) return;
        this.selectedPackageType = nextPackageType;
        this.updateFilterState();
        void this.reload();
      },
    }));
    if (this.filterTabs) {
      filters.append(this.filterTabs.element);
      this.updateFilterState();
    }
    this.status = document.createElement("p");
    this.status.className = "zeta-package-marketplace-status";
    this.status.setAttribute("role", "status");
    this.results = document.createElement("div");
    this.results.className = "zeta-package-marketplace-results";
    this.own(addDisposableListener(toolbar, "submit", (event: SubmitEvent) => {
      event.preventDefault();
      this.query = this.input.value.trim();
      void this.reload(true);
    }));
    this.element.append(toolbar);
    if (!fixedPackageType) this.element.append(filters);
    this.element.append(this.status, this.results);
    this.defer(() => { this.isDisposed = true; });
    void this.reload();
  }

  private async reload(forceRefresh = false): Promise<void> {
    const generation = ++this.reloadGeneration;
    const cached = forceRefresh ? undefined : this.marketplace.cachedBrowse(this.query, this.selectedPackageType, 100);
    if (cached) {
      this.render(cached);
      return;
    }
    this.status.textContent = "Loading Marketplace…";
    this.results.classList.remove("empty");
    this.results.replaceChildren();
    const loaded = await (forceRefresh
      ? this.marketplace.refreshBrowse(this.query, this.selectedPackageType, 100)
      : this.marketplace.browse(this.query, this.selectedPackageType, 100)).catch((error: unknown) => {
      if (generation !== this.reloadGeneration || this.isDisposed) return undefined;
      this.status.textContent = error instanceof Error ? `Marketplace unavailable: ${error.message}` : "Marketplace unavailable.";
      return undefined;
    });
    if (!loaded || this.isDisposed || generation !== this.reloadGeneration) return;
    this.render(loaded);
  }

  private render(snapshot: MarketplaceBrowseSnapshot): void {
    const packages = snapshot.packages;
    this.status.textContent = packages.length === 0 ? "No matching packages." : `${packages.length} package${packages.length === 1 ? "" : "s"}`;
    if (packages.length === 0) {
      this.results.classList.add("empty");
      this.results.replaceChildren(this.emptyState());
      return;
    }
    this.results.classList.remove("empty");
    this.results.replaceChildren(...packages.map(packageValue => this.packageCard(packageValue.summary, packageValue.details, snapshot.installed)));
  }

  private updateFilterState(): void {
    this.filterTabs?.setTabs(
      packageTypeFilters.map((filter) => {
        const id = marketplaceFilterId(filter.packageType);
        return {
          id,
          value: filter.packageType ?? "",
          label: filter.label,
          tabId: `${id}-tab`,
        };
      }),
      marketplaceFilterId(this.selectedPackageType),
    );
  }

  private emptyState(): HTMLElement {
    const empty = this.document.createElement("div");
    empty.className = "zeta-package-marketplace-empty-state";
    const heading = this.document.createElement("h4");
    heading.textContent = this.selectedPackageType ? `No ${this.selectedPackageType} packages found` : "Explore Marketplace packages";
    const description = this.document.createElement("p");
    description.textContent = this.query
      ? "Try a different search or clear the current filters."
      : "Install Plugins, Skills, MCP servers, language support, and themes from the signed catalog.";
    const reset = this.document.createElement("button");
    reset.type = "button";
    reset.textContent = "Clear filters";
    this.own(addDisposableListener(reset, "click", () => {
      this.query = "";
      this.input.value = "";
      this.selectedPackageType = this.fixedPackageType;
      this.updateFilterState();
      void this.reload(true);
    }));
    empty.append(heading, description, reset);
    return empty;
  }

  private packageCard(summary: MarketplacePackageSummary, details: MarketplacePackageDetails | undefined, installed: readonly MarketplaceInstalledPackage[]): HTMLElement {
    const card = this.document.createElement("article");
    card.className = "zeta-package-marketplace-card";
    const heading = this.document.createElement("h4");
    heading.textContent = summary.displayName;
    const description = this.document.createElement("p");
    description.className = "zeta-package-marketplace-description";
    description.textContent = summary.description;
    const metadata = this.document.createElement("p");
    metadata.className = "zeta-package-marketplace-metadata";
    metadata.textContent = `${summary.id} · ${summary.version} · ${summary.packageType}${details ? ` · ${details.license}` : ""}`;
    const provenance = this.document.createElement("p");
    provenance.className = "zeta-package-marketplace-metadata";
    provenance.textContent = details?.upstream?.registry === "officialMcp"
      ? `Listed in the official MCP Registry · ${details.upstream.name}@${details.upstream.version}`
      : details?.source === "official"
        ? "Marketplace official package"
        : "Third-party package";
    const capabilities = this.document.createElement("ul");
    capabilities.className = "zeta-package-marketplace-capabilities";
    for (const capability of details?.capabilities ?? []) {
      const item = this.document.createElement("li");
      item.textContent = `${capability.kind}: ${capability.id}`;
      capabilities.append(item);
    }
    const active = installed.find(candidate => candidate.package.id === summary.id && candidate.package.version === summary.version);
    const actions = this.document.createElement("div");
    actions.className = "zeta-package-marketplace-actions";
    const lifecycle = this.document.createElement("span");
    lifecycle.className = "zeta-package-marketplace-lifecycle";
    lifecycle.textContent = active ? (active.state === "installed" ? "Installed · activation is separate" : "Removal pending") : "Available";
    const action = this.document.createElement("button");
    action.type = "button";
    action.textContent = active ? "Uninstall" : "Install";
    action.disabled = active?.state === "pendingRemoval";
    this.own(addDisposableListener(action, "click", () => {
      action.disabled = true;
      const operation = active
        ? this.marketplace.uninstall(active.installationId)
        : this.marketplace.install(summary.id, summary.version).then(() => undefined);
      void operation.then(() => this.reload()).catch((error: unknown) => {
        this.status.textContent = error instanceof Error ? `Marketplace operation failed: ${error.message}` : "Marketplace operation failed.";
        action.disabled = false;
      });
    }));
    actions.append(lifecycle, action);
    card.append(heading, description, metadata, provenance);
    if (capabilities.childElementCount > 0) card.append(capabilities);
    card.append(actions);
    return card;
  }
}

function marketplaceFilterId(packageType: string | undefined): string {
  return `marketplace-filter-${packageType ?? "all"}`;
}
