import type { Event } from "../../../base/common/event.js";
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

  affectsConfiguration<T>(key: IConfigurationKey<T>): boolean;
}

/** Resolves typed Desktop configuration and publishes atomic snapshot changes. */
export interface IConfigurationService {
  readonly onDidChangeConfiguration: Event<IConfigurationChangeEvent>;

  getValue<T>(key: IConfigurationKey<T>): T;
  updateValue<T>(key: IConfigurationKey<T>, value: T): Promise<void>;
  resetValue<T>(key: IConfigurationKey<T>): Promise<void>;
  reload(): Promise<void>;
}

export const IConfigurationService = createServiceIdentifier<IConfigurationService>("configurationService");
