import { Emitter, type Event } from "../../../../base/common/event.js";
import { DisposableOwner, type IDisposable } from "../../../../base/common/lifecycle.js";
import { LanguageWorkerResultDisposition, type LanguageWorker, type LanguageWorkerModelSynchronizer, type LanguageWorkerRequest, type LanguageWorkerResultSettler } from "./languageRequestCoordinator.js";
import { LanguageWorkerDocumentMirror, type LanguageWorkerDocumentChange, type LanguageWorkerDocumentSynchronizationObserver } from "./languageWorkerDocumentMirror.js";
import { assertRequestId, createCancelMessage, createFailureMessage, createResultMessage, createSyncFailureMessage, decodeClientMessage, decodeRequestSnapshot, decodeServerMessage, encodeRequestMessage, encodeSyncMessage, isProtocolMessage, readRequestId, type FailureWireMessage, type LanguageWorkerWireCodec, type LanguageWorkerWireResultState, type RequestWireMessage, type ResultWireMessage, type SyncFailureWireMessage } from "./languageWorkerWireProtocol.js";
import { type TextModelChange } from "../../common/text.js";

export { type LanguageWorkerWireCodec } from "./languageWorkerWireProtocol.js";

export interface LanguageWorkerWirePort extends IDisposable {
  readonly onMessage: Event<unknown>;
  send(message: unknown): void;
}

export interface LanguageWorkerWireClientPort extends LanguageWorkerWirePort {
  readonly onFailure: Event<unknown>;
}

/** Error reconstructed from a remote worker failure DTO. */
export class LanguageWorkerRemoteError extends Error {
  constructor(readonly remoteName: string, message: string) {
    super(message);
    this.name = "LanguageWorkerRemoteError";
  }
}

/** Coordinator-compatible client for one typed worker lane and model mirror. */
export class LanguageWorkerWireClient<TLane extends string, TPayload, TResult> extends DisposableOwner implements LanguageWorker<TLane, TPayload, TResult>, LanguageWorkerModelSynchronizer, LanguageWorkerResultSettler {
  private readonly failureEmitter = this.own(new Emitter<Error>());
  private readonly pending = new Map<number, PendingWireRequest<TLane, TPayload, TResult>>();
  private readonly resultStates = new Map<TLane, LanguageWorkerWireResultState<TResult>>();
  private readonly stagedResultStates = new Map<number, StagedWireResultState<TLane, TResult>>();
  private mirroredVersion: number | undefined;
  private terminalFailure: Error | undefined;
  private disposed = false;

  readonly onDidFail: Event<Error> = this.failureEmitter.event;

  constructor(
    private readonly port: LanguageWorkerWireClientPort,
    private readonly codec: LanguageWorkerWireCodec<TLane, TPayload, TResult>,
  ) {
    super();
    assertPort(port, true);
    assertCodec(codec);
    this.own(port);
    this.own(port.onMessage(message => this.receive(message)));
    this.own(port.onFailure(error => this.fail(asError(error, "Language worker transport failed"))));
    this.defer(() => {
      this.disposed = true;
      this.mirroredVersion = undefined;
      this.resultStates.clear();
      this.stagedResultStates.clear();
      this.failAll(new ReferenceError("LanguageWorkerWireClient is already disposed"));
    });
  }

  run(request: LanguageWorkerRequest<TLane, TPayload>, signal: AbortSignal): Promise<TResult> {
    this.ensureAvailable();
    assertRequestId(request.requestId);
    if (!this.codec.lanes.includes(request.lane)) {
      throw new RangeError(`Language worker wire lane '${request.lane}' is unsupported`);
    }
    if (this.pending.has(request.requestId)) {
      throw new RangeError(`Language worker wire request '${request.requestId}' is already pending`);
    }
    signal.throwIfAborted();
    const resultBase = this.codec.resultProtocol === "confirmedBase" ? this.resultStates.get(request.lane) : undefined;
    const encoded = encodeRequestMessage(request, this.codec, this.mirroredVersion, resultBase?.requestId);
    return new Promise<TResult>((resolve, reject) => {
      const abort = (): void => {
        const pending = this.pending.get(request.requestId);
        if (!pending) return;
        this.pending.delete(request.requestId);
        pending.removeAbort();
        try {
          this.port.send(createCancelMessage(request.requestId));
        } catch {
          // The local cancellation outcome remains authoritative.
        }
        reject(abortError(signal.reason));
      };
      signal.addEventListener("abort", abort, { once: true });
      const pending: PendingWireRequest<TLane, TPayload, TResult> = {
        request,
        resultBase,
        resolve,
        reject,
        removeAbort: () => signal.removeEventListener("abort", abort),
      };
      this.pending.set(request.requestId, pending);
      try {
        this.port.send(encoded.message);
        if (encoded.establishesMirrorVersion !== undefined) {
          this.mirroredVersion = encoded.establishesMirrorVersion;
        }
      } catch (error) {
        this.pending.delete(request.requestId);
        pending.removeAbort();
        this.mirroredVersion = undefined;
        reject(error);
      }
    });
  }

  synchronizeModel(change: TextModelChange): void {
    this.ensureAvailable();
    if (this.mirroredVersion === undefined) return;
    if (change.version !== this.mirroredVersion + 1) {
      this.mirroredVersion = undefined;
      return;
    }
    const message = encodeSyncMessage(change);
    try {
      this.port.send(message);
      this.mirroredVersion = change.version;
    } catch {
      this.mirroredVersion = undefined;
    }
  }

  invalidate(error: unknown): void {
    this.ensureAvailable();
    this.fail(asError(error, "Language worker client was invalidated"));
  }

  settleResult(requestId: number, disposition: LanguageWorkerResultDisposition): void {
    const staged = this.stagedResultStates.get(requestId);
    if (!staged) return;
    this.stagedResultStates.delete(requestId);
    if (disposition !== LanguageWorkerResultDisposition.Applied) return;
    const current = this.resultStates.get(staged.lane);
    if (!current || requestId > current.requestId) {
      this.resultStates.set(staged.lane, staged.state);
    }
  }

  private receive(value: unknown): void {
    if (!isProtocolMessage(value)) return;
    let message: ResultWireMessage | FailureWireMessage | SyncFailureWireMessage;
    try {
      message = decodeClientMessage(value);
    } catch (error) {
      this.fail(asError(error, "Invalid language worker response"));
      return;
    }
    if (message.kind === "syncFailure") {
      this.fail(new LanguageWorkerRemoteError(message.error.name, message.error.message));
      return;
    }
    const pending = this.pending.get(message.requestId);
    if (!pending) return;
    this.pending.delete(message.requestId);
    pending.removeAbort();
    if (message.kind === "failure") {
      pending.reject(new LanguageWorkerRemoteError(message.error.name, message.error.message));
      return;
    }
    try {
      const result = this.codec.decodeResult(pending.request.lane, message.result, pending.request.snapshot, pending.resultBase);
      if (this.codec.resultProtocol === "confirmedBase") {
        this.stagedResultStates.set(message.requestId, Object.freeze({
          lane: pending.request.lane,
          state: Object.freeze({
            requestId: message.requestId,
            snapshot: pending.request.snapshot,
            result,
          }),
        }));
      }
      pending.resolve(result);
    } catch (error) {
      pending.reject(error);
    }
  }

  private failAll(error: Error): void {
    const pending = [...this.pending.values()];
    this.pending.clear();
    for (const request of pending) {
      request.removeAbort();
      request.reject(error);
    }
  }

  private fail(error: Error): void {
    const firstFailure = this.terminalFailure === undefined;
    if (firstFailure) this.terminalFailure = error;
    const terminalFailure = this.terminalFailure!;
    this.mirroredVersion = undefined;
    this.resultStates.clear();
    this.stagedResultStates.clear();
    this.failAll(terminalFailure);
    if (firstFailure) this.failureEmitter.fire(terminalFailure);
  }

  private ensureAvailable(): void {
    if (this.disposed) {
      throw new ReferenceError("LanguageWorkerWireClient is already disposed");
    }
    if (this.terminalFailure) throw this.terminalFailure;
  }
}

/** Worker-side dispatcher for one typed language lane and immutable mirror. */
export class LanguageWorkerWireServer<TLane extends string, TPayload, TResult> extends DisposableOwner {
  private readonly active = new Map<number, AbortController>();
  private readonly resultStates = new Map<TLane, LanguageWorkerWireResultState<TResult>>();
  private mirror: LanguageWorkerDocumentMirror | undefined;
  private disposed = false;

  constructor(
    private readonly port: LanguageWorkerWirePort,
    private readonly codec: LanguageWorkerWireCodec<TLane, TPayload, TResult>,
    private readonly worker: LanguageWorker<TLane, TPayload, TResult>,
  ) {
    super();
    assertPort(port, false);
    assertCodec(codec);
    assertWorker(worker);
    this.own(port);
    this.own(worker);
    this.own(port.onMessage(message => this.receive(message)));
    this.defer(() => {
      this.disposed = true;
      this.mirror = undefined;
      this.resultStates.clear();
      for (const controller of this.active.values()) {
        controller.abort("serverDisposed");
      }
      this.active.clear();
    });
  }

  private receive(value: unknown): void {
    if (!isProtocolMessage(value)) return;
    let message: ReturnType<typeof decodeServerMessage>;
    try {
      message = decodeServerMessage(value);
    } catch (error) {
      const requestId = readRequestId(value);
      if (requestId !== undefined) this.sendFailure(requestId, error);
      return;
    }
    if (message.kind === "cancel") {
      this.active.get(message.requestId)?.abort("clientCancelled");
      return;
    }
    if (message.kind === "sync") {
      try {
        if (!this.mirror) {
          throw new Error("Language worker sync requires an initialized document mirror");
        }
        const changes = normalizeDocumentChanges(message.changes);
        this.mirror.synchronize(message.previousVersion, message.modelVersion, changes);
        if (supportsDocumentSynchronization(this.worker)) {
          this.worker.synchronizeDocument(Object.freeze({
            previousVersion: message.previousVersion,
            modelVersion: message.modelVersion,
            changes,
            snapshot: this.mirror.createSnapshot(),
          }));
        }
      } catch (error) {
        this.mirror = undefined;
        this.port.send(createSyncFailureMessage(error));
      }
      return;
    }
    void this.runRequest(message);
  }

  private async runRequest(message: RequestWireMessage): Promise<void> {
    if (this.disposed) return;
    if (this.active.has(message.requestId)) {
      this.sendFailure(message.requestId, new RangeError(`Duplicate language worker request '${message.requestId}'`));
      return;
    }
    let request: LanguageWorkerRequest<TLane, TPayload>;
    try {
      if (!this.codec.lanes.includes(message.lane as TLane)) {
        throw new RangeError(`Unsupported language worker lane '${message.lane}'`);
      }
      const decoded = decodeRequestSnapshot(message.snapshot, this.mirror?.createSnapshot());
      if (decoded.replacesMirror) this.mirror = new LanguageWorkerDocumentMirror(decoded.snapshot);
      request = Object.freeze({
        requestId: message.requestId,
        lane: message.lane as TLane,
        snapshot: decoded.snapshot,
        payload: this.codec.decodePayload(message.lane as TLane, message.payload, decoded.snapshot),
      });
    } catch (error) {
      this.sendFailure(message.requestId, error);
      return;
    }

    const controller = new AbortController();
    this.active.set(message.requestId, controller);
    const currentBase = this.codec.resultProtocol === "confirmedBase" ? this.resultStates.get(request.lane) : undefined;
    const resultBase = currentBase?.requestId === message.resultBaseRequestId ? currentBase : undefined;
    try {
      const result = await this.worker.run(request, controller.signal);
      if (!this.disposed && !controller.signal.aborted) {
        const encoded = this.codec.encodeResult(request.lane, result, request.snapshot, resultBase);
        this.port.send(createResultMessage(message.requestId, encoded));
        const current = this.resultStates.get(request.lane);
        if (this.codec.resultProtocol === "confirmedBase" && (!current || message.requestId > current.requestId)) {
          this.resultStates.set(request.lane, Object.freeze({
            requestId: message.requestId,
            snapshot: request.snapshot,
            result,
          }));
        }
      }
    } catch (error) {
      if (!this.disposed && !controller.signal.aborted) {
        this.sendFailure(message.requestId, error);
      }
    } finally {
      if (this.active.get(message.requestId) === controller) {
        this.active.delete(message.requestId);
      }
    }
  }

  private sendFailure(requestId: number, error: unknown): void {
    if (this.disposed) return;
    this.port.send(createFailureMessage(requestId, error));
  }
}

interface PendingWireRequest<TLane extends string, TPayload, TResult> {
  readonly request: LanguageWorkerRequest<TLane, TPayload>;
  readonly resultBase: LanguageWorkerWireResultState<TResult> | undefined;
  readonly resolve: (value: TResult) => void;
  readonly reject: (reason: unknown) => void;
  readonly removeAbort: () => void;
}

interface StagedWireResultState<TLane extends string, TResult> {
  readonly lane: TLane;
  readonly state: LanguageWorkerWireResultState<TResult>;
}

function assertPort(value: LanguageWorkerWirePort, requireFailure: boolean): void {
  if (!value || typeof value.send !== "function" || typeof value.onMessage !== "function" || typeof value.dispose !== "function" || (requireFailure && typeof (value as LanguageWorkerWireClientPort).onFailure !== "function")) {
    throw new TypeError("Language worker wire port is invalid");
  }
}

function assertCodec<TLane extends string, TPayload, TResult>(value: LanguageWorkerWireCodec<TLane, TPayload, TResult>): void {
  if (!value || !Array.isArray(value.lanes) || value.lanes.length === 0 || value.lanes.some(lane => typeof lane !== "string" || lane.length === 0) || new Set(value.lanes).size !== value.lanes.length || (value.resultProtocol !== "stateless" && value.resultProtocol !== "confirmedBase") || typeof value.encodePayload !== "function" || typeof value.decodePayload !== "function" || typeof value.encodeResult !== "function" || typeof value.decodeResult !== "function") {
    throw new TypeError("Language worker wire codec is invalid");
  }
}

function assertWorker<TLane extends string, TPayload, TResult>(value: LanguageWorker<TLane, TPayload, TResult>): void {
  if (!value || typeof value.run !== "function" || typeof value.dispose !== "function" || typeof value[Symbol.dispose] !== "function") {
    throw new TypeError("Language worker wire server worker is invalid");
  }
}

function supportsDocumentSynchronization(value: Disposable): value is Disposable & LanguageWorkerDocumentSynchronizationObserver {
  return typeof (value as Partial<LanguageWorkerDocumentSynchronizationObserver>).synchronizeDocument === "function";
}

function normalizeDocumentChanges(changes: readonly LanguageWorkerDocumentChange[]): readonly LanguageWorkerDocumentChange[] {
  return Object.freeze(changes.map(change => Object.freeze({
    rangeOffset: change.rangeOffset,
    rangeLength: change.rangeLength,
    text: change.text,
  })));
}

function abortError(reason: unknown): Error {
  if (reason instanceof Error) return reason;
  const error = new Error(reason === undefined ? "Language worker request was cancelled" : String(reason));
  error.name = "AbortError";
  return error;
}

function asError(value: unknown, fallbackMessage: string): Error {
  return value instanceof Error ? value : new Error(value === undefined ? fallbackMessage : String(value));
}
