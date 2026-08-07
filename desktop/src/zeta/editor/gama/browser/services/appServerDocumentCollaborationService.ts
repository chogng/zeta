import { throwIfCancelled } from "../../../../base/common/cancellation.js";
import { Emitter } from "../../../../base/common/event.js";
import { DisposableOwner, type IDisposable } from "../../../../base/common/lifecycle.js";
import type { DocumentCollaborationSnapshot as AppServerDocumentCollaborationSnapshot, DocumentCollaborationUpdate as AppServerDocumentCollaborationUpdate } from "../../../../../../generated/app-server/types.js";
import type { IServerEventApi } from "../../../../platform/app-server/common/appServerApi.js";
import type { IDocumentCollaborationApi } from "../../../../platform/collaboration/common/documentCollaborationApi.js";
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

/** App Server transport adapter for Gama's server-ordered collaboration contract. */
export class AppServerDocumentCollaborationService extends DisposableOwner implements IDocumentCollaborationService {
  private readonly connections = new Map<string, Set<AppServerDocumentCollaborationConnection>>();

  constructor(private readonly api: IDocumentCollaborationApi, events: IServerEventApi) {
    super();
    const subscription = events.subscribe(event => {
      if (event.method !== "document/collaboration/update") return;
      const connections = this.connections.get(event.params.roomId);
      if (!connections) return;
      for (const connection of connections) connection.accept(event.params);
    });
    this.defer(() => subscription.dispose());
  }

  async open(input: DocumentCollaborationOpenInput, signal: AbortSignal): Promise<DocumentCollaborationConnection> {
    throwIfCancelled(signal, "Opening a Gama collaboration room was cancelled");
    const opened = await this.api.open({
      ...(input.roomId === undefined ? {} : { roomId: input.roomId }),
      clientId: input.clientId,
      schemaId: input.schemaId,
      document: serializeDocument(input.document, input.schema),
    });
    throwIfCancelled(signal, "Opening a Gama collaboration room was cancelled");
    const snapshot = decodeSnapshot(opened.snapshot, input.schema);
    const connection = new AppServerDocumentCollaborationConnection(this, input.schema, opened.clientId, snapshot);
    let roomConnections = this.connections.get(connection.roomId);
    if (!roomConnections) {
      roomConnections = new Set();
      this.connections.set(connection.roomId, roomConnections);
    }
    roomConnections.add(connection);
    return connection;
  }

  remove(connection: AppServerDocumentCollaborationConnection): void {
    const roomConnections = this.connections.get(connection.roomId);
    if (!roomConnections) return;
    roomConnections.delete(connection);
    if (roomConnections.size === 0) this.connections.delete(connection.roomId);
  }

  async submit(connection: AppServerDocumentCollaborationConnection, envelope: DocumentCollaborationEnvelope, document: DocumentNode, signal: AbortSignal): Promise<DocumentCollaborationSubmitOutcome> {
    throwIfCancelled(signal, "Submitting a Gama collaboration update was cancelled");
    const submitted = await this.api.submit({
      roomId: connection.roomId,
      clientId: connection.clientId,
      sequence: validateProtocolInteger(envelope.sequence, "sequence", 1),
      baseVersion: validateProtocolInteger(envelope.baseVersion, "baseVersion", 0),
      transaction: serializeDocumentTransaction(envelope.transaction, connection.schema),
      document: serializeDocument(document, connection.schema),
    });
    throwIfCancelled(signal, "Submitting a Gama collaboration update was cancelled");
    switch (submitted.status) {
      case "accepted": return { kind: "accepted", update: decodeUpdate(submitted.update, connection.schema) };
      case "conflict": return { kind: "conflict", updates: Object.freeze(submitted.updates.map(update => decodeUpdate(update, connection.schema))) };
      case "resync": return { kind: "resync", snapshot: decodeSnapshot(submitted.snapshot, connection.schema) };
    }
  }
}

class AppServerDocumentCollaborationConnection extends DisposableOwner implements DocumentCollaborationConnection {
  private readonly updateEmitter = this.own(new Emitter<DocumentCollaborationRemoteEnvelope>());
  private readonly snapshotEmitter = this.own(new Emitter<DocumentCollaborationSnapshot>());
  private readonly failureEmitter = this.own(new Emitter<Error>());
  private disposed = false;

  readonly onDidReceiveUpdate = this.updateEmitter.event;
  readonly onDidReceiveSnapshot = this.snapshotEmitter.event;
  readonly onDidFail = this.failureEmitter.event;

  constructor(private readonly service: AppServerDocumentCollaborationService, readonly schema: DocumentCollaborationConnection["schema"], readonly clientId: string, readonly initialSnapshot: DocumentCollaborationSnapshot) {
    super();
    this.roomId = initialSnapshot.roomId;
    this.defer(() => {
      this.disposed = true;
      service.remove(this);
    });
  }

  readonly roomId: string;

  submit(envelope: DocumentCollaborationEnvelope, document: DocumentNode, signal: AbortSignal): Promise<DocumentCollaborationSubmitOutcome> {
    if (this.disposed) return Promise.reject(new ReferenceError("Gama collaboration connection is disposed"));
    return this.service.submit(this, envelope, document, signal);
  }

  accept(value: AppServerDocumentCollaborationUpdate): void {
    if (this.disposed || value.roomId !== this.roomId) return;
    this.updateEmitter.fire(decodeUpdate(value, this.schema));
  }
}

function decodeSnapshot(value: AppServerDocumentCollaborationSnapshot, schema: DocumentCollaborationConnection["schema"]): DocumentCollaborationSnapshot {
  return Object.freeze({ roomId: value.roomId, version: validateProtocolInteger(value.version, "version", 0), document: deserializeDocument(value.document, schema) });
}

function decodeUpdate(value: AppServerDocumentCollaborationUpdate, schema: DocumentCollaborationConnection["schema"]): DocumentCollaborationRemoteEnvelope {
  return Object.freeze({
    clientId: value.clientId,
    sequence: validateProtocolInteger(value.sequence, "sequence", 1),
    baseVersion: validateProtocolInteger(value.baseVersion, "baseVersion", 0),
    version: validateProtocolInteger(value.version, "version", 1),
    transaction: deserializeDocumentTransaction(value.transaction, schema),
  });
}

function validateProtocolInteger(value: number, name: string, minimum: number): number {
  if (!Number.isSafeInteger(value) || value < minimum) throw new TypeError(`Gama collaboration ${name} must be a safe integer greater than or equal to ${minimum}`);
  return value;
}
