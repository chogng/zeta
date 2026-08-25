import { Emitter, type Event } from "../../../../base/common/event.js";
import { DisposableOwner } from "../../../../base/common/lifecycle.js";
import { rot } from "../../../../base/common/numbers.js";
import { EditorCommandHistoryMode, type EditorEditCommand } from "../../../common/commands/editorEditCommand.js";
import { type EditorSelectionController } from "../../../common/cursor/editorSelectionController.js";
import { type VersionedLanguageResult } from "../../../common/languages/languageRequestCoordinator.js";
import { type VersionedLanguageResultStore } from "../../../common/languages/languageResultStore.js";
import { assertLanguageCompletionCommitCharacter, LanguageCompletionInsertTextFormat, normalizeLanguageCompletionItemDetails, type LanguageCompletionItem, type LanguageCompletionItemDetails, type LanguageCompletionItemResolver, type LanguageCompletionResolveRequest, type LanguageCompletionResult } from "../../../common/languages/completion/languageCompletions.js";
import { parseLanguageCompletionSnippet, type LanguageCompletionSnippet, type LanguageCompletionSnippetVariableResolver } from "../../snippet/common/snippetParser.js";
import { LanguageCompletionSnippetSession } from "../../snippet/common/snippetSession.js";
import { normalizeTextLineEndings, type TextPosition } from "../../../common/core/text.js";
import { type TextModel } from "../../../common/model/textModel.js";

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
	readonly onDidAccept?: (item: LanguageCompletionItem) => void | Promise<void>;
	/** Editor-context variables made available to accepted completion snippets. */
	readonly snippetVariables?: LanguageCompletionSnippetVariableResolver;
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
	private readonly onDidAccept: ((item: LanguageCompletionItem) => void | Promise<void>) | undefined;
	private readonly snippetVariables: LanguageCompletionSnippetVariableResolver | undefined;
	private resolveController: AbortController | undefined;
	private snippetSession: LanguageCompletionSnippetSession | undefined;
	private accepting = false;

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
			if (options.onDidAccept !== undefined && typeof options.onDidAccept !== "function") throw new TypeError("Language completion accept handler must be a function");
			if (options.snippetVariables !== undefined && typeof options.snippetVariables.resolveVariable !== "function") {
				throw new TypeError("Language completion snippet variables require a resolver");
			}
			this.resolver = options.resolver;
			this.onResolveError = options.onResolveError ?? reportResolveError;
			this.onDidAccept = options.onDidAccept;
			this.snippetVariables = options.snippetVariables;
			this.currentState = this.createState(store.result);
			this.own(store.onDidChange(change => {
				if (!this.accepting) this.replaceState(change.result, LanguageCompletionSessionChangeReason.Store);
			}));
			this.own(selectionController.onDidChange(() => {
				if (!this.accepting) this.close(LanguageCompletionSessionChangeReason.Selection);
			}));
			this.defer(() => {
				this.cancelResolution("sessionDisposed");
				this.snippetSession?.dispose();
				this.snippetSession = undefined;
				const hadState = this.currentState !== undefined;
				this.currentState = undefined;
				if (hadState) this.fire(LanguageCompletionSessionChangeReason.Cancelled);
			});
			this.startResolution();
		} catch (error) {
			this.dispose();
			throw error;
		}
	}

	get textModel(): TextModel {
		this.assertNotDisposed();
		return this.store.textModel;
	}

	get resultStore(): VersionedLanguageResultStore<LanguageCompletionResult> {
		this.assertNotDisposed();
		return this.store;
	}

	get state(): LanguageCompletionSessionState | undefined {
		this.assertNotDisposed();
		return this.currentState;
	}

	selectNext(): boolean {
		return this.selectRelative(1);
	}

	selectPrevious(): boolean {
		return this.selectRelative(-1);
	}

	selectIndex(index: number): boolean {
		this.assertNotDisposed();
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
		this.assertNotDisposed();
		return this.close(LanguageCompletionSessionChangeReason.Cancelled);
	}

	acceptSelected(): boolean {
		return this.acceptSelectedWithCommitCharacter();
	}

	/** Accepts the focused item and writes its declared commit character in the same undo step. */
	acceptSelectedWithCommitCharacter(commitCharacter?: string): boolean {
		this.assertNotDisposed();
		const state = this.currentState;
		if (!state || !this.selectionMatches(state.position)) return false;
		if (commitCharacter !== undefined) {
			assertLanguageCompletionCommitCharacter(commitCharacter);
			if (!state.selectedItem.commitCharacters?.includes(commitCharacter)) return false;
		}
		const insertion = resolveLanguageCompletionInsertion(state.selectedItem, commitCharacter, this.textModel, this.snippetVariables);
		const command = createLanguageCompletionAcceptCommand(
			this.textModel,
			this.selectionController,
			state.selectedItem,
			commitCharacter,
			this.snippetVariables,
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
		if (insertion.snippet && insertion.snippet.placeholderGroups.length > 0) {
			this.snippetSession?.dispose();
			this.snippetSession = new LanguageCompletionSnippetSession(
				this.textModel,
				this.selectionController,
				insertion.resultStartOffset,
				insertion.snippet,
				insertion.text.length,
			);
		}
		if (state.selectedItem.command && this.onDidAccept) void Promise.resolve().then(() => this.onDidAccept!(state.selectedItem)).catch(this.onResolveError);
		this.close(LanguageCompletionSessionChangeReason.Accepted);
		return true;
	}

	/** Advances an active accepted-snippet tabstop sequence, if any. */
	selectNextSnippetPlaceholder(): boolean {
		this.assertNotDisposed();
		const session = this.snippetSession;
		if (!session) return false;
		const handled = session.selectNext();
		if (session.isDisposed) this.snippetSession = undefined;
		return handled;
	}

	/** Moves backwards through an active accepted-snippet tabstop sequence. */
	selectPreviousSnippetPlaceholder(): boolean {
		this.assertNotDisposed();
		const session = this.snippetSession;
		if (!session) return false;
		return session.selectPrevious();
	}

	/** Selects the next value of the active accepted-snippet choice tabstop. */
	selectNextSnippetChoice(): boolean {
		this.assertNotDisposed();
		return this.snippetSession?.selectNextChoice() ?? false;
	}

	/** Selects the previous value of the active accepted-snippet choice tabstop. */
	selectPreviousSnippetChoice(): boolean {
		this.assertNotDisposed();
		return this.snippetSession?.selectPreviousChoice() ?? false;
	}

	/** Stops tabstop navigation while preserving the expanded snippet text. */
	cancelSnippetPlaceholderNavigation(): boolean {
		this.assertNotDisposed();
		if (!this.snippetSession) return false;
		this.snippetSession.dispose();
		this.snippetSession = undefined;
		return true;
	}

	private selectRelative(delta: number): boolean {
		this.assertNotDisposed();
		const state = this.currentState;
		if (!state) return false;
		return this.selectIndex(rot(state.selectedIndex + delta, state.items.length));
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

export function createLanguageCompletionAcceptCommand(model: TextModel, selectionController: EditorSelectionController, item: LanguageCompletionItem, commitCharacter?: string, snippetVariables?: LanguageCompletionSnippetVariableResolver): EditorEditCommand {
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
	if (commitCharacter !== undefined) {
		assertLanguageCompletionCommitCharacter(commitCharacter);
		if (!item.commitCharacters?.includes(commitCharacter)) {
			throw new RangeError("Language completion item does not declare this commit character");
		}
	}
	const insertion = resolveLanguageCompletionInsertion(item, commitCharacter, model, snippetVariables);
	const insertText = insertion.text;
	const additionalTextEdits = item.additionalTextEdits ?? [];
	const selectionsAfter = insertion.snippet?.placeholderGroups[0]?.placeholders.map(placeholder => ({
		anchorOffset: insertion.resultStartOffset + placeholder.startOffset,
		activeOffset: insertion.resultStartOffset + placeholder.endOffset,
	})) ?? [{ anchorOffset: insertion.resultStartOffset + insertText.length, activeOffset: insertion.resultStartOffset + insertText.length }];
	return Object.freeze({
		edits: Object.freeze([{ range: item.range, text: insertText }, ...additionalTextEdits]),
		selectionsAfter: Object.freeze(selectionsAfter),
		primarySelectionIndex: 0,
		historyMode: EditorCommandHistoryMode.Isolated,
	});
}

interface LanguageCompletionInsertion {
	readonly text: string;
	readonly snippet: LanguageCompletionSnippet | undefined;
	readonly resultStartOffset: number;
}

function resolveLanguageCompletionInsertion(item: LanguageCompletionItem, commitCharacter: string | undefined, model?: TextModel, snippetVariables?: LanguageCompletionSnippetVariableResolver): LanguageCompletionInsertion {
	const snippet = item.insertTextFormat === LanguageCompletionInsertTextFormat.Snippet
		? parseLanguageCompletionSnippet(normalizeTextLineEndings(item.insertText), { variables: snippetVariables })
		: undefined;
	const text = (snippet?.text ?? normalizeTextLineEndings(item.insertText)) + (commitCharacter ?? "");
	const primaryStartOffset = model?.offsetAt(item.range.start) ?? 0;
	const offsetDelta = model
		? (item.additionalTextEdits ?? []).reduce((delta, edit) => edit.range.end.compareTo(item.range.start) < 0
			? delta + edit.text.length - (model.offsetAt(edit.range.end) - model.offsetAt(edit.range.start))
			: delta, 0)
		: 0;
	return Object.freeze({ text, snippet, resultStartOffset: primaryStartOffset + offsetDelta });
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
