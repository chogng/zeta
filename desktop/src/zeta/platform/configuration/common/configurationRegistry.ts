import type {
  IConfigurationKey,
} from "./configuration.js";

export interface IConfigurationKeyDefinition<T> {
  readonly key: string;
  readonly defaultValue: T;
  readonly parse: (value: unknown) => T;
  readonly serialize?: (value: T) => unknown;
}

/**
 * Registry of statically declared Desktop configuration keys.
 *
 * Keys are registered once for the current JavaScript realm. Configuration
 * services use the registry to validate complete persisted snapshots.
 */
export class ConfigurationRegistry {
  readonly #keys = new Map<string, IConfigurationKey<unknown>>();

  registerConfiguration<T>(
    definition: IConfigurationKeyDefinition<T>,
  ): IConfigurationKey<T> {
    if (!isConfigurationKey(definition.key)) {
      throw new TypeError(`Invalid configuration key: ${definition.key}`);
    }
    if (this.#keys.has(definition.key)) {
      throw new Error(
        `Configuration key is already registered: ${definition.key}`,
      );
    }
    const key: IConfigurationKey<T> = Object.freeze({
      key: definition.key,
      defaultValue: definition.defaultValue,
      parse: definition.parse,
      serialize: definition.serialize ?? ((value: T) => value),
    });
    this.#keys.set(
      definition.key,
      key as IConfigurationKey<unknown>,
    );
    return key;
  }

  getConfigurations(): readonly IConfigurationKey<unknown>[] {
    return [...this.#keys.values()];
  }

  owns<T>(key: IConfigurationKey<T>): boolean {
    return this.#keys.get(key.key) === key;
  }
}

export const ConfigurationsRegistry = new ConfigurationRegistry();

function isConfigurationKey(value: string): boolean {
  return /^[A-Za-z][A-Za-z0-9.-]{0,127}$/.test(value);
}
