import { Emitter, type Event } from "./base/common/event.js";

export type LocalizationParameters = Readonly<Record<string, string | number>>;

/** Stable bundle/key metadata that can be consumed without Workbench services. */
export interface LocalizationKey {
	readonly bundle: string;
	readonly key: string;
}

export type NlsResolver = (
	bundle: string,
	key: string,
	fallback: string,
	parameters?: LocalizationParameters,
) => string;

const changes = new Emitter<void>();
const fallbackResolver: NlsResolver = (_bundle, _key, fallback, parameters) =>
	formatNlsMessage(fallback, parameters);
let resolver: NlsResolver = fallbackResolver;

/** Fires when the active renderer-wide NLS projection changes. */
export const onDidChangeNls: Event<void> = changes.event;

/** Resolves a message through the active renderer-local NLS projection. */
export function localize(
	bundle: string,
	key: string,
	fallback: string,
	parameters?: LocalizationParameters,
): string {
	return resolver(bundle, key, fallback, parameters);
}

/** Installs the resolver for the current renderer realm. */
export function setNlsResolver(next: NlsResolver): void {
	resolver = next;
	changes.fire();
}

/** Restores source-language fallback behavior for isolated tests and hosts. */
export function resetNlsResolver(): void {
	resolver = fallbackResolver;
	changes.fire();
}

export function formatNlsMessage(
	message: string,
	parameters: LocalizationParameters | undefined,
): string {
	if (!parameters) return message;
	return message.replaceAll(/\{([A-Za-z0-9_.-]+)\}/gu, (placeholder, key: string) => {
		const value = parameters[key];
		return value === undefined ? placeholder : String(value);
	});
}
