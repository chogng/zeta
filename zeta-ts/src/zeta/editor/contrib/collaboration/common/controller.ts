import { Emitter, type Event } from "../../../../base/common/event.js";
import { DisposableOwner } from "../../../../base/common/lifecycle.js";
import { DocumentModel, DocumentRemoteHistoryPolicy } from "../../../common/model/documentModel.js";
import { serializeDocument } from "../../../common/model/documentSerialization.js";
import type { DocumentCollaborationConnection } from "../../../common/services/documentCollaborationService.js";
import type { DocumentCollaborationInvite } from "../../../common/services/documentCollaborationService.js";
import type { DocumentCollaborationMember } from "../../../common/services/documentCollaborationService.js";
import type { DocumentCollaborationPresence } from "../../../common/services/documentCollaborationService.js";
import type { DocumentCollaborationRoomRole } from "../../../common/services/documentCollaborationService.js";
import type { DocumentCollaborationSnapshot } from "../../../common/services/documentCollaborationService.js";
import type { DocumentCollaborationSubmitOutcome } from "../../../common/services/documentCollaborationService.js";
import type { DocumentCollaborationEnvelope } from "./protocol.js";
import type { DocumentCollaborationRemoteEnvelope } from "./protocol.js";
import { rebaseDocumentHistory, rebaseDocumentTransaction } from "./rebase.js";
import { DocumentCollaborationSynchronizer } from "./synchronizer.js";
import { DocumentTransaction } from "../../../common/model/documentTransaction.js";

export type DocumentCollaborationState = "connected" | "resyncRequired" | "error";

export interface DocumentCollaborationStateChange {
  readonly state: DocumentCollaborationState;
  readonly roomId: string;
  readonly message?: string;
}

/** Current remote selections projected by one connected collaboration transport. */
export interface DocumentCollaborationPresenceChange {
  readonly presences: readonly DocumentCollaborationPresence[];
}

/** Binds one Aster document model to a server-ordered collaboration connection. */
export class DocumentCollaborationController extends DisposableOwner {
  private readonly stateEmitter = this.own(new Emitter<DocumentCollaborationStateChange>());
  private readonly presenceEmitter = this.own(new Emitter<DocumentCollaborationPresenceChange>());
  private readonly synchronizer: DocumentCollaborationSynchronizer;
  private submitting = false;
  private synchronizingModel = false;
  private disposed = false;
  private _presences: readonly DocumentCollaborationPresence[] = [];

  readonly onDidChangeState: Event<DocumentCollaborationStateChange> = this.stateEmitter.event;
  readonly onDidChangePresence: Event<DocumentCollaborationPresenceChange> = this.presenceEmitter.event;

  constructor(private readonly model: DocumentModel, private readonly connection: DocumentCollaborationConnection) {
    super();
    this.synchronizer = this.own(new DocumentCollaborationSynchronizer({
      schema: model.schema,
      document: connection.initialSnapshot.document,
      clientId: connection.clientId,
      version: connection.initialSnapshot.version,
    }));
    this._presences = connection.currentPresence;
    this.own(connection);
    this.own(model.onDidChange(change => {
      if (this.synchronizingModel || (change.origin !== "user" && change.origin !== "undo" && change.origin !== "redo")) return;
      const envelope = this.synchronizer.dispatchLocal(change.transaction);
      if (envelope) this.submit(envelope);
    }));
    this.own(model.onDidChangeSelection(selection => this.publishPresence(selection)));
    this.own(connection.onDidReceiveUpdate(update => this.acceptUpdate(update)));
    this.own(connection.onDidReceiveSnapshot(snapshot => this.acceptResync(snapshot)));
    this.own(connection.onDidReceivePresence(presences => {
      this._presences = presences;
      this.presenceEmitter.fire(Object.freeze({ presences }));
    }));
    this.own(connection.onDidFail(error => this.setState("error", error.message)));
    this.synchronizeModel();
    this.setState("connected");
    this.publishPresence(model.selection);
    this.defer(() => {
      this.disposed = true;
    });
  }

  get roomId(): string {
    return this.connection.roomId;
  }

  get canEdit(): boolean {
    return this.connection.canEdit;
  }

  get canManageMembers(): boolean {
    return this.connection.canManageMembers;
  }

  get principalId(): string | undefined {
    return this.connection.principalId;
  }

  get state(): DocumentCollaborationState {
    return this._state;
  }

  get presences(): readonly DocumentCollaborationPresence[] {
    return this._presences;
  }

  createInvite(displayName: string, role: DocumentCollaborationRoomRole): Promise<DocumentCollaborationInvite> {
    if (this.disposed) return Promise.reject(new ReferenceError("Aster collaboration controller is disposed"));
    if (!this.connection.canManageMembers) return Promise.reject(new Error("This collaboration member cannot create room invitations"));
    return this.connection.createInvite(displayName, role, new AbortController().signal);
  }

  listMembers(): Promise<readonly DocumentCollaborationMember[]> {
    if (this.disposed) return Promise.reject(new ReferenceError("Aster collaboration controller is disposed"));
    if (!this.connection.canManageMembers) return Promise.reject(new Error("This collaboration member cannot inspect room members"));
    return this.connection.listMembers(new AbortController().signal);
  }

  rotateMemberAccessToken(principalId: string): Promise<DocumentCollaborationInvite> {
    if (this.disposed) return Promise.reject(new ReferenceError("Aster collaboration controller is disposed"));
    if (!this.connection.canManageMembers) return Promise.reject(new Error("This collaboration member cannot manage room credentials"));
    return this.connection.rotateMemberAccessToken(principalId, new AbortController().signal);
  }

  revokeMember(principalId: string): Promise<void> {
    if (this.disposed) return Promise.reject(new ReferenceError("Aster collaboration controller is disposed"));
    if (!this.connection.canManageMembers) return Promise.reject(new Error("This collaboration member cannot manage room credentials"));
    return this.connection.revokeMember(principalId, new AbortController().signal);
  }

  private _state: DocumentCollaborationState = "connected";

  private submit(envelope: DocumentCollaborationEnvelope): void {
    if (this.disposed || this.submitting || this._state !== "connected") return;
    const document = this.synchronizer.inFlightDocument;
    if (!document || this.synchronizer.inFlight?.sequence !== envelope.sequence) {
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

  private publishPresence(selection: DocumentModel["selection"]): void {
    if (this.disposed || this._state !== "connected") return;
    void this.connection.updatePresence(selection, new AbortController().signal).catch(error => {
      if (!this.disposed) this.setState("error", error instanceof Error ? error.message : "Publishing collaboration presence failed");
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
    if (this.disposed || this._state !== "connected" || update.version <= this.synchronizer.version) return;
    try {
      if (update.clientId === this.connection.clientId) {
        if (this.synchronizer.inFlight?.sequence !== update.sequence) return;
        this.synchronizer.acknowledge(update);
      } else {
        this.receiveRemoteUpdate(update);
      }
      this.synchronizeModel();
      this.submitNext();
    } catch (error) {
      this.setState("error", error instanceof Error ? error.message : "Applying a collaboration update failed");
    }
  }

  private receiveRemoteUpdate(update: DocumentCollaborationRemoteEnvelope): void {
    const document = this.model.document;
    const remote = documentTransactionOnly(update.transaction);
    const pending = this.synchronizer.pending;
    const canPreserveHistory = serializeDocument(document, this.model.schema) === serializeDocument(this.synchronizer.document, this.model.schema);
    const projection = canPreserveHistory && pending ? rebaseDocumentTransaction(this.synchronizer.canonicalDocument, this.model.schema, remote, pending) : undefined;
    this.synchronizer.receiveRemote(update);
    if (!canPreserveHistory) return;
    if (projection && serializeDocument(projection.document, this.model.schema) !== serializeDocument(this.synchronizer.document, this.model.schema)) return;
    const transaction = projection?.transaction ?? remote;
    if (transaction.steps.length === 0) return;
    this.model.rebaseHistory(entries => rebaseDocumentHistory(document, this.model.schema, entries, transaction));
    this.synchronizingModel = true;
    try {
      this.model.dispatchRemote(transaction, DocumentRemoteHistoryPolicy.Preserve);
    } finally {
      this.synchronizingModel = false;
    }
  }

  private acceptResync(snapshot: DocumentCollaborationSnapshot): void {
    try {
      this.synchronizer.replaceSnapshot(snapshot.document, snapshot.version);
      this.synchronizeModel();
      this.setState("connected");
    } catch (error) {
      this.setState("resyncRequired", error instanceof Error ? error.message : "Local updates require a manual collaboration resync");
    }
  }

  private submitNext(): void {
    if (this.submitting || this._state !== "connected") return;
    const envelope = this.synchronizer.takeNextSubmission();
    if (envelope) this.submit(envelope);
  }

  private synchronizeModel(): void {
    if (serializeDocument(this.model.document, this.model.schema) === serializeDocument(this.synchronizer.document, this.model.schema)) return;
    this.synchronizingModel = true;
    try {
      this.model.reset(this.synchronizer.document);
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

function documentTransactionOnly(transaction: DocumentTransaction): DocumentTransaction {
  return new DocumentTransaction(transaction.steps, { addToHistory: false, label: transaction.label, metadata: transaction.metadata });
}
