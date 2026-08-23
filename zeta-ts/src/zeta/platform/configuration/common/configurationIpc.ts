import { type JsonValue, validateJsonValue } from "../../../base/common/jsonValue.js";

export const CONFIGURATION_READ_CHANNEL = "zeta:configuration:read";
export const CONFIGURATION_UPDATE_CHANNEL = "zeta:configuration:update";
export const CONFIGURATION_CHANGED_CHANNEL = "zeta:configuration:changed";

export type ConfigurationValue = JsonValue;

/** Versioned Desktop configuration persisted by the host. */
export interface IConfigurationDocument {
	readonly version: 1;
	readonly values: Readonly<Record<string, ConfigurationValue>>;
}

/** One host-authoritative configuration snapshot. */
export interface IConfigurationSnapshot {
	readonly revision: number;
	readonly document: IConfigurationDocument;
}

/** Compare-and-swap update used to avoid overwriting a newer snapshot. */
export interface IConfigurationUpdateRequest {
	readonly expectedRevision: number;
	readonly document: IConfigurationDocument;
}

export interface IConfigurationSubscription {
	dispose(): void;
}

/** Narrow context-bridge capability for Desktop configuration transport. */
export interface IConfigurationApi {
	read(): Promise<unknown>;
	update(request: IConfigurationUpdateRequest): Promise<unknown>;
	onDidChange(listener: (snapshot: unknown) => void): IConfigurationSubscription;
}

export function emptyConfigurationDocument(): IConfigurationDocument {
	return { version: 1, values: {} };
}

/** Validates an untrusted persisted configuration document. */
export function validateConfigurationDocument(value: unknown): IConfigurationDocument {
	const document = exactRecord(value, ["values", "version"]);
	if (document.version !== 1) throw new Error("configuration version must be 1");
	const values = record(document.values, "values");
	const validated: Record<string, ConfigurationValue> = {};
	for (const [key, candidate] of Object.entries(values)) {
		if (!/^[A-Za-z][A-Za-z0-9.-]{0,127}$/.test(key)) throw new Error(`invalid configuration key: ${key}`);
		validated[key] = validateJsonValue(candidate, { path: `values.${key}` });
	}
	return { version: 1, values: validated };
}

export function validateConfigurationSnapshot(value: unknown): IConfigurationSnapshot {
	const snapshot = exactRecord(value, ["document", "revision"]);
	return { revision: nonNegativeSafeInteger(snapshot.revision, "revision"), document: validateConfigurationDocument(snapshot.document) };
}

export function validateConfigurationUpdateRequest(value: unknown): IConfigurationUpdateRequest {
	const request = exactRecord(value, ["document", "expectedRevision"]);
	return { expectedRevision: nonNegativeSafeInteger(request.expectedRevision, "expectedRevision"), document: validateConfigurationDocument(request.document) };
}

export function validateConfigurationRead(value: unknown): undefined {
	if (value !== undefined) throw new Error("configuration read does not accept parameters");
	return undefined;
}

function exactRecord(value: unknown, keys: readonly string[]): Record<string, unknown> {
	const result = record(value, "configuration");
	const actual = Object.keys(result).sort();
	const expected = [...keys].sort();
	if (actual.length !== expected.length || actual.some((key, index) => key !== expected[index])) throw new Error(`configuration object must contain exactly: ${expected.join(", ")}`);
	return result;
}

function record(value: unknown, path: string): Record<string, unknown> {
	if (typeof value !== "object" || value === null || Array.isArray(value)) throw new Error(`${path} must be an object`);
	return value as Record<string, unknown>;
}

function nonNegativeSafeInteger(value: unknown, field: string): number {
	if (!Number.isSafeInteger(value) || (value as number) < 0) throw new Error(`${field} must be a non-negative safe integer`);
	return value as number;
}
