import { Emitter } from "../../../../base/common/event.js";
import { Disposable, toDisposable } from "../../../../base/common/lifecycle.js";
import type { IServerEventApi } from "../../../../platform/app-server/common/appServerApi.js";
import type { IMarketplaceApi } from "../../../../platform/marketplace/common/marketplaceApi.js";
import type { IMarketplaceService, MarketplaceAcquiredCapability, MarketplaceBrowseSnapshot, MarketplaceInstalledPackage, MarketplacePackageDetails, MarketplacePackageSummary } from "../../../../platform/marketplace/common/marketplaceService.js";

/** App Server adapter and owner of path-free Renderer browse snapshots. */
export class AppServerMarketplaceService extends Disposable implements IMarketplaceService {
	private readonly browseSnapshots = new Map<string, MarketplaceBrowseSnapshot>();
	private readonly browseRequests = new Map<string, Promise<MarketplaceBrowseSnapshot>>();
	private readonly details = new Map<string, Promise<MarketplacePackageDetails>>();
	private browseGeneration = 0;
	private installedInstanceId: string | undefined;
	private installedGeneration = 0;
	private readonly _onDidChangeInstalled = this._register(new Emitter<void>());

	readonly onDidChangeInstalled = this._onDidChangeInstalled.event;

	constructor(private readonly api: IMarketplaceApi, events: IServerEventApi) {
		super();
		const subscription = events.subscribe(event => {
			if (event.method !== "marketplace/changed") return;
			if (event.params.instanceId === this.installedInstanceId && event.params.generation <= this.installedGeneration) return;
			this.installedInstanceId = event.params.instanceId;
			this.installedGeneration = event.params.generation;
			this.invalidateBrowse();
			this._onDidChangeInstalled.fire();
		});
		this._register(toDisposable(() => subscription.dispose()));
	}

	cachedBrowse(query: string, packageType?: string, limit?: number): MarketplaceBrowseSnapshot | undefined {
		return this.browseSnapshots.get(browseKey(query, packageType, limit));
	}

	browse(query: string, packageType?: string, limit?: number): Promise<MarketplaceBrowseSnapshot> {
		return Promise.resolve(this.cachedBrowse(query, packageType, limit) ?? this.loadBrowse(query, packageType, limit));
	}

	refreshBrowse(query: string, packageType?: string, limit?: number): Promise<MarketplaceBrowseSnapshot> {
		const key = browseKey(query, packageType, limit);
		this.browseSnapshots.delete(key);
		return this.loadBrowse(query, packageType, limit);
	}

	async search(query: string, packageType?: string, limit?: number): Promise<readonly MarketplacePackageSummary[]> {
		return (await this.api.search({ query, packageType: packageType ?? null, limit: limit ?? null })).packages;
	}

	get(packageId: string, version?: string): Promise<MarketplacePackageDetails> {
		return this.api.get({ packageId, version: version ?? null });
	}

	download(packageId: string, version?: string) {
		return this.api.download({ packageId, version: version ?? null });
	}

	async install(packageId: string, version?: string): Promise<MarketplaceInstalledPackage> {
		return this.api.install({ packageId, version: version ?? null });
	}

	async update(installationId: string, version?: string): Promise<MarketplaceInstalledPackage> {
		return this.api.update({ installationId, version: version ?? null });
	}

	async uninstall(installationId: string, mode: "ifUnused" | "whenUnused" = "whenUnused"): Promise<void> {
		await this.api.uninstall({ installationId, mode });
	}

	async listInstalled(): Promise<readonly MarketplaceInstalledPackage[]> {
		const result = await this.api.listInstalled();
		if (result.instanceId !== this.installedInstanceId) {
			this.installedInstanceId = result.instanceId;
			this.installedGeneration = result.generation;
		} else {
			this.installedGeneration = Math.max(this.installedGeneration, result.generation);
		}
		return result.packages;
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

	private loadBrowse(query: string, packageType?: string, limit?: number): Promise<MarketplaceBrowseSnapshot> {
		const key = browseKey(query, packageType, limit);
		const existing = this.browseRequests.get(key);
		if (existing) return existing;
		const generation = this.browseGeneration;
		let request!: Promise<MarketplaceBrowseSnapshot>;
		request = Promise.all([
			this.search(query, packageType, limit),
			this.listInstalled(),
		]).then(async ([packages, installed]) => {
			const browsePackages = await Promise.all(packages.map(async summary => ({
				summary,
				details: await this.packageDetails(summary.id, summary.version).catch(() => undefined),
			})));
			const snapshot = Object.freeze({ query, packageType, limit, packages: Object.freeze(browsePackages), installed: Object.freeze([...installed]) });
			if (generation === this.browseGeneration) this.browseSnapshots.set(key, snapshot);
			return snapshot;
		}).finally(() => {
			if (this.browseRequests.get(key) === request) this.browseRequests.delete(key);
		});
		this.browseRequests.set(key, request);
		return request;
	}

	private packageDetails(packageId: string, version: string): Promise<MarketplacePackageDetails> {
		const key = `${packageId}\0${version}`;
		const existing = this.details.get(key);
		if (existing) return existing;
		const request = this.get(packageId, version).catch((error: unknown) => {
			this.details.delete(key);
			throw error;
		});
		this.details.set(key, request);
		return request;
	}

	private invalidateBrowse(): void {
		this.browseGeneration += 1;
		this.browseSnapshots.clear();
		this.browseRequests.clear();
	}
}

function browseKey(query: string, packageType?: string, limit?: number): string {
	return JSON.stringify([query, packageType ?? null, limit ?? null]);
}
