import { Emitter } from "../../../../base/common/event.js";
import { editJsonObjectProperty } from '../../../../base/common/json.js';
import {
	Disposable,
	toDisposable,
} from "../../../../base/common/lifecycle.js";
import { configurationValues, emptyConfigurationDocument, type IConfigurationApi, type IConfigurationDocument, type IConfigurationSnapshot, validateConfigurationDocument, validateConfigurationSnapshot } from "../../../../platform/configuration/common/configurationIpc.js";
import type { IConfigurationChangeEvent, IConfigurationKey, IConfigurationService } from "../../../../platform/configuration/common/configurationService.js";
import { ConfigurationResourceRevisionConflictError, type IConfigurationResourceService, type IConfigurationResourceSnapshot } from "../../../../platform/configuration/common/configurationResourceService.js";
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
	extends Disposable
	implements IConfigurationService, IConfigurationResourceService {
	private readonly api: IConfigurationApi | undefined;
	private readonly registry: ConfigurationRegistry;
	private readonly onError: (error: unknown) => void;
	private readonly _onDidChangeConfiguration =
		this._register(new Emitter<IConfigurationChangeEvent>());
	private readonly resourceChangeEmitter = this._register(new Emitter<IConfigurationResourceSnapshot>());
	private readonly values = new Map<IConfigurationKey<unknown>, unknown>();
	private revision = 0;
	private document = emptyConfigurationDocument();
	private hasAuthoritativeSnapshot: boolean;
	private initialLoad: Promise<void> | undefined;

	readonly onDidChangeConfiguration =
		this._onDidChangeConfiguration.event;
	readonly onDidChangeResource = this.resourceChangeEmitter.event;

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
			this._register(toDisposable(() => subscription.dispose()));
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
		if (serialized === undefined) throw new TypeError(`Configuration key '${key.key}' did not serialize to JSON`);
		if (this.api && !this.hasAuthoritativeSnapshot) {
			await this.reload();
		}
		const document = validateConfigurationDocument({ version: 1, source: editJsonObjectProperty(this.document.source, key.key, serialized) });
		await this.writeDocument(document);
	}

	async resetValue<T>(key: IConfigurationKey<T>): Promise<void> {
		this.assertRegistered(key);
		if (this.api && !this.hasAuthoritativeSnapshot) {
			await this.reload();
		}
		await this.writeDocument(validateConfigurationDocument({ version: 1, source: editJsonObjectProperty(this.document.source, key.key, undefined) }));
	}

	async read(): Promise<IConfigurationResourceSnapshot> {
		if (this.api && !this.hasAuthoritativeSnapshot) await this.reload();
		return this.resourceSnapshot();
	}

	async write(source: string, expectedRevision: number): Promise<IConfigurationResourceSnapshot> {
		if (typeof source !== "string") throw new TypeError("Configuration resource source must be text");
		if (!Number.isSafeInteger(expectedRevision) || expectedRevision < 0) {
			throw new TypeError("Configuration resource revision must be a non-negative safe integer");
		}
		if (this.api && !this.hasAuthoritativeSnapshot) await this.reload();
		if (expectedRevision !== this.revision) {
			throw new ConfigurationResourceRevisionConflictError(expectedRevision, this.revision);
		}
		const document = this.parseResourceSource(source);
		try {
			await this.writeDocument(document, expectedRevision);
		} catch (error) {
			if (isRevisionConflict(error)) throw new ConfigurationResourceRevisionConflictError(expectedRevision, undefined);
			throw error;
		}
		return this.resourceSnapshot();
	}

	private async writeDocument(
		document: IConfigurationDocument,
		expectedRevision = this.revision,
	): Promise<void> {
		if (!this.api) {
			if (expectedRevision !== this.revision) {
				throw new ConfigurationResourceRevisionConflictError(expectedRevision, this.revision);
			}
			this.acceptSnapshot({
				revision: this.revision + 1,
				document,
			});
			return;
		}

		const result = await this.api.update({
			expectedRevision,
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
		this.resourceChangeEmitter.fire(this.resourceSnapshot());
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
		const configuredValues = configurationValues(this.document);
		for (const key of this.registry.getConfigurations()) {
			const candidate = configuredValues[key.key];
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

	private parseResourceSource(source: string): IConfigurationDocument {
		let document: IConfigurationDocument;
		try {
			document = validateConfigurationDocument({ version: 1, source });
		} catch (error) {
			throw new TypeError(`Settings JSONC is invalid: ${error instanceof Error ? error.message : String(error)}`);
		}
		const values = configurationValues(document);
		let normalizedSource = source;
		for (const [key, value] of Object.entries(values)) {
			const configuration = this.registry.getConfiguration(key);
			if (!configuration) continue;
			try {
				const normalized = configuration.key.serialize(configuration.key.parse(value));
				if (normalized === undefined) throw new TypeError('serialized value is not JSON');
				if (JSON.stringify(normalized) !== JSON.stringify(value)) normalizedSource = editJsonObjectProperty(normalizedSource, key, normalized);
			} catch (error) {
				throw new TypeError(`Invalid configuration value for '${key}': ${error instanceof Error ? error.message : String(error)}`);
			}
		}
		return validateConfigurationDocument({ version: 1, source: normalizedSource });
	}

	private resourceSnapshot(): IConfigurationResourceSnapshot {
		return Object.freeze({
			source: this.document.source,
			revision: this.revision,
		});
	}
}

function isRevisionConflict(error: unknown): boolean {
	return error instanceof Error && /revision conflict/i.test(error.message);
}

function changedConfigurationKeys(
	previous: IConfigurationDocument,
	next: IConfigurationDocument,
): ReadonlySet<string> {
	const previousValues = configurationValues(previous);
	const nextValues = configurationValues(next);
	const keys = new Set([
		...Object.keys(previousValues),
		...Object.keys(nextValues),
	]);
	const changed = new Set<string>();
	for (const key of keys) {
		if (
				JSON.stringify(previousValues[key]) !==
				JSON.stringify(nextValues[key])
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
