/** Preserves Error instances and wraps any other thrown value as an Error. */
export function toError(value: unknown): Error {
	return value instanceof Error ? value : new Error(String(value));
}

/** Returns a useful one-line description of an error-like value. */
export function getErrorMessage(err: any): string {
	if (!err) {
		return 'Error';
	}

	if (err.message) {
		return err.message;
	}

	if (err.stack) {
		return err.stack.split('\n')[0];
	}

	return String(err);
}
