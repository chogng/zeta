import { Emitter } from '../../../base/common/event.js';
import { Disposable } from '../../../base/common/lifecycle.js';
import type { IConfigurationChangeEvent, IConfigurationKey, IConfigurationOverrides, IConfigurationService } from './configurationService.js';
import { ConfigurationsRegistry, type ConfigurationRegistry } from './configurationRegistry.js';

/** Mutable, process-local configuration used by standalone compositions without persisted settings. */
export class InMemoryConfigurationService extends Disposable implements IConfigurationService {
	private readonly values = new Map<IConfigurationKey<unknown>, unknown>();
	private readonly _onDidChangeConfiguration = this._register(new Emitter<IConfigurationChangeEvent>());
	readonly onDidChangeConfiguration = this._onDidChangeConfiguration.event;

	constructor(private readonly registry: ConfigurationRegistry = ConfigurationsRegistry) {
		super();
	}

	getValue<T>(key: IConfigurationKey<T>, _overrides?: IConfigurationOverrides): T {
		this.assertRegistered(key);
		return (this.values.has(key as IConfigurationKey<unknown>)
			? this.values.get(key as IConfigurationKey<unknown>)
			: key.defaultValue) as T;
	}

	async updateValue<T>(key: IConfigurationKey<T>, value: T): Promise<void> {
		this.assertRegistered(key);
		const parsed = key.parse(key.serialize(value));
		if (Object.is(this.getValue(key), parsed)) return;
		this.values.set(key as IConfigurationKey<unknown>, parsed);
		this.fireChange(key);
	}

	async resetValue<T>(key: IConfigurationKey<T>): Promise<void> {
		this.assertRegistered(key);
		if (!this.values.delete(key as IConfigurationKey<unknown>)) return;
		this.fireChange(key);
	}

	async reload(): Promise<void> {}

	private fireChange<T>(key: IConfigurationKey<T>): void {
		const keys = new Set([key.key]);
		this._onDidChangeConfiguration.fire(Object.freeze({
			keys,
			affectsConfiguration: <TValue>(candidate: IConfigurationKey<TValue>) => candidate.key === key.key,
		}));
	}

	private assertRegistered<T>(key: IConfigurationKey<T>): void {
		if (!this.registry.owns(key)) throw new ReferenceError(`Configuration key is not registered: ${key.key}`);
	}
}
