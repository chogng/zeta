import type { Event } from "../../../base/common/event.js";
import { URI } from "../../../base/common/uri.js";
import { createServiceIdentifier } from "../../instantiation/common/instantiation.js";

/**
 * A typed configuration address with validation and persistence semantics.
 *
 * Definitions parse untrusted persisted values and serialize validated values
 * without exposing storage details to consumers.
 */
export interface IConfigurationKey<T> {
	readonly key: string;
	readonly defaultValue: T;

	parse(value: unknown): T;
	serialize(value: T): unknown;
}

export interface IConfigurationChangeEvent {
	readonly keys: ReadonlySet<string>;

	affectsConfiguration<T>(key: IConfigurationKey<T>, overrides?: IConfigurationOverrides): boolean;
}

export interface IConfigurationOverrides {
	readonly overrideIdentifier?: string | null;
	readonly resource?: URI | null;
}

export const enum ConfigurationTarget {
	APPLICATION = 1,
	USER,
	USER_LOCAL,
	USER_REMOTE,
	WORKSPACE,
	WORKSPACE_FOLDER,
	DEFAULT,
	MEMORY,
}

export interface IConfigurationValue<T> {
	readonly defaultValue?: T;
	readonly applicationValue?: T;
	readonly userValue?: T;
	readonly userLocalValue?: T;
	readonly userRemoteValue?: T;
	readonly workspaceValue?: T;
	readonly workspaceFolderValue?: T;
	readonly memoryValue?: T;
	readonly policyValue?: T;
	readonly value?: T;
}

export interface IConfigurationModel {
	readonly contents: Readonly<Record<string, unknown>>;
	readonly keys: readonly string[];
	readonly overrides: readonly { readonly keys: readonly string[]; readonly contents: Readonly<Record<string, unknown>>; readonly identifiers: readonly string[] }[];
}

export interface IConfigurationData {
	readonly defaults: IConfigurationModel;
	readonly policy: IConfigurationModel;
	readonly application: IConfigurationModel;
	readonly userLocal: IConfigurationModel;
	readonly userRemote: IConfigurationModel;
	readonly workspace: IConfigurationModel;
	readonly folders: readonly (readonly [URI, IConfigurationModel])[];
}

export function getConfigValueInTarget<T>(value: IConfigurationValue<T>, target: ConfigurationTarget): T | undefined {
	switch (target) {
		case ConfigurationTarget.APPLICATION: return value.applicationValue;
		case ConfigurationTarget.USER: return value.userValue;
		case ConfigurationTarget.USER_LOCAL: return value.userLocalValue;
		case ConfigurationTarget.USER_REMOTE: return value.userRemoteValue;
		case ConfigurationTarget.WORKSPACE: return value.workspaceValue;
		case ConfigurationTarget.WORKSPACE_FOLDER: return value.workspaceFolderValue;
		case ConfigurationTarget.DEFAULT: return value.defaultValue;
		case ConfigurationTarget.MEMORY: return value.memoryValue;
	}
}

export function isConfigurationOverrides(value: unknown): value is IConfigurationOverrides {
	if (!value || typeof value !== "object") return false;
	const candidate = value as IConfigurationOverrides;
	return (candidate.overrideIdentifier == null || typeof candidate.overrideIdentifier === "string")
		&& (candidate.resource == null || candidate.resource instanceof URI);
}

/** Resolves typed Desktop configuration and publishes atomic snapshot changes. */
export interface IConfigurationService {
	readonly onDidChangeConfiguration: Event<IConfigurationChangeEvent>;

	getValue<T>(key: IConfigurationKey<T>, overrides?: IConfigurationOverrides): T;
	updateValue<T>(key: IConfigurationKey<T>, value: T): Promise<void>;
	resetValue<T>(key: IConfigurationKey<T>): Promise<void>;
	reload(): Promise<void>;
}

export const IConfigurationService = createServiceIdentifier<IConfigurationService>("configurationService");
