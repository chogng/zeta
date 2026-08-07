import { Emitter, type Event } from "../../../../../base/common/event.js";
import { DisposableOwner } from "../../../../../base/common/lifecycle.js";
import { DocumentModel } from "../../../common/model/documentModel.js";
import { serializeDocument } from "../../../common/model/documentSerialization.js";
import type { DocumentCollaborationConnection } from "../../../common/services/documentCollaborationService.js";
import type { DocumentCollaborationSnapshot } from "../../../common/services/documentCollaborationService.js";
import type { DocumentCollaborationSubmitOutcome } from "../../../common/services/documentCollaborationService.js";
import type { DocumentCollaborationEnvelope } from "./session.js";
import type { DocumentCollaborationRemoteEnvelope } from "./session.js";
import { DocumentCollaborationSession } from "./session.js";

export type DocumentCollaborationState = "connected" | "resyncRequired" | "error";

export interface DocumentCollaborationStateChange {
  readonly state: DocumentCollaborationState;
  readonly roomId: string;
  readonly message?: string;
}

/** Binds one Gama document model to a server-ordered collaboration connection. */
export class DocumentCollaborationController extends DisposableOwner {
  private readonly stateEmitter = this.own(new Emitter<DocumentCollaborationStateChange>());
  private readonly session: DocumentCollaborationSession;
  private submitting = false;
  private synchronizingModel = false;
  private disposed = false;

  readonly onDidChangeState: Event<DocumentCollaborationStateChange> = this.stateEmitter.event;

  constructor(private readonly model: DocumentModel, private readonly connection: DocumentCollaborationConnection) {
    super();
    this.session = this.own(new DocumentCollaborationSession({
      schema: model.schema,
      document: connection.initialSnapshot.document,
      clientId: connection.clientId,
      version: connection.initialSnapshot.version,
    }));
    this.own(connection);
    this.own(model.onDidChange(change => {
      if (this.synchronizingModel || change.origin !== "user") return;
      const envelope = this.session.dispatchLocal(change.transaction);
      if (envelope) this.submit(envelope);
    }));
    this.own(connection.onDidReceiveUpdate(update => this.acceptUpdate(update)));
    this.own(connection.onDidReceiveSnapshot(snapshot => this.acceptResync(snapshot)));
    this.own(connection.onDidFail(error => this.setState("error", error.message)));
    this.synchronizeModel();
    this.setState("connected");
    this.defer(() => {
      this.disposed = true;
    });
  }

  get roomId(): string {
    return this.connection.roomId;
  }

  get state(): DocumentCollaborationState {
    return this._state;
  }

  private _state: DocumentCollaborationState = "connected";

  private submit(envelope: DocumentCollaborationEnvelope): void {
    if (this.disposed || this.submitting || this._state !== "connected") return;
    const document = this.session.inFlightDocument;
    if (!document || this.session.inFlight?.sequence !== envelope.sequence) {
      this.setState("error", "The collaboration submission no longer matches the local document state");
      return;
    }
    this.submitting = true;
    void this.connection.submit(envelope, document, new AbortController().signal).then(
      outcome => this.acceptSubmitOutcome(outcome),
      error => this.setState("error", error instanceof Error ? error.message : "Submitting a collaboration update failed"),
    ).finally(() => {
      this.submitting = false;
      this.submitNext();
    });
  }

  private acceptSubmitOutcome(outcome: DocumentCollaborationSubmitOutcome): void {
    if (this.disposed || this._state !== "connected") return;
    switch (outcome.kind) {
      case "accepted":
        this.acceptUpdate(outcome.update);
        return;
      case "conflict":
        for (const update of outcome.updates) this.acceptUpdate(update);
        return;
      case "resync":
        this.acceptResync(outcome.snapshot);
        return;
    }
  }

  private acceptUpdate(update: DocumentCollaborationRemoteEnvelope): void {
    if (this.disposed || this._state !== "connected" || update.version <= this.session.version) return;
    try {
      if (update.clientId === this.connection.clientId) {
        if (this.session.inFlight?.sequence !== update.sequence) return;
        this.session.acknowledge(update);
      } else {
        this.session.receiveRemote(update);
      }
      this.synchronizeModel();
      this.submitNext();
    } catch (error) {
      this.setState("error", error instanceof Error ? error.message : "Applying a collaboration update failed");
    }
  }

  private acceptResync(snapshot: DocumentCollaborationSnapshot): void {
    try {
      this.session.replaceSnapshot(snapshot.document, snapshot.version);
      this.synchronizeModel();
      this.setState("connected");
    } catch (error) {
      this.setState("resyncRequired", error instanceof Error ? error.message : "Local updates require a manual collaboration resync");
    }
  }

  private submitNext(): void {
    if (this.submitting || this._state !== "connected") return;
    const envelope = this.session.takeNextSubmission();
    if (envelope) this.submit(envelope);
  }

  private synchronizeModel(): void {
    if (serializeDocument(this.model.document, this.model.schema) === serializeDocument(this.session.document, this.model.schema)) return;
    this.synchronizingModel = true;
    try {
      this.model.reset(this.session.document);
    } finally {
      this.synchronizingModel = false;
    }
  }

  private setState(state: DocumentCollaborationState, message?: string): void {
    if (this._state === state && message === undefined) return;
    this._state = state;
    this.stateEmitter.fire(Object.freeze({ state, roomId: this.connection.roomId, ...(message === undefined ? {} : { message }) }));
  }
}
