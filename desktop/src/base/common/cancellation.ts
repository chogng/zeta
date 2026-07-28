/**
 * The project-level error used when an asynchronous operation observes
 * cancellation through an {@link AbortSignal}.
 *
 * APIs continue to accept the standard signal directly. This error only
 * provides a consistent local classification and preserves the abort reason.
 */
export class CancellationError extends Error {
  constructor(
    message = "Operation cancelled",
    readonly reason?: unknown,
  ) {
    super(message, { cause: reason });
    this.name = "CancellationError";
  }
}

/**
 * Returns whether an error was produced by Zeta's cancellation layer.
 */
export function isCancellationError(
  error: unknown,
): error is CancellationError {
  return error instanceof CancellationError;
}

/**
 * Throws a project cancellation error when the standard signal is aborted.
 */
export function throwIfCancelled(
  signal: AbortSignal,
  message?: string,
): void {
  if (signal.aborted) {
    throw new CancellationError(message, signal.reason);
  }
}
