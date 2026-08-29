/**
 * Narrows code after a caller-defined condition or fails with its context.
 */
export function assert(
	condition: unknown,
	messageOrError: string | Error,
): asserts condition {
	if (!condition) {
		throw typeof messageOrError === "string"
			? new Error(messageOrError)
			: messageOrError;
	}
}

/**
 * Narrows a value to its non-nullable type or fails with caller-owned context.
 */
export function assertDefined<T>(
	value: T,
	messageOrError: string | Error,
): asserts value is NonNullable<T> {
	assert(value !== undefined && value !== null, messageOrError);
}

export function isDefined<T>(value: T | null | undefined): value is T {
	return value !== undefined && value !== null;
}

export function assertType(condition: unknown, type?: string): asserts condition {
	if (!condition) throw new TypeError(type ? `Unexpected type, expected '${type}'` : 'Unexpected type');
}

/** Narrows an unknown value to a non-array object with string keys. */
export function isRecord(value: unknown): value is Record<string, unknown> {
	return typeof value === "object" && value !== null && !Array.isArray(value);
}

/** Narrows an unknown value to a string containing non-whitespace text. */
export function isNonEmptyString(value: unknown): value is string {
	return typeof value === "string" && value.trim().length > 0;
}

export type Mutable<T> = {
	-readonly [K in keyof T]: T[K];
};

export function isFunction(value: unknown): value is (...args: never[]) => unknown {
	return typeof value === 'function';
}

export function isObject(value: unknown): value is object {
	return typeof value === 'object' && value !== null && !Array.isArray(value) && !(value instanceof RegExp) && !(value instanceof Date);
}

export function assertReturnsDefined<T>(value: T | null | undefined): NonNullable<T> {
	assertDefined(value, 'Value must not be null or undefined');
	return value;
}
