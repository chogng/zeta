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

/**
 * Observes one promise until it settles or the caller's signal is cancelled.
 *
 * Cancellation does not attempt to stop the underlying operation. Its owner
 * remains responsible for that operation's lifecycle.
 */
export function raceCancellation<T>(
  promise: PromiseLike<T>,
  signal: AbortSignal,
  message = "Operation cancelled",
): Promise<T> {
  if (signal.aborted) {
    return Promise.reject(new CancellationError(message, signal.reason));
  }
  return new Promise<T>((resolve, reject) => {
    const abort = (): void => reject(new CancellationError(message, signal.reason));
    signal.addEventListener("abort", abort, { once: true });
    Promise.resolve(promise).then(resolve, reject).finally(() => {
      signal.removeEventListener("abort", abort);
    });
  });
}
