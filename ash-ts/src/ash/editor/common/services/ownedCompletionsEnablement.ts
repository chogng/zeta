import { isRecord } from '../../../base/common/types.js';

export type CompletionsEnablement = boolean | Readonly<Record<string, boolean>>;

/** Resolves a completion feature's language override before its wildcard value. */
export function isCompletionsEnablementEnabled(enablement: CompletionsEnablement | undefined, languageId: string = '*'): boolean {
	if (typeof enablement === 'boolean') return enablement;
	if (!isRecord(enablement)) return false;
	const languageValue = Object.hasOwn(enablement, languageId) ? enablement[languageId] : undefined;
	if (typeof languageValue === 'boolean') return languageValue;
	return Object.hasOwn(enablement, '*') && typeof enablement['*'] === 'boolean' && enablement['*'];
}

export function isCompletionsEnablement(value: unknown): value is CompletionsEnablement {
	return typeof value === 'boolean' || (isRecord(value) && Object.values(value).every(entry => typeof entry === 'boolean'));
}
