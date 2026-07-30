/**
 * Narrows code after a caller-defined condition or fails with its context.
 */
export function assert(
  condition: unknown,
  message: string,
): asserts condition {
  if (!condition) {
    throw new Error(message);
  }
}

/**
 * Narrows a value to its non-nullable type or fails with caller-owned context.
 */
export function assertDefined<T>(
  value: T,
  message: string,
): asserts value is NonNullable<T> {
  assert(value !== undefined && value !== null, message);
}
