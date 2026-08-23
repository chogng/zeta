import { Emitter } from "../../../../base/common/event.js";
import {
	DisposableOwner,
	toDisposable,
} from "../../../../base/common/lifecycle.js";
import { emptyConfigurationDocument, type IConfigurationApi, type IConfigurationDocument, type IConfigurationSnapshot, validateConfigurationDocument, validateConfigurationSnapshot } from "../../../../platform/configuration/common/configurationIpc.js";
import type { IConfigurationChangeEvent, IConfigurationKey, IConfigurationService } from "../../../../platform/configuration/common/configurationService.js";
import {
	type ConfigurationRegistry,
	ConfigurationsRegistry,
} from "../../../../platform/configuration/common/configurationRegistry.js";

export interface WorkbenchConfigurationServiceOptions {
	readonly api?: IConfigurationApi;
	readonly registry?: ConfigurationRegistry;
	readonly onError?: (error: unknown) => void;
}

/**
 * Window-scoped projection of the host-authoritative configuration.
 *
 * Persisted values are validated through registered typed keys. Invalid
 * values fall back atomically to their defaults without mutating the source.
 */
export class WorkbenchConfigurationService
	extends DisposableOwner
	implements IConfigurationService {
	private readonly api: IConfigurationApi | undefined;
	private readonly registry: ConfigurationRegistry;
	private readonly onError: (error: unknown) => void;
	private readonly _onDidChangeConfiguration =
		this.own(new Emitter<IConfigurationChangeEvent>());
	private readonly values = new Map<IConfigurationKey<unknown>, unknown>();
	private revision = 0;
	private document = emptyConfigurationDocument();
	private hasAuthoritativeSnapshot: boolean;
	private initialLoad: Promise<void> | undefined;

	readonly onDidChangeConfiguration =
		this._onDidChangeConfiguration.event;

	constructor(options: WorkbenchConfigurationServiceOptions = {}) {
		super();
		this.api = options.api;
		this.registry = options.registry ?? ConfigurationsRegistry;
		this.onError = options.onError ??
			((error) => console.error("Failed to apply configuration", error));
		this.hasAuthoritativeSnapshot = this.api === undefined;
		this.rebuildValues();

		if (this.api) {
			const subscription = this.api.onDidChange((candidate) => {
				try {
					this.acceptSnapshot(
						validateConfigurationSnapshot(candidate),
					);
				} catch (error) {
					this.onError(error);
				}
			});
			this.own(toDisposable(() => subscription.dispose()));
		}
	}

	getValue<T>(key: IConfigurationKey<T>): T {
		this.assertRegistered(key);
		return this.values.get(
			key as IConfigurationKey<unknown>,
		) as T;
	}

	async updateValue<T>(
		key: IConfigurationKey<T>,
		value: T,
	): Promise<void> {
		this.assertRegistered(key);
		const serialized = key.serialize(value);
		key.parse(serialized);
		if (this.api && !this.hasAuthoritativeSnapshot) {
			await this.reload();
		}
		const document = validateConfigurationDocument({
			version: 1,
			values: {
				...this.document.values,
				[key.key]: serialized,
			},
		});
		await this.writeDocument(document);
	}

	async resetValue<T>(key: IConfigurationKey<T>): Promise<void> {
		this.assertRegistered(key);
		if (this.api && !this.hasAuthoritativeSnapshot) {
			await this.reload();
		}
		const values: Record<string, unknown> = {
			...this.document.values,
		};
		delete values[key.key];
		await this.writeDocument(validateConfigurationDocument({
			version: 1,
			values,
		}));
	}

	private async writeDocument(
		document: IConfigurationDocument,
	): Promise<void> {
		if (!this.api) {
			this.acceptSnapshot({
				revision: this.revision + 1,
				document,
			});
			return;
		}

		const result = await this.api.update({
			expectedRevision: this.revision,
			document,
		});
		this.acceptSnapshot(validateConfigurationSnapshot(result));
	}

	async reload(): Promise<void> {
		if (!this.api) return;
		if (!this.initialLoad) {
			this.initialLoad = this.api.read()
				.then((candidate) => {
					this.acceptSnapshot(
						validateConfigurationSnapshot(candidate),
					);
				})
				.finally(() => {
					this.initialLoad = undefined;
				});
		}
		await this.initialLoad;
	}

	private acceptSnapshot(snapshot: IConfigurationSnapshot): void {
		if (!this.hasAuthoritativeSnapshot) {
			this.hasAuthoritativeSnapshot = true;
			this.applySnapshot(snapshot);
			return;
		}
		if (snapshot.revision < this.revision) return;
		if (
			snapshot.revision === this.revision &&
			serializeDocument(snapshot.document) ===
				serializeDocument(this.document)
		) {
			return;
		}
		if (snapshot.revision === this.revision) {
			throw new Error(
				"Configuration changed without advancing its revision",
			);
		}
		this.applySnapshot(snapshot);
	}

	private applySnapshot(snapshot: IConfigurationSnapshot): void {
		const changedKeys = changedConfigurationKeys(
			this.document,
			snapshot.document,
		);
		this.revision = snapshot.revision;
		this.document = snapshot.document;
		this.rebuildValues();
		if (changedKeys.size === 0) return;
		this._onDidChangeConfiguration.fire({
			keys: changedKeys,
			affectsConfiguration<T>(key: IConfigurationKey<T>): boolean {
				return changedKeys.has(key.key);
			},
		});
	}

	private rebuildValues(): void {
		this.values.clear();
		for (const key of this.registry.getConfigurations()) {
			const candidate = this.document.values[key.key];
			if (candidate === undefined) {
				this.values.set(key, key.defaultValue);
				continue;
			}
			try {
				this.values.set(key, key.parse(candidate));
			} catch (error) {
				this.values.set(key, key.defaultValue);
				this.onError(
					new Error(`Invalid configuration value for '${key.key}'`, {
						cause: error,
					}),
				);
			}
		}
	}

	private assertRegistered<T>(key: IConfigurationKey<T>): void {
		if (!this.registry.owns(key)) {
			throw new Error(`Unknown configuration key: ${key.key}`);
		}
	}
}

function changedConfigurationKeys(
	previous: IConfigurationDocument,
	next: IConfigurationDocument,
): ReadonlySet<string> {
	const keys = new Set([
		...Object.keys(previous.values),
		...Object.keys(next.values),
	]);
	const changed = new Set<string>();
	for (const key of keys) {
		if (
			JSON.stringify(previous.values[key]) !==
				JSON.stringify(next.values[key])
		) {
			changed.add(key);
		}
	}
	return changed;
}

function serializeDocument(
	document: IConfigurationDocument,
): string {
	return JSON.stringify(document);
}
