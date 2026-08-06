import { Emitter, type Event } from "../../../base/common/event.js";
import { DisposableOwner } from "../../../base/common/lifecycle.js";
import { DocumentDecorationSet } from "./decoration.js";
import { DocumentHistory, type DocumentHistoryEntry } from "./history.js";
import { type DocumentPlugin, type DocumentPluginApplyContext, type DocumentPluginChangeOrigin, type DocumentPluginDecorationContext, type DocumentPluginInitContext, type DocumentPluginKey, type DocumentPluginSelectionContext, type DocumentPluginTransactionContext } from "./plugin.js";
import { isDocumentSelectionValid, selectionsEqual, validateDocumentSelection, type DocumentSelection } from "./selection.js";
import { freezeDocumentNode, type DocumentMark, type DocumentNode } from "./document.js";
import { DocumentSchema } from "./schema.js";
import { applyDocumentTransaction, DocumentTransaction } from "./transaction.js";

export type DocumentChangeOrigin = DocumentPluginChangeOrigin;

export interface DocumentChange {
  readonly version: number;
  readonly origin: DocumentChangeOrigin;
  readonly transaction: DocumentTransaction;
  readonly previousDocument: DocumentNode;
  readonly document: DocumentNode;
  readonly selectionBefore: DocumentSelection | undefined;
  readonly selectionAfter: DocumentSelection | undefined;
}

export interface DocumentModelOptions {
  readonly historyLimit?: number;
  readonly selection?: DocumentSelection;
  readonly storedMarks?: readonly DocumentMark[];
  readonly plugins?: readonly DocumentPlugin<unknown>[];
}

export interface DocumentPluginDecorationSource {
  readonly key: DocumentPluginKey<unknown>;
  readonly set: DocumentDecorationSet;
}

/** Mutable document session around immutable Gamma snapshots and transactions. */
export class DocumentModel extends DisposableOwner {
  private readonly changeEmitter = this.own(new Emitter<DocumentChange>());
  private readonly selectionEmitter = this.own(new Emitter<DocumentSelection | undefined>());
  private readonly storedMarksEmitter = this.own(new Emitter<readonly DocumentMark[] | undefined>());
  private readonly history: DocumentHistory;
  private readonly plugins: readonly DocumentPlugin<unknown>[];
  private pluginStates: Map<DocumentPluginKey<unknown>, unknown>;
  private _document: DocumentNode;
  private _selection: DocumentSelection | undefined;
  private _storedMarks: readonly DocumentMark[] | undefined;
  private _version = 1;
  private disposed = false;

  readonly onDidChange: Event<DocumentChange> = this.changeEmitter.event;
  readonly onDidChangeSelection: Event<DocumentSelection | undefined> = this.selectionEmitter.event;
  readonly onDidChangeStoredMarks: Event<readonly DocumentMark[] | undefined> = this.storedMarksEmitter.event;

  constructor(readonly schema: DocumentSchema, document = schema.createDocument(), options: DocumentModelOptions = {}) {
    super();
    const normalizedDocument = freezeDocumentNode(document);
    schema.validate(normalizedDocument);
    if (options.selection) validateDocumentSelection(normalizedDocument, options.selection);
    if (options.storedMarks) schema.validateMarks(options.storedMarks);
    this._document = normalizedDocument;
    this._selection = options.selection;
    this._storedMarks = cloneMarks(options.storedMarks);
    this.history = new DocumentHistory(options.historyLimit);
    const plugins = options.plugins ?? [];
    const pluginKeys = new Set<DocumentPluginKey<unknown>>();
    for (const plugin of plugins) {
      if (pluginKeys.has(plugin.key)) throw new TypeError(`Document plugin '${plugin.key.name}' is registered more than once`);
      pluginKeys.add(plugin.key);
    }
    this.plugins = Object.freeze([...plugins]);
    this.pluginStates = new Map();
    const initContext: DocumentPluginInitContext = Object.freeze({ schema, document: normalizedDocument, selection: options.selection, version: this._version });
    for (const plugin of this.plugins) this.pluginStates.set(plugin.key, plugin.state.init(initContext));
    this.defer(() => {
      this.disposed = true;
    });
  }

  get document(): DocumentNode {
    this.ensureAlive();
    return this._document;
  }

  get version(): number {
    this.ensureAlive();
    return this._version;
  }

  get selection(): DocumentSelection | undefined {
    this.ensureAlive();
    return this._selection;
  }

  /** Marks explicitly requested for the next text insertion, or undefined for inherited marks. */
  get storedMarks(): readonly DocumentMark[] | undefined {
    this.ensureAlive();
    return this._storedMarks;
  }

  get canUndo(): boolean {
    this.ensureAlive();
    return this.history.canUndo;
  }

  get canRedo(): boolean {
    this.ensureAlive();
    return this.history.canRedo;
  }

  /** Returns the current state owned by a registered plugin. */
  getPluginState<T>(key: DocumentPluginKey<T>): T | undefined {
    this.ensureAlive();
    return this.pluginStates.get(key) as T | undefined;
  }

  /** Returns each plugin-owned decoration set without merging plugin identities. */
  getPluginDecorations(): readonly DocumentPluginDecorationSource[] {
    this.ensureAlive();
    const context = { schema: this.schema, document: this._document, selection: this._selection, version: this._version };
    const sources: DocumentPluginDecorationSource[] = [];
    for (const plugin of this.plugins) {
      if (!plugin.decorations) continue;
      const state = this.pluginStates.get(plugin.key);
      const decorations = plugin.decorations(state, Object.freeze({ ...context, state }) as DocumentPluginDecorationContext<unknown>);
      if (decorations === undefined) continue;
      if (!(decorations instanceof DocumentDecorationSet)) throw new TypeError(`Document plugin '${plugin.key.name}' decorations must return a DocumentDecorationSet`);
      sources.push(Object.freeze({ key: plugin.key, set: decorations }));
    }
    return Object.freeze(sources);
  }

  dispatch(transaction: DocumentTransaction): DocumentChange | undefined {
    this.ensureAlive();
    if (transaction.steps.length === 0 && !transaction.selectionSet && !transaction.storedMarksSet) return undefined;
    if (!this.acceptsTransaction(transaction, "user")) return undefined;
    if (transaction.steps.length === 0) {
      const selectionBefore = this._selection;
      if (transaction.selectionSet && transaction.selection) validateDocumentSelection(this._document, transaction.selection);
      const selectionAfter = transaction.selectionSet ? transaction.selection : this._selection;
      const selectionChanged = !selectionsEqual(selectionBefore, selectionAfter);
      if (selectionChanged) {
        this.pluginStates = this.applyPluginSelection(this._document, selectionBefore, selectionAfter, this._version);
        this._selection = selectionAfter;
      }
      if (selectionChanged || transaction.storedMarksSet) this.history.closeGroup();
      if (transaction.storedMarksSet) this.setStoredMarks(transaction.storedMarks);
      if (selectionChanged) this.selectionEmitter.fire(selectionAfter);
      return Object.freeze({ version: this._version, origin: "user" as const, transaction, previousDocument: this._document, document: this._document, selectionBefore, selectionAfter });
    }
    const applied = applyDocumentTransaction(this._document, this.schema, transaction, this._selection);
    const selectionAfter = transaction.selectionSet ? transaction.selection : (isDocumentSelectionValid(applied.document, applied.selection) ? applied.selection : undefined);
    if (transaction.selection) validateDocumentSelection(applied.document, transaction.selection);
    const change = this.commit(applied.document, transaction, "user", this._selection, selectionAfter);
    if (transaction.addToHistory) {
      this.history.pushUndo({ transaction, inverse: applied.inverse, selectionBefore: change.selectionBefore, selectionAfter: change.selectionAfter, historyGroup: transaction.historyGroup });
    }
    if (transaction.storedMarksSet) this.setStoredMarks(transaction.storedMarks);
    return change;
  }

  /** Applies an already-transformed remote transaction; it is never added to local history, which is cleared until a collaboration layer can rebase it. */
  dispatchRemote(transaction: DocumentTransaction): DocumentChange | undefined {
    this.ensureAlive();
    if (transaction.steps.length === 0 && !transaction.selectionSet && !transaction.storedMarksSet) return undefined;
    if (!this.acceptsTransaction(transaction, "remote")) return undefined;
    if (transaction.steps.length === 0) {
      const selectionBefore = this._selection;
      if (transaction.selectionSet && transaction.selection) validateDocumentSelection(this._document, transaction.selection);
      const selectionAfter = transaction.selectionSet ? transaction.selection : this._selection;
      const selectionChanged = !selectionsEqual(selectionBefore, selectionAfter);
      if (selectionChanged) {
        this.pluginStates = this.applyPluginSelection(this._document, selectionBefore, selectionAfter, this._version);
        this._selection = selectionAfter;
      }
      if (transaction.storedMarksSet) this.setStoredMarks(transaction.storedMarks);
      if (selectionChanged) this.selectionEmitter.fire(selectionAfter);
      this.history.clear();
      return Object.freeze({ version: this._version, origin: "remote" as const, transaction, previousDocument: this._document, document: this._document, selectionBefore, selectionAfter });
    }
    const applied = applyDocumentTransaction(this._document, this.schema, transaction, this._selection);
    const selectionAfter = transaction.selectionSet ? transaction.selection : (isDocumentSelectionValid(applied.document, applied.selection) ? applied.selection : undefined);
    if (transaction.selection) validateDocumentSelection(applied.document, transaction.selection);
    const change = this.commit(applied.document, transaction, "remote", this._selection, selectionAfter);
    if (transaction.storedMarksSet) this.setStoredMarks(transaction.storedMarks);
    this.history.clear();
    return change;
  }

  undo(): DocumentChange | undefined {
    this.ensureAlive();
    this.history.closeGroup();
    const entry = this.history.takeUndo();
    if (!entry) return undefined;
    try {
      if (!this.acceptsTransaction(entry.inverse, "undo")) {
        this.history.restoreUndo(entry);
        return undefined;
      }
      const applied = applyDocumentTransaction(this._document, this.schema, entry.inverse);
      const change = this.commit(applied.document, entry.inverse, "undo", this._selection, entry.selectionBefore);
      this.history.pushRedo(entry);
      return change;
    } catch (error) {
      this.history.restoreUndo(entry);
      throw error;
    }
  }

  redo(): DocumentChange | undefined {
    this.ensureAlive();
    this.history.closeGroup();
    const entry = this.history.takeRedo();
    if (!entry) return undefined;
    try {
      if (!this.acceptsTransaction(entry.transaction, "redo")) {
        this.history.restoreRedo(entry);
        return undefined;
      }
      const applied = applyDocumentTransaction(this._document, this.schema, entry.transaction);
      const change = this.commit(applied.document, entry.transaction, "redo", this._selection, entry.selectionAfter);
      this.history.pushUndo(entry);
      return change;
    } catch (error) {
      this.history.restoreRedo(entry);
      throw error;
    }
  }

  setSelection(selection: DocumentSelection | undefined): void {
    this.ensureAlive();
    if (selection) validateDocumentSelection(this._document, selection);
    if (selectionsEqual(this._selection, selection)) return;
    const pluginStates = this.applyPluginSelection(this._document, this._selection, selection, this._version);
    this.history.closeGroup();
    this._selection = selection;
    this.pluginStates = pluginStates;
    this.selectionEmitter.fire(selection);
  }

  /** Sets the insertion marks without changing the document or its history. */
  setStoredMarks(marks: readonly DocumentMark[] | undefined): void {
    this.ensureAlive();
    if (marks) this.schema.validateMarks(marks);
    const normalized = cloneMarks(marks);
    if (marksEqual(this._storedMarks, normalized)) return;
    this._storedMarks = normalized;
    this.storedMarksEmitter.fire(normalized);
  }

  reset(document: DocumentNode): DocumentChange | undefined {
    this.ensureAlive();
    const normalizedDocument = freezeDocumentNode(document);
    this.schema.validate(normalizedDocument);
    if (normalizedDocument === this._document) return undefined;
    const previousDocument = this._document;
    const previousSelection = this._selection;
    const previousStoredMarks = this._storedMarks;
    const previousVersion = this._version;
    const version = previousVersion + 1;
    const transaction = new DocumentTransaction([], { addToHistory: false, label: "reset" });
    const change = Object.freeze({
      version,
      origin: "reset" as const,
      transaction,
      previousDocument,
      document: normalizedDocument,
      selectionBefore: previousSelection,
      selectionAfter: undefined,
    });
    const pluginStates = this.applyPluginStates({ schema: this.schema, previousDocument, document: normalizedDocument, transaction, previousSelection, selection: undefined, origin: "reset", previousVersion, version });
    this._document = normalizedDocument;
    this._selection = undefined;
    this._storedMarks = undefined;
    this._version = version;
    this.pluginStates = pluginStates;
    this.history.clear();
    this.changeEmitter.fire(change);
    if (previousSelection) this.selectionEmitter.fire(undefined);
    if (previousStoredMarks) this.storedMarksEmitter.fire(undefined);
    return change;
  }

  private commit(
    document: DocumentNode,
    transaction: DocumentTransaction,
    origin: DocumentChangeOrigin,
    selectionBefore: DocumentSelection | undefined,
    selectionAfter: DocumentSelection | undefined,
  ): DocumentChange {
    const previousDocument = this._document;
    const previousVersion = this._version;
    const version = previousVersion + 1;
    const change = Object.freeze({ version, origin, transaction, previousDocument, document, selectionBefore, selectionAfter });
    const pluginStates = this.applyPluginStates({ schema: this.schema, previousDocument, document, transaction, previousSelection: selectionBefore, selection: selectionAfter, origin, previousVersion, version });
    this._document = document;
    this._selection = selectionAfter;
    this._version = version;
    this.pluginStates = pluginStates;
    this.changeEmitter.fire(change);
    if (!selectionsEqual(selectionBefore, selectionAfter)) this.selectionEmitter.fire(selectionAfter);
    return change;
  }

  private applyPluginStates(context: DocumentPluginApplyContext): Map<DocumentPluginKey<unknown>, unknown> {
    const next = new Map<DocumentPluginKey<unknown>, unknown>();
    for (const plugin of this.plugins) {
      const previous = this.pluginStates.get(plugin.key);
      next.set(plugin.key, plugin.state.apply(previous, context));
    }
    return next;
  }

  private acceptsTransaction(transaction: DocumentTransaction, origin: DocumentPluginChangeOrigin): boolean {
    const context: DocumentPluginTransactionContext = Object.freeze({ schema: this.schema, document: this._document, selection: this._selection, origin, version: this._version });
    return this.plugins.every(plugin => plugin.filterTransaction?.(transaction, context) ?? true);
  }

  private applyPluginSelection(document: DocumentNode, previousSelection: DocumentSelection | undefined, selection: DocumentSelection | undefined, version: number): Map<DocumentPluginKey<unknown>, unknown> {
    const next = new Map(this.pluginStates);
    const context: DocumentPluginSelectionContext = Object.freeze({ schema: this.schema, document, previousSelection, selection, version });
    for (const plugin of this.plugins) {
      if (!plugin.state.applySelection) continue;
      next.set(plugin.key, plugin.state.applySelection(this.pluginStates.get(plugin.key), context));
    }
    return next;
  }

  private ensureAlive(): void {
    if (this.disposed) throw new ReferenceError("Document model is already disposed");
  }
}

function cloneMarks(marks: readonly DocumentMark[] | undefined): readonly DocumentMark[] | undefined {
  if (marks === undefined) return undefined;
  return Object.freeze(marks.map(mark => Object.freeze({ type: mark.type, attrs: Object.freeze({ ...(mark.attrs ?? {}) }) })));
}

function marksEqual(left: readonly DocumentMark[] | undefined, right: readonly DocumentMark[] | undefined): boolean {
  if (left === right) return true;
  if (!left || !right || left.length !== right.length) return false;
  return left.every((mark, index) => {
    const other = right[index];
    return mark.type === other?.type && JSON.stringify(mark.attrs) === JSON.stringify(other.attrs);
  });
}
