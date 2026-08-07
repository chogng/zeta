import { throwIfCancelled } from "../../../../base/common/cancellation.js";
import { Emitter } from "../../../../base/common/event.js";
import { DisposableOwner } from "../../../../base/common/lifecycle.js";
import { deserializeDocument, serializeDocument } from "../../common/model/documentSerialization.js";
import { deserializeDocumentTransaction, serializeDocumentTransaction } from "../../common/model/documentTransactionSerialization.js";
import type { DocumentNode } from "../../common/model/document.js";
import type { DocumentCollaborationConnection } from "../../common/services/documentCollaborationService.js";
import type { DocumentCollaborationOpenInput } from "../../common/services/documentCollaborationService.js";
import type { DocumentCollaborationRemoteEnvelope } from "../../contrib/collaboration/common/session.js";
import type { DocumentCollaborationSnapshot } from "../../common/services/documentCollaborationService.js";
import type { DocumentCollaborationSubmitOutcome } from "../../common/services/documentCollaborationService.js";
import type { IDocumentCollaborationService } from "../../common/services/documentCollaborationService.js";
import type { DocumentCollaborationEnvelope } from "../../contrib/collaboration/common/session.js";

const API_ROOT = "/v1/document-collaboration";
const INITIAL_POLL_RETRY_DELAY_MS = 250;
const MAXIMUM_POLL_RETRY_DELAY_MS = 5_000;

class RemoteCollaborationRequestError extends Error {
  constructor(message: string, readonly status: number | undefined) {
    super(message);
  }
}

/** Fetch transport for the independently hosted durable Gama collaboration service. */
export class RemoteDocumentCollaborationService extends DisposableOwner implements IDocumentCollaborationService {
  private readonly connections = new Set<RemoteDocumentCollaborationConnection>();

  constructor() {
    super();
    this.defer(() => {
      for (const connection of [...this.connections]) connection.dispose();
    });
  }

  async open(input: DocumentCollaborationOpenInput, signal: AbortSignal): Promise<DocumentCollaborationConnection> {
    if (input.target?.kind !== "remote") throw new TypeError("Remote Gama collaboration requires a remote target");
    throwIfCancelled(signal, "Opening a remote Gama collaboration room was cancelled");
    const target = normalizeTarget(input.target.endpoint, input.target.bearerToken);
    const opened = await this.request(target, "rooms/open", {
      ...(input.roomId === undefined ? {} : { roomId: input.roomId }),
      clientId: input.clientId,
      schemaId: input.schemaId,
      document: serializeDocument(input.document, input.schema),
    }, signal);
    throwIfCancelled(signal, "Opening a remote Gama collaboration room was cancelled");
    const snapshot = decodeSnapshot(expectRecord(opened, "remote collaboration open response").snapshot, input.schema);
    const clientId = expectString(expectRecord(opened, "remote collaboration open response").clientId, "remote collaboration clientId");
    const connection = new RemoteDocumentCollaborationConnection(this, target, input.schema, clientId, snapshot);
    this.connections.add(connection);
    return connection;
  }

  remove(connection: RemoteDocumentCollaborationConnection): void {
    this.connections.delete(connection);
  }

  async submit(connection: RemoteDocumentCollaborationConnection, envelope: DocumentCollaborationEnvelope, document: DocumentNode, signal: AbortSignal): Promise<DocumentCollaborationSubmitOutcome> {
    throwIfCancelled(signal, "Submitting a remote Gama collaboration update was cancelled");
    const response = expectRecord(await this.request(connection.target, "rooms/submit", {
      roomId: connection.roomId,
      clientId: connection.clientId,
      sequence: validateProtocolInteger(envelope.sequence, "sequence", 1),
      baseVersion: validateProtocolInteger(envelope.baseVersion, "baseVersion", 0),
      transaction: serializeDocumentTransaction(envelope.transaction, connection.schema),
      document: serializeDocument(document, connection.schema),
    }, signal), "remote collaboration submit response");
    throwIfCancelled(signal, "Submitting a remote Gama collaboration update was cancelled");
    switch (expectString(response.status, "remote collaboration submit status")) {
      case "accepted": return { kind: "accepted", update: decodeUpdate(response.update, connection.schema) };
      case "conflict": return { kind: "conflict", updates: Object.freeze(expectArray(response.updates, "remote collaboration conflict updates").map(update => decodeUpdate(update, connection.schema))) };
      case "resync": return { kind: "resync", snapshot: decodeSnapshot(response.snapshot, connection.schema) };
      default: throw new TypeError("Remote Gama collaboration returned an unknown submit status");
    }
  }

  async poll(connection: RemoteDocumentCollaborationConnection, signal: AbortSignal): Promise<RemoteReplay> {
    const path = `rooms/${encodeURIComponent(connection.roomId)}/updates?afterVersion=${connection.version}`;
    const response = expectRecord(await this.request(connection.target, path, undefined, signal), "remote collaboration updates response");
    switch (expectString(response.status, "remote collaboration updates status")) {
      case "updates": return { kind: "updates", updates: Object.freeze(expectArray(response.updates, "remote collaboration updates").map(update => decodeUpdate(update, connection.schema))) };
      case "resync": return { kind: "resync", snapshot: decodeSnapshot(response.snapshot, connection.schema) };
      default: throw new TypeError("Remote Gama collaboration returned an unknown updates status");
    }
  }

  private async request(target: RemoteTarget, path: string, body: object | undefined, signal: AbortSignal): Promise<unknown> {
    let response: Response;
    try {
      response = await fetch(new URL(`${API_ROOT}/${path}`, target.endpoint), {
        method: body === undefined ? "GET" : "POST",
        headers: {
          Authorization: `Bearer ${target.bearerToken}`,
          ...(body === undefined ? {} : { "Content-Type": "application/json" }),
        },
        credentials: "omit",
        signal,
        ...(body === undefined ? {} : { body: JSON.stringify(body) }),
      });
    } catch (error) {
      throwIfCancelled(signal, "Remote Gama collaboration request was cancelled");
      throw new RemoteCollaborationRequestError(`Remote Gama collaboration is unavailable: ${error instanceof Error ? error.message : "network request failed"}`, undefined);
    }
    const payload: unknown = await response.json().catch(() => undefined);
    if (!response.ok) {
      const message = payload !== undefined && isRecord(payload) && typeof payload.error === "string" ? payload.error : `HTTP ${response.status}`;
      throw new RemoteCollaborationRequestError(`Remote Gama collaboration request failed: ${message}`, response.status);
    }
    return payload;
  }
}

class RemoteDocumentCollaborationConnection extends DisposableOwner implements DocumentCollaborationConnection {
  private readonly updateEmitter = this.own(new Emitter<DocumentCollaborationRemoteEnvelope>());
  private readonly snapshotEmitter = this.own(new Emitter<DocumentCollaborationSnapshot>());
  private readonly failureEmitter = this.own(new Emitter<Error>());
  private polling: AbortController | undefined;
  private disposed = false;
  private _version: number;

  readonly onDidReceiveUpdate = this.updateEmitter.event;
  readonly onDidReceiveSnapshot = this.snapshotEmitter.event;
  readonly onDidFail = this.failureEmitter.event;
  readonly roomId: string;

  constructor(private readonly service: RemoteDocumentCollaborationService, readonly target: RemoteTarget, readonly schema: DocumentCollaborationConnection["schema"], readonly clientId: string, readonly initialSnapshot: DocumentCollaborationSnapshot) {
    super();
    this.roomId = initialSnapshot.roomId;
    this._version = initialSnapshot.version;
    this.defer(() => {
      this.disposed = true;
      this.polling?.abort();
      service.remove(this);
    });
    void this.poll();
  }

  get version(): number {
    return this._version;
  }

  submit(envelope: DocumentCollaborationEnvelope, document: DocumentNode, signal: AbortSignal): Promise<DocumentCollaborationSubmitOutcome> {
    if (this.disposed) return Promise.reject(new ReferenceError("Remote Gama collaboration connection is disposed"));
    return this.service.submit(this, envelope, document, signal);
  }

  private async poll(): Promise<void> {
    let retryDelay = INITIAL_POLL_RETRY_DELAY_MS;
    while (!this.disposed) {
      const polling = new AbortController();
      this.polling = polling;
      try {
        const replay = await this.service.poll(this, polling.signal);
        retryDelay = INITIAL_POLL_RETRY_DELAY_MS;
        if (this.disposed || polling.signal.aborted) return;
        if (replay.kind === "resync") {
          this._version = replay.snapshot.version;
          this.snapshotEmitter.fire(replay.snapshot);
          continue;
        }
        for (const update of replay.updates) {
          if (update.version <= this._version) continue;
          this._version = update.version;
          this.updateEmitter.fire(update);
        }
      } catch (error) {
        if (this.disposed || polling.signal.aborted) return;
        const failure = error instanceof Error ? error : new Error("Remote Gama collaboration updates failed");
        if (!isRetryablePollFailure(failure)) {
          this.failureEmitter.fire(failure);
          return;
        }
        await waitForRetry(polling.signal, retryDelay);
        retryDelay = Math.min(retryDelay * 2, MAXIMUM_POLL_RETRY_DELAY_MS);
      } finally {
        if (this.polling === polling) this.polling = undefined;
      }
    }
  }
}

interface RemoteTarget {
  readonly endpoint: URL;
  readonly bearerToken: string;
}

type RemoteReplay = { readonly kind: "updates"; readonly updates: readonly DocumentCollaborationRemoteEnvelope[] } | { readonly kind: "resync"; readonly snapshot: DocumentCollaborationSnapshot };

function normalizeTarget(endpoint: string, bearerToken: string): RemoteTarget {
  let parsed: URL;
  try {
    parsed = new URL(endpoint);
  } catch {
    throw new TypeError("Remote Gama collaboration endpoint must be an absolute HTTP(S) URL");
  }
  if ((parsed.protocol !== "http:" && parsed.protocol !== "https:") || parsed.username || parsed.password || parsed.search || parsed.hash || parsed.pathname !== "/") throw new TypeError("Remote Gama collaboration endpoint must be an HTTP(S) origin without a path, credentials, query, or fragment");
  if (bearerToken.length < 32 || !/^[\x21-\x7e]+$/.test(bearerToken)) throw new TypeError("Remote Gama collaboration bearer token must contain at least 32 visible ASCII characters");
  return Object.freeze({ endpoint: parsed, bearerToken });
}

function decodeSnapshot(value: unknown, schema: DocumentCollaborationConnection["schema"]): DocumentCollaborationSnapshot {
  const record = expectRecord(value, "remote collaboration snapshot");
  return Object.freeze({ roomId: expectString(record.roomId, "remote collaboration roomId"), version: validateProtocolInteger(record.version, "version", 0), document: deserializeDocument(expectString(record.document, "remote collaboration document"), schema) });
}

function decodeUpdate(value: unknown, schema: DocumentCollaborationConnection["schema"]): DocumentCollaborationRemoteEnvelope {
  const record = expectRecord(value, "remote collaboration update");
  return Object.freeze({
    clientId: expectString(record.clientId, "remote collaboration clientId"),
    sequence: validateProtocolInteger(record.sequence, "sequence", 1),
    baseVersion: validateProtocolInteger(record.baseVersion, "baseVersion", 0),
    version: validateProtocolInteger(record.version, "version", 1),
    transaction: deserializeDocumentTransaction(expectString(record.transaction, "remote collaboration transaction"), schema),
  });
}

function expectRecord(value: unknown, name: string): Readonly<Record<string, unknown>> {
  if (!isRecord(value)) throw new TypeError(`${name} must be an object`);
  return value;
}

function expectArray(value: unknown, name: string): readonly unknown[] {
  if (!Array.isArray(value)) throw new TypeError(`${name} must be an array`);
  return value;
}

function expectString(value: unknown, name: string): string {
  if (typeof value !== "string" || value.length === 0) throw new TypeError(`${name} must be a non-empty string`);
  return value;
}

function isRecord(value: unknown): value is Readonly<Record<string, unknown>> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function validateProtocolInteger(value: unknown, name: string, minimum: number): number {
  if (typeof value !== "number" || !Number.isSafeInteger(value) || value < minimum) throw new TypeError(`Gama collaboration ${name} must be a safe integer greater than or equal to ${minimum}`);
  return value;
}

function isRetryablePollFailure(error: Error): boolean {
  if (!(error instanceof RemoteCollaborationRequestError)) return false;
  return error.status === undefined || error.status === 408 || error.status === 429 || error.status >= 500;
}

function waitForRetry(signal: AbortSignal, delay: number): Promise<void> {
  return new Promise(resolve => {
    if (signal.aborted) {
      resolve();
      return;
    }
    const complete = () => {
      clearTimeout(timeout);
      signal.removeEventListener("abort", complete);
      resolve();
    };
    const timeout = setTimeout(complete, delay);
    signal.addEventListener("abort", complete, { once: true });
  });
}
