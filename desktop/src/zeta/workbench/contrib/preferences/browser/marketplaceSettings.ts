import "./media/marketplaceSettings.css";
import { addDisposableListener } from "../../../../base/browser/dom.js";
import { DisposableOwner } from "../../../../base/common/lifecycle.js";
import type { IMarketplaceService, MarketplaceInstalledPackage, MarketplacePackageDetails, MarketplacePackageSummary } from "../../../../platform/marketplace/common/marketplaceService.js";

/** Generic package discovery and installation surface backed only by Marketplace business APIs. */
export class MarketplaceSettingsPane extends DisposableOwner {
  readonly element: HTMLElement;
  private readonly status: HTMLParagraphElement;
  private readonly results: HTMLDivElement;
  private query = "";
  private isDisposed = false;

  constructor(private readonly document: Document, private readonly marketplace: IMarketplaceService, private readonly packageType?: string) {
    super();
    this.element = document.createElement("section");
    this.element.className = "zeta-package-marketplace";
    const toolbar = document.createElement("form");
    toolbar.className = "zeta-package-marketplace-toolbar";
    const input = document.createElement("input");
    input.type = "search";
    input.placeholder = packageType === "language" ? "Search language extensions" : "Search packages, skills, MCPs, connectors, languages, and themes";
    input.setAttribute("aria-label", "Search Marketplace");
    const search = document.createElement("button");
    search.type = "submit";
    search.textContent = "Search";
    toolbar.append(input, search);
    this.status = document.createElement("p");
    this.status.className = "zeta-package-marketplace-status";
    this.status.setAttribute("role", "status");
    this.results = document.createElement("div");
    this.results.className = "zeta-package-marketplace-results";
    this.own(addDisposableListener(toolbar, "submit", (event: SubmitEvent) => {
      event.preventDefault();
      this.query = input.value.trim();
      void this.reload();
    }));
    this.element.append(toolbar, this.status, this.results);
    this.defer(() => { this.isDisposed = true; });
    void this.reload();
  }

  private async reload(): Promise<void> {
    this.status.textContent = "Loading Marketplace…";
    this.results.replaceChildren();
    const loaded = await Promise.all([
      this.marketplace.search(this.query, this.packageType, 100),
      this.marketplace.listInstalled(),
    ]).catch((error: unknown) => {
      this.status.textContent = error instanceof Error ? `Marketplace unavailable: ${error.message}` : "Marketplace unavailable.";
      return undefined;
    });
    if (!loaded || this.isDisposed) return;
    const [packages, installed] = loaded;
    this.status.textContent = packages.length === 0 ? "No matching packages." : `${packages.length} package${packages.length === 1 ? "" : "s"}`;
    const details = await Promise.all(packages.map(packageValue => this.marketplace.get(packageValue.id, packageValue.version).catch(() => undefined)));
    if (this.isDisposed) return;
    this.results.replaceChildren(...packages.map((packageValue, index) => this.packageCard(packageValue, details[index], installed)));
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
