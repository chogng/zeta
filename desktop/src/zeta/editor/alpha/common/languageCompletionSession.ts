import { Emitter, type Event } from "../../../base/common/event.js";
import { DisposableOwner } from "../../../base/common/lifecycle.js";
import { EditorCommandHistoryMode, type EditorEditCommand, type EditorSelectionController } from "./editorSelectionController.js";
import { type VersionedLanguageResult } from "./languageRequestCoordinator.js";
import { type VersionedLanguageResultStore } from "./languageResultStore.js";
import { normalizeLanguageCompletionItemDetails, type LanguageCompletionItem, type LanguageCompletionItemDetails, type LanguageCompletionItemResolver, type LanguageCompletionResolveRequest, type LanguageCompletionResult } from "./languageCompletions.js";
import { normalizeTextLineEndings, type TextPosition } from "./text.js";
import { type TextModel } from "./textModel.js";

export enum LanguageCompletionSessionChangeReason {
  Store = "store",
  Focus = "focus",
  Selection = "selection",
  Cancelled = "cancelled",
  Accepted = "accepted",
  Details = "details",
}

export enum LanguageCompletionDetailsStatus {
  Complete = "complete",
  Loading = "loading",
  Failed = "failed",
  Unavailable = "unavailable",
}

export interface LanguageCompletionSessionState {
  readonly requestId: number;
  readonly modelVersion: number;
  readonly position: TextPosition;
  readonly items: readonly LanguageCompletionItem[];
  readonly selectedIndex: number;
  readonly selectedItem: LanguageCompletionItem;
  readonly isIncomplete: boolean;
  readonly detailsStatus: LanguageCompletionDetailsStatus;
  readonly details: LanguageCompletionItemDetails;
}

export interface LanguageCompletionSessionChange {
  readonly reason: LanguageCompletionSessionChangeReason;
  readonly state: LanguageCompletionSessionState | undefined;
}

export interface LanguageCompletionSessionOptions {
  readonly resolver?: LanguageCompletionItemResolver;
  readonly onResolveError?: (error: unknown) => void;
}

/**
 * Owns one editor instance's completion focus and acceptance lifecycle.
 *
 * The controller observes but does not own its result store, selection
 * controller, or text model.
 */
export class LanguageCompletionSessionController extends DisposableOwner {
  private readonly changeEmitter = this.own(new Emitter<LanguageCompletionSessionChange>());
  private currentState: LanguageCompletionSessionState | undefined;
  private readonly resolver: LanguageCompletionItemResolver | undefined;
  private readonly onResolveError: (error: unknown) => void;
  private resolveController: AbortController | undefined;
  private accepting = false;
  private disposed = false;

  readonly onDidChange: Event<LanguageCompletionSessionChange> = this.changeEmitter.event;

  constructor(
    private readonly store: VersionedLanguageResultStore<LanguageCompletionResult>,
    private readonly selectionController: EditorSelectionController,
    options: LanguageCompletionSessionOptions = {},
  ) {
    super();
    try {
      if (store.textModel !== selectionController.textModel) {
        throw new TypeError("Language completion store and selection controller must share one text model");
      }
      if (options.resolver !== undefined && typeof options.resolver.resolveCompletionItem !== "function") {
        throw new TypeError("Language completion session resolver must implement resolveCompletionItem");
      }
      if (options.onResolveError !== undefined && typeof options.onResolveError !== "function") {
        throw new TypeError("Language completion resolve error handler must be a function");
      }
      this.resolver = options.resolver;
      this.onResolveError = options.onResolveError ?? reportResolveError;
      this.currentState = this.createState(store.result);
      this.own(store.onDidChange(change => {
        if (!this.accepting) this.replaceState(change.result, LanguageCompletionSessionChangeReason.Store);
      }));
      this.own(selectionController.onDidChange(() => {
        if (!this.accepting) this.close(LanguageCompletionSessionChangeReason.Selection);
      }));
      this.defer(() => {
        this.cancelResolution("sessionDisposed");
        const hadState = this.currentState !== undefined;
        this.currentState = undefined;
        if (hadState) this.fire(LanguageCompletionSessionChangeReason.Cancelled);
        this.disposed = true;
      });
      this.startResolution();
    } catch (error) {
      this.dispose();
      throw error;
    }
  }

  get textModel(): TextModel {
    this.ensureAlive();
    return this.store.textModel;
  }

  get resultStore(): VersionedLanguageResultStore<LanguageCompletionResult> {
    this.ensureAlive();
    return this.store;
  }

  get state(): LanguageCompletionSessionState | undefined {
    this.ensureAlive();
    return this.currentState;
  }

  selectNext(): boolean {
    return this.selectRelative(1);
  }

  selectPrevious(): boolean {
    return this.selectRelative(-1);
  }

  selectIndex(index: number): boolean {
    this.ensureAlive();
    const state = this.currentState;
    if (!state) return false;
    if (!Number.isSafeInteger(index) || index < 0 || index >= state.items.length) {
      throw new RangeError(`Completion selection index must be between 0 and ${state.items.length - 1}`);
    }
    if (index === state.selectedIndex) return true;
    this.cancelResolution("selectionChanged");
    this.currentState = createStateSnapshot(state, index, this.resolver !== undefined);
    this.fire(LanguageCompletionSessionChangeReason.Focus);
    this.startResolution();
    return true;
  }

  cancel(): boolean {
    this.ensureAlive();
    return this.close(LanguageCompletionSessionChangeReason.Cancelled);
  }

  acceptSelected(): boolean {
    this.ensureAlive();
    const state = this.currentState;
    if (!state || !this.selectionMatches(state.position)) return false;
    const command = createLanguageCompletionAcceptCommand(
      this.textModel,
      this.selectionController,
      state.selectedItem,
    );
    this.accepting = true;
    try {
      this.selectionController.execute(command);
    } catch (error) {
      this.accepting = false;
      this.replaceState(this.store.result, LanguageCompletionSessionChangeReason.Store);
      throw error;
    }
    this.accepting = false;
    this.close(LanguageCompletionSessionChangeReason.Accepted);
    return true;
  }

  private selectRelative(delta: number): boolean {
    this.ensureAlive();
    const state = this.currentState;
    if (!state) return false;
    return this.selectIndex((state.selectedIndex + delta + state.items.length) % state.items.length);
  }

  private replaceState(result: VersionedLanguageResult<LanguageCompletionResult> | undefined, reason: LanguageCompletionSessionChangeReason): void {
    const next = this.createState(result);
    if (statesEqual(this.currentState, next)) return;
    this.cancelResolution("completionResultChanged");
    this.currentState = next;
    this.fire(reason);
    this.startResolution();
  }

  private createState(result: VersionedLanguageResult<LanguageCompletionResult> | undefined): LanguageCompletionSessionState | undefined {
    if (!result || result.value.items.length === 0 || !this.selectionMatches(result.value.position)) {
      return undefined;
    }
    const previousItem = this.currentState?.selectedItem;
    const retainedIndex = previousItem === undefined
      ? -1
      : result.value.items.findIndex(item => (
        item.providerId === previousItem.providerId &&
        item.id === previousItem.id
      ));
    const preselectedIndex = result.value.items.findIndex(item => item.preselect === true);
    const selectedIndex = retainedIndex >= 0
      ? retainedIndex
      : Math.max(0, preselectedIndex);
    return Object.freeze({
      requestId: result.requestId,
      modelVersion: result.modelVersion,
      position: result.value.position,
      items: result.value.items,
      selectedIndex,
      selectedItem: result.value.items[selectedIndex]!,
      isIncomplete: result.value.isIncomplete,
      ...createDetailsState(result.value.items[selectedIndex]!, this.resolver !== undefined),
    });
  }

  private selectionMatches(position: TextPosition): boolean {
    const selections = this.selectionController.selections;
    return selections.selections.length === 1 &&
      selections.primary.collapsed &&
      selections.primary.active.compareTo(position) === 0;
  }

  private close(reason: LanguageCompletionSessionChangeReason): boolean {
    if (!this.currentState) return false;
    this.cancelResolution("sessionClosed");
    this.currentState = undefined;
    this.fire(reason);
    return true;
  }

  private fire(reason: LanguageCompletionSessionChangeReason): void {
    this.changeEmitter.fire(Object.freeze({
      reason,
      state: this.currentState,
    }));
  }

  private ensureAlive(): void {
    if (this.disposed) {
      throw new ReferenceError("LanguageCompletionSessionController is already disposed");
    }
  }

  private startResolution(): void {
    const state = this.currentState;
    if (!state || state.detailsStatus !== LanguageCompletionDetailsStatus.Loading || !this.resolver) return;
    const controller = new AbortController();
    this.resolveController = controller;
    const request = createResolveRequest(state);
    void Promise.resolve().then(() => this.resolver!.resolveCompletionItem(request, controller.signal)).then(details => {
      if (controller.signal.aborted || this.currentState !== state) return;
      this.resolveController = undefined;
      this.currentState = Object.freeze({
        ...state,
        detailsStatus: LanguageCompletionDetailsStatus.Complete,
        details: mergeDetails(state.selectedItem, normalizeLanguageCompletionItemDetails(details)),
      });
      this.fire(LanguageCompletionSessionChangeReason.Details);
    }, error => {
      if (controller.signal.aborted || this.currentState !== state) return;
      this.resolveController = undefined;
      this.currentState = Object.freeze({
        ...state,
        detailsStatus: LanguageCompletionDetailsStatus.Failed,
      });
      this.fire(LanguageCompletionSessionChangeReason.Details);
      try {
        this.onResolveError(error);
      } catch (reportingError) {
        reportResolveError(new AggregateError([error, reportingError], "Completion resolution and error reporting both failed"));
      }
    });
  }

  private cancelResolution(reason: string): void {
    this.resolveController?.abort(reason);
    this.resolveController = undefined;
  }
}

export function createLanguageCompletionAcceptCommand(model: TextModel, selectionController: EditorSelectionController, item: LanguageCompletionItem): EditorEditCommand {
  if (model !== selectionController.textModel) {
    throw new TypeError("Language completion command and selection controller must share one text model");
  }
  const selections = selectionController.selections;
  if (selections.selections.length !== 1 || !selections.primary.collapsed) {
    throw new Error("Language completion acceptance requires one collapsed selection");
  }
  const position = selections.primary.active;
  if (item.range.start.compareTo(position) > 0 || item.range.end.compareTo(position) < 0) {
    throw new RangeError("Language completion item range must contain the active position");
  }
  const insertText = normalizeTextLineEndings(item.insertText);
  const caretOffset = model.offsetAt(item.range.start) + insertText.length;
  return Object.freeze({
    edits: Object.freeze([{ range: item.range, text: insertText }]),
    selectionsAfter: Object.freeze([{
      anchorOffset: caretOffset,
      activeOffset: caretOffset,
    }]),
    primarySelectionIndex: 0,
    historyMode: EditorCommandHistoryMode.Isolated,
  });
}

function createStateSnapshot(state: LanguageCompletionSessionState, selectedIndex: number, resolverAvailable: boolean): LanguageCompletionSessionState {
  const selectedItem = state.items[selectedIndex]!;
  return Object.freeze({
    ...state,
    selectedIndex,
    selectedItem,
    ...createDetailsState(selectedItem, resolverAvailable),
  });
}

function createDetailsState(item: LanguageCompletionItem, resolverAvailable: boolean): Pick<LanguageCompletionSessionState, "details" | "detailsStatus"> {
  return Object.freeze({
    details: mergeDetails(item, undefined),
    detailsStatus: item.hasDeferredDetails
      ? resolverAvailable
        ? LanguageCompletionDetailsStatus.Loading
        : LanguageCompletionDetailsStatus.Unavailable
      : LanguageCompletionDetailsStatus.Complete,
  });
}

function mergeDetails(item: LanguageCompletionItem, resolved: LanguageCompletionItemDetails | undefined): LanguageCompletionItemDetails {
  return normalizeLanguageCompletionItemDetails({
    ...(resolved?.detail === undefined && item.detail === undefined ? {} : { detail: resolved?.detail ?? item.detail }),
    ...(resolved?.documentation === undefined && item.documentation === undefined ? {} : { documentation: resolved?.documentation ?? item.documentation }),
  });
}

function createResolveRequest(state: LanguageCompletionSessionState): LanguageCompletionResolveRequest {
  return Object.freeze({
    completionRequestId: state.requestId,
    modelVersion: state.modelVersion,
    providerId: state.selectedItem.providerId,
    itemId: state.selectedItem.id,
  });
}

function statesEqual(left: LanguageCompletionSessionState | undefined, right: LanguageCompletionSessionState | undefined): boolean {
  return left === right || (
    left !== undefined &&
    right !== undefined &&
    left.requestId === right.requestId &&
    left.selectedIndex === right.selectedIndex
  );
}

function reportResolveError(error: unknown): void {
  console.error("Language completion item resolution failed", error);
}
