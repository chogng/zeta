import { Emitter, type Event } from "../../../../../base/common/event.js";
import { DisposableOwner } from "../../../../../base/common/lifecycle.js";
import { freezeDocumentNode, type DocumentNode } from "../../../common/document.js";
import { DocumentSchema } from "../../../common/schema.js";
import { validateDocumentSelection } from "../../../common/selection.js";
import { applyDocumentTransaction, DocumentTransaction, type DocumentStep } from "../../../common/transaction.js";
import { rebaseDocumentTransaction } from "./rebase.js";

export interface DocumentCollaborationEnvelope {
  readonly clientId: string;
  readonly sequence: number;
  readonly baseVersion: number;
  readonly transaction: DocumentTransaction;
}

export interface DocumentCollaborationRemoteEnvelope extends DocumentCollaborationEnvelope {
  readonly version: number;
}

export type DocumentCollaborationAcknowledgement = DocumentCollaborationRemoteEnvelope;

export interface DocumentCollaborationSessionOptions {
  readonly schema: DocumentSchema;
  readonly document: DocumentNode;
  readonly clientId: string;
  readonly version?: number;
}

export interface DocumentCollaborationChange {
  readonly kind: "local" | "remote" | "acknowledged";
  readonly document: DocumentNode;
  readonly canonicalDocument: DocumentNode;
  readonly pending: DocumentTransaction | undefined;
  readonly envelope: DocumentCollaborationEnvelope;
  readonly droppedSteps: readonly DocumentStep[];
}

/** Coordinates optimistic local edits with one canonical document snapshot. */
export class DocumentCollaborationSession extends DisposableOwner {
  private readonly changeEmitter = this.own(new Emitter<DocumentCollaborationChange>());
  private _canonicalDocument: DocumentNode;
  private _document: DocumentNode;
  private _pending: DocumentTransaction | undefined;
  private _version: number;
  private _sequence = 0;
  private disposed = false;

  readonly onDidChange: Event<DocumentCollaborationChange> = this.changeEmitter.event;

  constructor(options: DocumentCollaborationSessionOptions) {
    super();
    if (typeof options.clientId !== "string" || options.clientId.trim().length === 0) throw new TypeError("A collaboration client id is required");
    const version = options.version ?? 1;
    if (!Number.isSafeInteger(version) || version < 0) throw new RangeError("A collaboration document version must be a non-negative safe integer");
    const document = freezeDocumentNode(options.document);
    const schema = options.schema;
    schema.validate(document);
    this.schema = schema;
    this._canonicalDocument = document;
    this._document = document;
    this._version = version;
    this.clientId = options.clientId;
    this.defer(() => {
      this.disposed = true;
    });
  }

  readonly schema: DocumentSchema;

  readonly clientId: string;

  get document(): DocumentNode {
    this.ensureAlive();
    return this._document;
  }

  get canonicalDocument(): DocumentNode {
    this.ensureAlive();
    return this._canonicalDocument;
  }

  get version(): number {
    this.ensureAlive();
    return this._version;
  }

  get pending(): DocumentTransaction | undefined {
    this.ensureAlive();
    return this._pending;
  }

  get pendingSequence(): number | undefined {
    this.ensureAlive();
    return this._pending ? this._sequence : undefined;
  }

  /** Applies one optimistic local transaction and returns the cumulative pending update. */
  dispatchLocal(transaction: DocumentTransaction): DocumentCollaborationEnvelope | undefined {
    this.ensureAlive();
    const applied = applySessionTransaction(this._document, this.schema, transaction);
    this._document = applied.document;
    if (transaction.steps.length === 0) return undefined;
    this._pending = this._pending ? appendTransactions(this._pending, transaction) : transaction;
    this._sequence += 1;
    const envelope = this.createEnvelope();
    this.emitChange({ kind: "local", envelope, droppedSteps: [] });
    return envelope;
  }

  /** Applies a server-ordered remote update and replays the current pending batch on top. */
  receiveRemote(envelope: DocumentCollaborationRemoteEnvelope): DocumentCollaborationChange {
    this.ensureAlive();
    this.validateRemoteEnvelope(envelope);
    if (envelope.clientId === this.clientId) throw new DocumentCollaborationError("A local update must be acknowledged, not received as remote");
    const remote = applySessionTransaction(this._canonicalDocument, this.schema, envelope.transaction);
    const previousPending = this._pending;
    const rebased = previousPending ? rebaseDocumentTransaction(this._canonicalDocument, this.schema, previousPending, envelope.transaction) : undefined;
    const nextPending = rebased && rebased.transaction.steps.length > 0 ? rebased.transaction : undefined;
    this._canonicalDocument = remote.document;
    this._document = rebased?.document ?? remote.document;
    this._pending = nextPending;
    this._version = envelope.version;
    if (!nextPending) this._sequence = 0;
    return this.emitChange({ kind: "remote", envelope, droppedSteps: rebased?.droppedSteps ?? [] });
  }

  /** Commits the server-accepted form of the cumulative pending update. */
  acknowledge(envelope: DocumentCollaborationAcknowledgement): DocumentCollaborationChange {
    this.ensureAlive();
    this.validateRemoteEnvelope(envelope);
    if (envelope.clientId !== this.clientId) throw new DocumentCollaborationError("Only the local client can acknowledge its own update");
    if (!this._pending || envelope.sequence !== this._sequence) throw new DocumentCollaborationError("The acknowledgement does not match the current pending update");
    const committed = applySessionTransaction(this._canonicalDocument, this.schema, envelope.transaction);
    this._canonicalDocument = committed.document;
    this._document = committed.document;
    this._pending = undefined;
    this._version = envelope.version;
    return this.emitChange({ kind: "acknowledged", envelope, droppedSteps: [] });
  }

  private createEnvelope(): DocumentCollaborationEnvelope {
    if (!this._pending) throw new Error("A collaboration envelope requires a pending transaction");
    return Object.freeze({ clientId: this.clientId, sequence: this._sequence, baseVersion: this._version, transaction: this._pending });
  }

  private validateRemoteEnvelope(envelope: DocumentCollaborationRemoteEnvelope): void {
    if (!envelope || typeof envelope !== "object") throw new TypeError("A collaboration envelope is required");
    if (typeof envelope.clientId !== "string" || envelope.clientId.trim().length === 0) throw new TypeError("A collaboration envelope requires a client id");
    if (!Number.isSafeInteger(envelope.sequence) || envelope.sequence < 1) throw new RangeError("A collaboration envelope sequence must be a positive safe integer");
    if (!Number.isSafeInteger(envelope.baseVersion) || envelope.baseVersion !== this._version) throw new DocumentCollaborationError(`Collaboration update is based on version ${envelope.baseVersion}, expected ${this._version}`);
    if (!Number.isSafeInteger(envelope.version) || envelope.version <= envelope.baseVersion) throw new DocumentCollaborationError("A collaboration update must advance the document version");
    if (!(envelope.transaction instanceof DocumentTransaction)) throw new TypeError("A collaboration envelope requires a Gamma transaction");
  }

  private emitChange(options: { readonly kind: DocumentCollaborationChange["kind"]; readonly envelope: DocumentCollaborationEnvelope; readonly droppedSteps: readonly DocumentStep[] }): DocumentCollaborationChange {
    const change = Object.freeze({ kind: options.kind, document: this._document, canonicalDocument: this._canonicalDocument, pending: this._pending, envelope: options.envelope, droppedSteps: Object.freeze([...options.droppedSteps]) });
    this.changeEmitter.fire(change);
    return change;
  }

  private ensureAlive(): void {
    if (this.disposed) throw new ReferenceError("Document collaboration session is already disposed");
  }
}

export class DocumentCollaborationError extends Error {
  constructor(message: string, options?: ErrorOptions) {
    super(message, options);
    this.name = "DocumentCollaborationError";
  }
}

function applySessionTransaction(document: DocumentNode, schema: DocumentSchema, transaction: DocumentTransaction): { readonly document: DocumentNode } {
  const applied = applyDocumentTransaction(document, schema, transaction);
  if (transaction.selection) validateDocumentSelection(applied.document, transaction.selection);
  return applied;
}

function appendTransactions(previous: DocumentTransaction, next: DocumentTransaction): DocumentTransaction {
  return new DocumentTransaction([...previous.steps, ...next.steps], {
    addToHistory: previous.addToHistory && next.addToHistory,
    label: next.label,
    selection: next.selection,
    selectionSet: next.selectionSet,
    storedMarks: next.storedMarks,
    storedMarksSet: next.storedMarksSet,
    historyGroup: next.historyGroup,
    metadata: [...previous.metadata, ...next.metadata],
  });
}
