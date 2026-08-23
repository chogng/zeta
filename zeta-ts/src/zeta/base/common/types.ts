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
