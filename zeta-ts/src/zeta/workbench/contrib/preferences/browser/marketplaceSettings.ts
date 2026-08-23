import "./media/marketplaceSettings.css";
import { addDisposableListener, h } from "../../../../base/browser/dom.js";
import { appendIcon } from "../../../../base/browser/ui/icon/icon.js";
import { TabList } from "../../../../base/browser/ui/tablist/tabList.js";
import { lxiconsLibrary } from "../../../../base/common/lxiconsLibrary.js";
import { DisposableOwner } from "../../../../base/common/lifecycle.js";
import type { ILocalizationService } from "../../../services/localization/common/localizationService.js";
import type { IMarketplaceService, MarketplaceBrowseSnapshot, MarketplaceInstalledPackage, MarketplacePackageDetails, MarketplacePackageSummary } from "../../../../platform/marketplace/common/marketplaceService.js";

const packageTypeFilters = [
	{ label: "All", packageType: undefined },
	{ label: "Plugins", packageType: "plugin" },
	{ label: "MCPs", packageType: "mcp" },
	{ label: "Skills", packageType: "skill" },
	{ label: "Languages", packageType: "language" },
	{ label: "Localization", packageType: "localization" },
	{ label: "Themes", packageType: "theme" },
] as const;

/** Generic package discovery and installation surface backed only by Marketplace business APIs. */
export class MarketplaceSettingsPane extends DisposableOwner {
	readonly element: HTMLElement;
	private readonly document: Document;
	private readonly input: HTMLInputElement;
	private readonly status: HTMLParagraphElement;
	private readonly results: HTMLDivElement;
	private readonly filterTabs: TabList<string> | undefined;
	private query = "";
	private selectedPackageType: string | undefined;
	private reloadGeneration = 0;
	private isDisposed = false;
	private readonly localization: ILocalizationService | undefined;

	constructor(container: HTMLElement, private readonly marketplace: IMarketplaceService, private readonly fixedPackageType?: string, localization?: ILocalizationService) {
		super();
		this.localization = localization;
		this.document = container.ownerDocument;
		this.selectedPackageType = fixedPackageType;
		this.element = h(this.document, "section");
		this.element.className = "zeta-package-marketplace";
		container.append(this.element);
		const toolbar = h(this.document, "form");
		toolbar.className = "zeta-package-marketplace-toolbar";
		const searchControl = h(this.document, "div");
		searchControl.className = "zeta-package-marketplace-search-control";
		appendIcon(lxiconsLibrary.search, searchControl);
		this.input = h(this.document, "input");
		this.input.type = "search";
		this.input.placeholder = fixedPackageType === "language" ? this.t("searchLanguageExtensions", "Search language extensions") : this.t("searchPackages", "Search Plugins, Skills, MCPs…");
		this.input.setAttribute("aria-label", this.t("searchMarketplace", "Search Marketplace"));
		searchControl.append(this.input);
		const search = h(this.document, "button");
		search.type = "submit";
		search.textContent = "Browse Marketplace";
		toolbar.append(searchControl, search);
		const filters = h(this.document, "div");
		filters.className = "zeta-package-marketplace-filters";
		this.filterTabs = fixedPackageType ? undefined : this.own(new TabList<string>(filters, {
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
		this.status = h(this.document, "p");
		this.status.className = "zeta-package-marketplace-status";
		this.status.setAttribute("role", "status");
		this.results = h(this.document, "div");
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
		this.status.textContent = this.t("loading", "Loading Marketplace…");
		this.results.classList.remove("empty");
		this.results.replaceChildren();
		const loaded = await (forceRefresh
			? this.marketplace.refreshBrowse(this.query, this.selectedPackageType, 100)
			: this.marketplace.browse(this.query, this.selectedPackageType, 100)).catch((error: unknown) => {
			if (generation !== this.reloadGeneration || this.isDisposed) return undefined;
			this.status.textContent = error instanceof Error ? `${this.t("unavailable", "Marketplace unavailable.")} ${error.message}` : this.t("unavailable", "Marketplace unavailable.");
			return undefined;
		});
		if (!loaded || this.isDisposed || generation !== this.reloadGeneration) return;
		this.render(loaded);
	}

	private render(snapshot: MarketplaceBrowseSnapshot): void {
		const packages = snapshot.packages;
		this.status.textContent = packages.length === 0 ? this.t("noMatching", "No matching packages.") : this.t(packages.length === 1 ? "packageCount" : "packageCountPlural", `${packages.length} package${packages.length === 1 ? "" : "s"}`, { count: packages.length });
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
					label: this.filterLabel(filter.label),
					tabId: `${id}-tab`,
				};
			}),
			marketplaceFilterId(this.selectedPackageType),
		);
	}

	private emptyState(): HTMLElement {
		const empty = h(this.document, "div");
		empty.className = "zeta-package-marketplace-empty-state";
		const heading = h(this.document, "h4");
		heading.textContent = this.selectedPackageType ? this.t("noTypeFound", `No ${this.selectedPackageType} packages found`, { type: this.selectedPackageType }) : this.t("explore", "Explore Marketplace packages");
		const description = h(this.document, "p");
		description.textContent = this.query
			? this.t("tryDifferent", "Try a different search or clear the current filters.")
			: this.t("installDescription", "Install Plugins, Skills, MCP servers, language support, and themes from the signed catalog.");
		const reset = h(this.document, "button");
		reset.type = "button";
		reset.textContent = this.t("clearFilters", "Clear filters");
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
		const card = h(this.document, "article");
		card.className = "zeta-package-marketplace-card";
		const heading = h(this.document, "h4");
		heading.textContent = summary.displayName;
		const description = h(this.document, "p");
		description.className = "zeta-package-marketplace-description";
		description.textContent = summary.description;
		const metadata = h(this.document, "p");
		metadata.className = "zeta-package-marketplace-metadata";
		metadata.textContent = `${summary.id} · ${summary.version} · ${summary.packageType}${details ? ` · ${details.license}` : ""}`;
		const provenance = h(this.document, "p");
		provenance.className = "zeta-package-marketplace-metadata";
		provenance.textContent = details?.upstream?.registry === "officialMcp"
			? this.t("officialMcp", `Listed in the official MCP Registry · ${details.upstream.name}@${details.upstream.version}`, { name: details.upstream.name, version: details.upstream.version })
			: details?.source === "official"
				? this.t("official", "Marketplace official package")
				: this.t("thirdParty", "Third-party package");
		const capabilities = h(this.document, "ul");
		capabilities.className = "zeta-package-marketplace-capabilities";
		for (const capability of details?.capabilities ?? []) {
			const item = h(this.document, "li");
			item.textContent = `${capability.kind}: ${capability.id}`;
			capabilities.append(item);
		}
		const active = installed.find(candidate => candidate.package.id === summary.id && candidate.package.version === summary.version);
		const actions = h(this.document, "div");
		actions.className = "zeta-package-marketplace-actions";
		const lifecycle = h(this.document, "span");
		lifecycle.className = "zeta-package-marketplace-lifecycle";
		lifecycle.textContent = active ? (active.state === "installed" ? this.t("installed", "Installed · activation is separate") : this.t("removalPending", "Removal pending")) : this.t("available", "Available");
		const action = h(this.document, "button");
		action.type = "button";
		action.textContent = active ? this.t("uninstall", "Uninstall") : this.t("install", "Install");
		action.disabled = active?.state === "pendingRemoval";
		this.own(addDisposableListener(action, "click", () => {
			action.disabled = true;
			const operation = active
				? this.marketplace.uninstall(active.installationId)
				: this.marketplace.install(summary.id, summary.version).then(() => undefined);
			void operation.then(() => this.reload()).catch((error: unknown) => {
				this.status.textContent = error instanceof Error ? `${this.t("operationFailed", "Marketplace operation failed.")} ${error.message}` : this.t("operationFailed", "Marketplace operation failed.");
				action.disabled = false;
			});
		}));
		actions.append(lifecycle, action);
		card.append(heading, description, metadata, provenance);
		if (capabilities.childElementCount > 0) card.append(capabilities);
		card.append(actions);
		return card;
	}

	private t(key: string, fallback: string, parameters?: Readonly<Record<string, string | number>>): string {
		return this.localization?.translate("zeta.marketplace", key, fallback, parameters) ?? fallback;
	}

	private filterLabel(label: string): string {
		const key = label === "All" ? "filterAll" : label === "Plugins" ? "filterPlugins" : label === "MCPs" ? "filterMcps" : label === "Skills" ? "filterSkills" : label === "Languages" ? "filterLanguages" : label === "Localization" ? "filterLocalization" : "filterThemes";
		return this.t(key, label);
	}
}

function marketplaceFilterId(packageType: string | undefined): string {
	return `marketplace-filter-${packageType ?? "all"}`;
}
