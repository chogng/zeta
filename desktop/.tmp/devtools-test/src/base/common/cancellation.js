/**
 * The project-level error used when an asynchronous operation observes
 * cancellation through an {@link AbortSignal}.
 *
 * APIs continue to accept the standard signal directly. This error only
 * provides a consistent local classification and preserves the abort reason.
 */
export class CancellationError extends Error {
    reason;
    constructor(message = "Operation cancelled", reason) {
        super(message, { cause: reason });
        this.reason = reason;
        this.name = "CancellationError";
    }
}
/**
 * Returns whether an error was produced by Zeta's cancellation layer.
 */
export function isCancellationError(error) {
    return error instanceof CancellationError;
}
/**
 * Throws a project cancellation error when the standard signal is aborted.
 */
export function throwIfCancelled(signal, message) {
    if (signal.aborted) {
        throw new CancellationError(message, signal.reason);
    }
}
