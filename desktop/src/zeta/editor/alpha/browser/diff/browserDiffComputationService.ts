import { DisposableOwner } from "../../../../base/common/lifecycle.js";
import { InlineDiffComputationService } from "../../common/models/diff/inlineDiffComputationService.js";
import { type DiffComputationRequest, type IDiffComputationService } from "../../common/models/diff/diffComputationService.js";
import { type LineDiff } from "../../common/models/diff/lineDiff.js";

interface PendingComputation {
  readonly requestId: number;
  readonly resolve: (diff: LineDiff) => void;
  readonly reject: (error: Error) => void;
  readonly removeAbortListener: () => void;
}

interface DiffWorkerResultMessage {
  readonly kind: "result";
  readonly requestId: number;
  readonly diff: LineDiff;
}

interface DiffWorkerFailureMessage {
  readonly kind: "failure";
  readonly requestId: number;
  readonly error: {
    readonly name: string;
    readonly message: string;
  };
}

/**
 * Browser diff service that runs each current computation in a dedicated Worker.
 *
 * Cancelling a request terminates its Worker immediately. DiffModel submits one
 * request at a time, so a replacement worker is created only for the next
 * current version and stale computation never blocks the renderer.
 */
export class BrowserDiffComputationService extends DisposableOwner implements IDiffComputationService {
  private readonly fallback = typeof Worker === "undefined"
    ? this.own(new InlineDiffComputationService())
    : undefined;
  private worker: Worker | undefined;
  private pending: PendingComputation | undefined;
  private messageListener: ((event: MessageEvent<unknown>) => void) | undefined;
  private errorListener: ((event: ErrorEvent) => void) | undefined;
  private messageErrorListener: (() => void) | undefined;
  private nextRequestId = 1;
  private disposed = false;

  constructor() {
    super();
    this.defer(() => {
      this.disposed = true;
      this.rejectPending(new ReferenceError("Browser diff computation service is already disposed"));
      this.disposeWorker();
    });
  }

  compute(request: DiffComputationRequest, signal: AbortSignal): Promise<LineDiff> {
    if (this.disposed) return Promise.reject(new ReferenceError("Browser diff computation service is already disposed"));
    const fallback = this.fallback;
    if (fallback) return fallback.compute(request, signal);
    if (this.pending) throw new Error("Browser diff computation service accepts one request at a time");
    signal.throwIfAborted();
    const worker = this.getWorker();
    const requestId = this.nextRequestId++;
    return new Promise<LineDiff>((resolve, reject) => {
      const abort = (): void => {
        if (this.pending?.requestId !== requestId) return;
        this.pending = undefined;
        signal.removeEventListener("abort", abort);
        reject(abortError(signal.reason));
        this.disposeWorker();
      };
      this.pending = {
        requestId,
        resolve,
        reject,
        removeAbortListener: () => signal.removeEventListener("abort", abort),
      };
      signal.addEventListener("abort", abort, { once: true });
      try {
        worker.postMessage(Object.freeze({ kind: "compute", requestId, request }));
      } catch (error) {
        if (this.pending?.requestId === requestId) {
          this.pending = undefined;
          signal.removeEventListener("abort", abort);
        }
        reject(asError(error));
        this.disposeWorker();
      }
    });
  }

  private getWorker(): Worker {
    const current = this.worker;
    if (current) return current;
    const worker = new Worker(
      new URL("./diffComputationWorkerMain.ts", import.meta.url),
      { type: "module", name: "zeta-diff" },
    );
    this.messageListener = event => this.receive(event.data);
    this.errorListener = event => this.fail(asError(event.error ?? new Error(event.message)));
    this.messageErrorListener = () => this.fail(new TypeError("Diff Worker returned an unreadable message"));
    worker.addEventListener("message", this.messageListener);
    worker.addEventListener("error", this.errorListener);
    worker.addEventListener("messageerror", this.messageErrorListener);
    this.worker = worker;
    return worker;
  }

  private receive(value: unknown): void {
    if (!isResultMessage(value) && !isFailureMessage(value)) return;
    const pending = this.pending;
    if (!pending || pending.requestId !== value.requestId) return;
    this.pending = undefined;
    pending.removeAbortListener();
    if (value.kind === "failure") {
      pending.reject(remoteError(value.error));
      return;
    }
    pending.resolve(value.diff);
  }

  private fail(error: Error): void {
    this.rejectPending(error);
    this.disposeWorker();
  }

  private rejectPending(error: Error): void {
    const pending = this.pending;
    if (!pending) return;
    this.pending = undefined;
    pending.removeAbortListener();
    pending.reject(error);
  }

  private disposeWorker(): void {
    const worker = this.worker;
    if (!worker) return;
    if (this.messageListener) worker.removeEventListener("message", this.messageListener);
    if (this.errorListener) worker.removeEventListener("error", this.errorListener);
    if (this.messageErrorListener) worker.removeEventListener("messageerror", this.messageErrorListener);
    this.worker = undefined;
    this.messageListener = undefined;
    this.errorListener = undefined;
    this.messageErrorListener = undefined;
    worker.terminate();
  }
}

function isResultMessage(value: unknown): value is DiffWorkerResultMessage {
  if (!isRecord(value)) return false;
  const requestId = value.requestId;
  return value.kind === "result" && typeof requestId === "number" && Number.isSafeInteger(requestId) && requestId > 0 && "diff" in value;
}

function isFailureMessage(value: unknown): value is DiffWorkerFailureMessage {
  if (!isRecord(value)) return false;
  const requestId = value.requestId;
  return value.kind === "failure" && typeof requestId === "number" && Number.isSafeInteger(requestId) && requestId > 0 && isRecord(value.error) && typeof value.error.name === "string" && typeof value.error.message === "string";
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

function remoteError(error: DiffWorkerFailureMessage["error"]): Error {
  const result = new Error(error.message);
  result.name = error.name;
  return result;
}

function abortError(reason: unknown): Error {
  if (reason instanceof Error) return reason;
  const error = new Error(reason === undefined ? "Diff computation was cancelled" : String(reason));
  error.name = "AbortError";
  return error;
}

function asError(error: unknown): Error {
  return error instanceof Error ? error : new Error(String(error));
}
