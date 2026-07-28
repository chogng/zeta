/**
 * Registry of statically declared Desktop configuration keys.
 *
 * Keys are registered once for the current JavaScript realm. Configuration
 * services use the registry to validate complete persisted snapshots.
 */
export class ConfigurationRegistry {
    #keys = new Map();
    registerConfiguration(definition) {
        if (!isConfigurationKey(definition.key)) {
            throw new TypeError(`Invalid configuration key: ${definition.key}`);
        }
        if (this.#keys.has(definition.key)) {
            throw new Error(`Configuration key is already registered: ${definition.key}`);
        }
        const key = Object.freeze({
            key: definition.key,
            defaultValue: definition.defaultValue,
            parse: definition.parse,
            serialize: definition.serialize ?? ((value) => value),
        });
        this.#keys.set(definition.key, key);
        return key;
    }
    getConfigurations() {
        return [...this.#keys.values()];
    }
    owns(key) {
        return this.#keys.get(key.key) === key;
    }
}
export const ConfigurationsRegistry = new ConfigurationRegistry();
function isConfigurationKey(value) {
    return /^[A-Za-z][A-Za-z0-9.-]{0,127}$/.test(value);
}
