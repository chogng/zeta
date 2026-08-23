import { addDisposableListener, stopEvent, h } from "../../../base/browser/dom.js";
import { FastDomNode } from "../../../base/browser/fastDomNode.js";
import { DisposableOwner, toDisposable, type IDisposable } from "../../../base/common/lifecycle.js";
import { createBackspaceCommand, createDeleteForwardCommand, createDeleteToLineEndCommand, createDeleteToLineStartCommand } from "../../common/cursor/cursorDeleteOperations.js";
import { createDeleteWordBackwardCommand, createDeleteWordForwardCommand } from "../../common/cursor/cursorWordOperations.js";
import { createTypeTextCommand } from "../../common/cursor/cursorTypeOperations.js";
import { type EditorEditCommand } from "../../common/commands/editorEditCommand.js";
import { type EditorSelectionController } from "../../common/cursor/editorSelectionController.js";
import { type LanguageCompletionService } from "../../common/languages/completion/languageCompletionService.js";
import { type LanguageCompletionResult } from "../../common/languages/completion/languageCompletions.js";
import { type VersionedLanguageResultStore } from "../../common/languages/languageResultStore.js";
import { type LanguageConfigurationSource } from "../../common/languages/languageConfiguration.js";
import { type LanguageLexicalContextSource } from "../../common/languages/languageLexicalContext.js";
import { createLanguageCompletionIncompleteRefreshContext, createLanguageCompletionInvokeContext, type LanguageCompletionContext } from "../../common/languages/completion/languageCompletionProviders.js";
import { createOvertypeTextCommand } from "../../common/cursor/cursorOvertype.js";
import { TextRange, type TextModelChange } from "../../common/core/text.js";
import { TextSelection, TextSelectionSet } from "../../common/core/selection.js";
import { type EditorViewport } from "../view/editorViewport.js";
import { CompositionController } from "./compositionController.js";

const MAXIMUM_ACCESSIBLE_INPUT_TEXT_UNITS = 32 * 1_024;

export interface TextInputControllerOptions {
	readonly ariaLabel?: string;
	/** Compatibility seam for explicitly selected clipboard adapters. */
	readonly clipboard?: unknown;
	readonly completion?: TextInputCompletionOptions;
	/** Compatibility seams implemented by the optional language-editing contribution. */
	readonly indentation?: TextInputIndentationOptions;
	readonly language?: TextInputLanguageOptions;
	readonly languageEditing?: TextInputLanguageEditingAdapter;
	readonly wordPattern?: () => RegExp | undefined;
}

export interface TextInputCommandContext {
	readonly inputType: string;
}

/** Extends one native input command before it becomes an atomic model transaction. */
export type TextInputCommandTransformer = (command: EditorEditCommand, context: TextInputCommandContext) => EditorEditCommand;

export interface TextInputIndentationOptions {
	readonly kind?: "tabs" | "spaces";
	readonly tabSize?: number;
}

export interface TextInputLanguageOptions {
	readonly languageId: string;
	readonly configurations: LanguageConfigurationSource;
	readonly lexicalContext?: LanguageLexicalContextSource;
}

export type TextInputClipboardFactory = (element: HTMLTextAreaElement, viewport: EditorViewport, selections: EditorSelectionController, options: unknown, isEditingAllowed: () => boolean) => IDisposable;

let clipboardFactory: TextInputClipboardFactory | undefined;

/** Registers the optional clipboard adapter without introducing a browser-to-contrib dependency. */
export function registerTextInputClipboardFactory(factory: TextInputClipboardFactory): void {
	if (typeof factory !== "function") throw new TypeError("Text input clipboard factory must be a function");
	if (clipboardFactory && clipboardFactory !== factory) throw new Error("Text input clipboard factory is already registered");
	clipboardFactory = factory;
}

export interface TextInputLanguageTypeCommand {
	readonly command: EditorEditCommand;
	readonly insertedText: boolean;
	afterExecute?(change: TextModelChange): void;
}

/** Optional language-aware editing seam implemented by bracket and indentation contributions. */
export interface TextInputLanguageEditingAdapter extends IDisposable {
	readonly textModel: import("../../common/model/textModel.js").TextModel;
	createTypeCommand(selections: TextSelectionSet, text: string): TextInputLanguageTypeCommand | undefined;
	createEnterCommand(selections: TextSelectionSet): EditorEditCommand | undefined;
	createBackspaceCommand(selections: TextSelectionSet): EditorEditCommand | undefined;
}

export type TextInputLanguageEditingFactory = (model: import("../../common/model/textModel.js").TextModel, selections: EditorSelectionController, language: TextInputLanguageOptions, indentation: TextInputIndentationOptions | undefined) => TextInputLanguageEditingAdapter;

let languageEditingFactory: TextInputLanguageEditingFactory | undefined;

/** Registers optional language-aware editing without a browser-to-contrib dependency. */
export function registerTextInputLanguageEditingFactory(factory: TextInputLanguageEditingFactory): void {
	if (typeof factory !== "function") throw new TypeError("Text input language editing factory must be a function");
	if (languageEditingFactory && languageEditingFactory !== factory) throw new Error("Text input language editing factory is already registered");
	languageEditingFactory = factory;
}

export interface TextInputCompletionOptions {
	readonly session: TextInputCompletionSession;
	readonly requests?: TextInputCompletionRequests;
}

/** Structural completion session contract consumed by native text input. */
export interface TextInputCompletionSession {
	readonly textModel: import("../../common/model/textModel.js").TextModel;
	readonly resultStore: VersionedLanguageResultStore<LanguageCompletionResult>;
	readonly state: { readonly isIncomplete: boolean } | undefined;
	acceptSelectedWithCommitCharacter(commitCharacter?: string): boolean;
	cancel(): boolean;
	cancelSnippetPlaceholderNavigation(): boolean;
	selectNextSnippetChoice(): boolean;
	selectPreviousSnippetChoice(): boolean;
	selectNextSnippetPlaceholder(): boolean;
	selectPreviousSnippetPlaceholder(): boolean;
}

export interface TextInputCompletionView extends IDisposable {
	readonly element: HTMLElement;
	readonly visible: boolean;
}

export type TextInputCompletionViewFactory = (element: HTMLTextAreaElement, viewport: EditorViewport, selections: EditorSelectionController, session: TextInputCompletionSession) => TextInputCompletionView;

let completionViewFactory: TextInputCompletionViewFactory | undefined;

/** Registers the optional Suggest presentation without coupling native input to that contrib. */
export function registerTextInputCompletionViewFactory(factory: TextInputCompletionViewFactory): void {
	if (typeof factory !== "function") throw new TypeError("Text input completion view factory must be a function");
	if (completionViewFactory && completionViewFactory !== factory) throw new Error("Text input completion view factory is already registered");
	completionViewFactory = factory;
}

export interface TextInputCompletionRequests {
	readonly service: LanguageCompletionService;
	readonly languageId: string;
	readonly onRequestError?: (error: unknown) => void;
}

/**
 * Owns Aster's hidden textarea and non-composition beforeinput editing.
 */
export class TextInputController extends DisposableOwner {
	readonly element: HTMLTextAreaElement;
	readonly compositionController: CompositionController;
	readonly completionWidget: TextInputCompletionView | undefined;
	private readonly completionSession: TextInputCompletionSession | undefined;
	private readonly completionRequests: TextInputCompletionRequests | undefined;
	private readonly languageEditing: TextInputLanguageEditingAdapter | undefined;
	private readonly wordPattern: (() => RegExp | undefined) | undefined;
	private readonly inputNode: FastDomNode<HTMLTextAreaElement>;
	private readonly commandTransformers: TextInputCommandTransformer[] = [];
	private completionRequest: AbortController | undefined;
	private completionIsIncomplete = false;
	private overtype = false;
	private accessibleInputSyncScheduled = false;
	private accessibleInputStartOffset = 0;
	private disposed = false;

	constructor(
		private readonly viewport: EditorViewport,
		private readonly selectionController: EditorSelectionController,
		options: TextInputControllerOptions = {},
	) {
		super();
		validateIndentationOptions(options.indentation);
		if (
			viewport.textModel !== selectionController.textModel ||
			(
				options.completion &&
				viewport.textModel !== options.completion.session.textModel
			) ||
			(
				options.completion?.requests &&
				(
					viewport.textModel !== options.completion.requests.service.textModel ||
					options.completion.session.resultStore !== options.completion.requests.service.results
				)
			)
		) {
			this.dispose();
			throw new TypeError("Aster text input dependencies must share one text model and completion result store");
		}
		if (
			options.completion?.requests?.onRequestError !== undefined &&
			typeof options.completion.requests.onRequestError !== "function"
		) {
			this.dispose();
			throw new TypeError("Aster completion request error handler must be a function");
		}
		if (options.languageEditing && options.languageEditing.textModel !== viewport.textModel) {
			this.dispose();
			throw new TypeError("Aster text input language editing must share its text model");
		}
		if (options.language && options.completion?.requests && options.completion.requests.languageId !== options.language.languageId) {
			this.dispose();
			throw new TypeError("Aster text input language and completion request identities must match");
		}
		this.completionSession = options.completion?.session;
		this.completionRequests = options.completion?.requests;
		const completionResults = this.completionRequests?.service.results;
		if (completionResults) {
			this.completionIsIncomplete = completionResults.result?.value.isIncomplete === true;
			this.own(completionResults.onDidChange(change => {
				if (change.result) this.completionIsIncomplete = change.result.value.isIncomplete;
			}));
		}
		if (options.language && !languageEditingFactory) {
			this.dispose();
			throw new Error("Text input language options require the language-editing contribution");
		}
		this.languageEditing = options.languageEditing ? this.own(options.languageEditing) : options.language ? this.own(languageEditingFactory!(viewport.textModel, selectionController, options.language, options.indentation)) : undefined;
		this.wordPattern = options.wordPattern ?? (options.language ? () => options.language!.configurations.getLanguageConfiguration(options.language!.languageId).wordPattern : undefined);
		const ownerDocument = viewport.element.ownerDocument;
		this.inputNode = new FastDomNode(h(ownerDocument, "textarea"));
		this.element = this.inputNode.domNode;
		this.inputNode.setClassName("aster-editor-input");
		this.inputNode.setTabIndex(-1);
		this.element.spellcheck = false;
		this.element.readOnly = selectionController.readOnly;
		this.element.wrap = "off";
		this.element.dir = viewport.editorTextDirection;
		this.element.autocomplete = "off";
		this.element.setAttribute("autocapitalize", "off");
		this.element.setAttribute("aria-label", options.ariaLabel ?? "Aster editor input");
		this.element.setAttribute("aria-multiline", "true");
		this.element.setAttribute("aria-roledescription", "code editor");
		this.element.setAttribute("aria-readonly", String(selectionController.readOnly));
		if (this.completionSession && !completionViewFactory) {
			this.dispose();
			throw new Error("Text input completion requires the Suggest contribution");
		}
		this.completionWidget = this.completionSession ? this.own(completionViewFactory!(this.element, viewport, selectionController, this.completionSession)) : undefined;
		this.compositionController = this.own(new CompositionController(
			this.inputNode,
			viewport,
			selectionController,
		));
		if (options.clipboard !== undefined) {
			if (!clipboardFactory) throw new Error("Text input clipboard options require the clipboard contribution");
			this.own(clipboardFactory(this.element, viewport, selectionController, options.clipboard, () => !this.compositionController.composing));
		}
		viewport.element.append(this.element);
		this.defer(() => {
			this.disposed = true;
			this.cancelCompletionRequest();
			viewport.element.classList.remove("input-focused");
			viewport.element.classList.remove("overtype");
			this.element.remove();
		});

		this.own(addDisposableListener(viewport.element, "focus", event => {
			if (event.target === viewport.element) this.focus();
		}));
		this.own(addDisposableListener(this.element, "focus", () => {
			viewport.element.classList.add("input-focused");
			this.synchronizeAccessibleInput();
		}));
		this.own(addDisposableListener(this.element, "blur", () => {
			viewport.element.classList.remove("input-focused");
			this.resetInput();
		}));
		this.own(addDisposableListener<InputEvent>(
			this.element,
			"beforeinput",
			event => this.handleBeforeInput(event),
		));
		this.own(addDisposableListener(this.element, "select", () => this.acceptAccessibleSelection()));
		this.own(addDisposableListener<InputEvent>(
			this.element,
			"input",
			event => {
				if (!event.isComposing || !this.compositionController.composing) this.resetInput();
			},
		));
		this.own(addDisposableListener(
			this.element,
			"keydown",
			event => this.handleKeydown(event),
		));
		this.own(viewport.textModel.onDidChange(() => this.scheduleAccessibleInputSynchronization()));
		this.own(selectionController.onDidChange(() => this.scheduleAccessibleInputSynchronization()));
	}

	focus(): void {
		this.element.focus({ preventScroll: true });
	}

	get overtyping(): boolean {
		return this.overtype;
	}

	registerCommandTransformer(transformer: TextInputCommandTransformer): IDisposable {
		if (typeof transformer !== "function") throw new TypeError("Text input command transformer must be a function");
		this.commandTransformers.push(transformer);
		return toDisposable(() => {
			const index = this.commandTransformers.indexOf(transformer);
			if (index >= 0) this.commandTransformers.splice(index, 1);
		});
	}

	/** Toggles this editor instance's transient overtype input mode. */
	toggleOvertype(): boolean {
		this.overtype = !this.overtype;
		this.viewport.element.classList.toggle("overtype", this.overtype);
		return this.overtype;
	}

	private handleBeforeInput(event: InputEvent): void {
		if (event.defaultPrevented || (event.isComposing && this.compositionController.composing)) return;
		const refreshIncomplete = this.readCompletionIsIncomplete();
		let insertedText: string | undefined;
		let command: EditorEditCommand | undefined;
		let languageTypeCommand: TextInputLanguageTypeCommand | undefined;
		switch (event.inputType) {
			case "insertText":
			case "insertReplacementText":
				if (!event.data) return;
				if (this.completionSession?.acceptSelectedWithCommitCharacter(event.data)) {
					stopEvent(event);
					this.resetInput();
					this.revealPrimary();
					this.requestAfterInsert(event.data, false);
					return;
				}
				{
					languageTypeCommand = this.languageEditing?.createTypeCommand(this.selectionController.selections, event.data);
					insertedText = languageTypeCommand?.insertedText === false ? undefined : event.data;
					command = languageTypeCommand?.command ?? (this.overtype
						? createOvertypeTextCommand(this.viewport.textModel, this.selectionController.selections, event.data)
						: createTypeTextCommand(this.viewport.textModel, this.selectionController.selections, event.data));
				}
				break;
			case "insertLineBreak":
			case "insertParagraph":
				command = this.languageEditing?.createEnterCommand(this.selectionController.selections) ?? createTypeTextCommand(this.viewport.textModel, this.selectionController.selections, "\n");
				break;
			case "deleteContentBackward":
				command = this.languageEditing?.createBackspaceCommand(this.selectionController.selections) ?? createBackspaceCommand(
					this.viewport.textModel,
					this.selectionController.selections,
				);
				break;
			case "deleteContentForward":
				command = createDeleteForwardCommand(
					this.viewport.textModel,
					this.selectionController.selections,
				);
				break;
			case "deleteWordBackward":
				command = createDeleteWordBackwardCommand(
					this.viewport.textModel,
					this.selectionController.selections,
					this.currentWordPattern,
				);
				break;
			case "deleteWordForward":
				command = createDeleteWordForwardCommand(
					this.viewport.textModel,
					this.selectionController.selections,
					this.currentWordPattern,
				);
				break;
			case "deleteSoftLineBackward":
				command = createDeleteToLineStartCommand(
					this.viewport.textModel,
					this.selectionController.selections,
				);
				break;
			case "deleteSoftLineForward":
				command = createDeleteToLineEndCommand(
					this.viewport.textModel,
					this.selectionController.selections,
				);
				break;
			case "historyUndo":
				stopEvent(event);
				this.undo();
				return;
			case "historyRedo":
				stopEvent(event);
				this.redo();
				return;
			default:
				return;
		}
		stopEvent(event);
		this.resetInput();
		for (const transformer of this.commandTransformers) command = transformer(command, { inputType: event.inputType });
		const change = this.execute(command);
		if (change) languageTypeCommand?.afterExecute?.(change);
		if (insertedText !== undefined) {
			this.requestAfterInsert(insertedText, refreshIncomplete);
		} else if (change && refreshIncomplete) {
			this.requestCompletion(createLanguageCompletionIncompleteRefreshContext());
		}
	}

	private handleKeydown(event: KeyboardEvent): void {
		if (!event.defaultPrevented && !event.isComposing && !event.getModifierState("AltGraph")) {
			if (isUndoKeybinding(event)) {
				stopEvent(event);
				this.undo();
				return;
			}
			if (isRedoKeybinding(event)) {
				stopEvent(event);
				this.redo();
				return;
			}
		}
		if (!event.defaultPrevented && !event.isComposing && event.key === "Insert" && !event.shiftKey && !event.ctrlKey && !event.altKey && !event.metaKey) {
			stopEvent(event);
			this.toggleOvertype();
			return;
		}
		if (
			!event.defaultPrevented &&
			!event.isComposing &&
			event.key === " " &&
			event.ctrlKey &&
			!event.shiftKey &&
			!event.altKey &&
			!event.metaKey
		) {
			stopEvent(event);
			this.requestCompletion(createLanguageCompletionInvokeContext());
			return;
		}
		if (
			!event.defaultPrevented &&
			!event.isComposing &&
			event.altKey &&
			!event.shiftKey &&
			!event.ctrlKey &&
			!event.metaKey &&
			(event.key === "ArrowDown" || event.key === "ArrowUp") &&
			(event.key === "ArrowDown"
				? this.completionSession?.selectNextSnippetChoice()
				: this.completionSession?.selectPreviousSnippetChoice())
		) {
			stopEvent(event);
			return;
		}
		if (
			!event.defaultPrevented &&
			!event.isComposing &&
			event.key === "Escape" &&
			!event.shiftKey &&
			!event.ctrlKey &&
			!event.altKey &&
			!event.metaKey &&
			this.completionSession?.cancelSnippetPlaceholderNavigation()
		) {
			stopEvent(event);
			return;
		}
		if (
			!event.defaultPrevented &&
			!event.isComposing &&
			event.key === "Tab" &&
			!event.ctrlKey &&
			!event.altKey &&
			!event.metaKey &&
			(event.shiftKey
				? this.completionSession?.selectPreviousSnippetPlaceholder()
				: this.completionSession?.selectNextSnippetPlaceholder())
		) {
			stopEvent(event);
			return;
		}
		if (
			event.defaultPrevented ||
			event.isComposing ||
			event.key !== "Tab" ||
			event.shiftKey ||
			event.ctrlKey ||
			event.altKey ||
			event.metaKey
		) {
			return;
		}
		if (this.selectionController.selections.selections.some(selection => !selection.collapsed)) return;
		stopEvent(event);
		this.execute(createTypeTextCommand(
			this.viewport.textModel,
			this.selectionController.selections,
			"\t",
		));
	}

	private execute(command: EditorEditCommand): TextModelChange | undefined {
		const change = this.selectionController.execute(command);
		this.revealPrimary();
		return change;
	}

	private undo(): void {
		this.resetInput();
		this.selectionController.undo();
		this.revealPrimary();
	}

	private redo(): void {
		this.resetInput();
		this.selectionController.redo();
		this.revealPrimary();
	}

	private revealPrimary(): void {
		this.viewport.revealPosition(
			this.selectionController.selections.primary.active,
		);
	}

	private resetInput(): void {
		this.element.value = "";
	}

	private synchronizeAccessibleInput(): void {
		if (this.disposed || this.compositionController.composing || this.element.ownerDocument.activeElement !== this.element) return;
		const model = this.viewport.textModel;
		const selection = this.selectionController.selections.primary;
		this.updateAccessibleSelectionDescription();
		const selectionStartOffset = model.offsetAt(selection.range.start);
		const selectionEndOffset = model.offsetAt(selection.range.end);
		const window = accessibleInputWindow(model.length, selectionStartOffset, selectionEndOffset, model.offsetAt(selection.active));
		this.accessibleInputStartOffset = window.startOffset;
		const text = model.getTextInRange(TextRange.from(model.positionAt(window.startOffset), model.positionAt(window.endOffset)));
		if (this.element.value !== text) this.element.value = text;
		this.element.setSelectionRange(
			clampOffset(selectionStartOffset - window.startOffset, text.length),
			clampOffset(selectionEndOffset - window.startOffset, text.length),
			selection.direction === "backward" ? "backward" : "forward",
		);
	}

	private updateAccessibleSelectionDescription(): void {
		const selections = this.selectionController.selections;
		if (selections.selections.length === 1) {
			this.element.removeAttribute("aria-description");
			return;
		}
		const primary = selections.primary.active;
		this.element.setAttribute(
			"aria-description",
			`${selections.selections.length} selections. Primary at line ${primary.lineIndex + 1}, column ${primary.columnIndex + 1}.`,
		);
	}

	private scheduleAccessibleInputSynchronization(): void {
		if (this.accessibleInputSyncScheduled) return;
		this.accessibleInputSyncScheduled = true;
		queueMicrotask(() => {
			this.accessibleInputSyncScheduled = false;
			this.synchronizeAccessibleInput();
		});
	}

	private acceptAccessibleSelection(): void {
		if (this.compositionController.composing || this.element.ownerDocument.activeElement !== this.element) return;
		const model = this.viewport.textModel;
		const startOffset = this.accessibleInputStartOffset + this.element.selectionStart;
		const endOffset = this.accessibleInputStartOffset + this.element.selectionEnd;
		const anchorOffset = this.element.selectionDirection === "backward" ? endOffset : startOffset;
		const activeOffset = this.element.selectionDirection === "backward" ? startOffset : endOffset;
		const current = this.selectionController.selections.primary;
		if (
			model.offsetAt(current.anchor) === anchorOffset &&
			model.offsetAt(current.active) === activeOffset
		) {
			return;
		}
		this.selectionController.setSelections(TextSelectionSet.single(TextSelection.from(
			model.positionAt(anchorOffset),
			model.positionAt(activeOffset),
		)));
		this.viewport.revealPosition(this.selectionController.selections.primary.active);
	}

	private requestAfterInsert(insertedText: string, refreshIncomplete: boolean): void {
		const requests = this.completionRequests;
		if (!requests) return;
		if ([...insertedText].length === 1) {
			const selections = this.selectionController.selections;
			if (selections.selections.length !== 1 || !selections.primary.collapsed) {
				this.completionSession?.cancel();
				return;
			}
			const position = selections.primary.active;
			const modelVersion = this.viewport.textModel.version;
			const request = this.beginCompletionRequest();
			void requests.service.requestTriggerCharacter(
				requests.languageId,
				position,
				insertedText,
				{ signal: request.signal },
			).then(outcome => {
				if (
					!request.signal.aborted &&
					outcome === undefined &&
					refreshIncomplete &&
					this.viewport.textModel.version === modelVersion &&
					this.selectionController.selections.selections.length === 1 &&
					this.selectionController.selections.primary.collapsed &&
					this.selectionController.selections.primary.active.compareTo(position) === 0
				) {
					this.requestCompletion(createLanguageCompletionIncompleteRefreshContext());
				}
			}).catch(error => {
				if (!request.signal.aborted) this.reportCompletionRequestError(error);
			}).finally(() => this.releaseCompletionRequest(request));
			return;
		}
		if (refreshIncomplete) {
			this.requestCompletion(createLanguageCompletionIncompleteRefreshContext());
		}
	}

	private requestCompletion(context: LanguageCompletionContext): void {
		const requests = this.completionRequests;
		if (!requests) return;
		const selections = this.selectionController.selections;
		if (selections.selections.length !== 1 || !selections.primary.collapsed) {
			this.completionSession?.cancel();
			return;
		}
		const request = this.beginCompletionRequest();
		try {
			void requests.service.request(
				requests.languageId,
				selections.primary.active,
				context,
				{ signal: request.signal },
			).catch(error => {
				if (!request.signal.aborted) this.reportCompletionRequestError(error);
			}).finally(() => this.releaseCompletionRequest(request));
		} catch (error) {
			this.releaseCompletionRequest(request);
			if (!request.signal.aborted) this.reportCompletionRequestError(error);
		}
	}

	private beginCompletionRequest(): AbortController {
		this.cancelCompletionRequest();
		const request = new AbortController();
		this.completionRequest = request;
		return request;
	}

	private cancelCompletionRequest(): void {
		this.completionRequest?.abort();
		this.completionRequest = undefined;
	}

	private releaseCompletionRequest(request: AbortController): void {
		if (this.completionRequest === request) this.completionRequest = undefined;
	}

	private readCompletionIsIncomplete(): boolean {
		const result = this.completionRequests?.service.results.result;
		if (result) return result.value.isIncomplete;
		if (this.completionIsIncomplete) return true;
		try {
			return this.completionSession?.state?.isIncomplete === true;
		} catch (error) {
			if (error instanceof ReferenceError) return false;
			throw error;
		}
	}

	private get currentWordPattern(): RegExp | undefined {
		return this.wordPattern?.();
	}

	private reportCompletionRequestError(error: unknown): void {
		try {
			const handler = this.completionRequests?.onRequestError;
			if (handler) handler(error);
			else console.error("Aster completion request failed", error);
		} catch (reportingError) {
			console.error("Aster completion request and error handler both failed", new AggregateError([error, reportingError]));
		}
	}
}

function accessibleInputWindow(modelLength: number, selectionStartOffset: number, selectionEndOffset: number, activeOffset: number): { readonly startOffset: number; readonly endOffset: number } {
	if (modelLength <= MAXIMUM_ACCESSIBLE_INPUT_TEXT_UNITS) return { startOffset: 0, endOffset: modelLength };
	const selectionLength = selectionEndOffset - selectionStartOffset;
	if (selectionLength <= MAXIMUM_ACCESSIBLE_INPUT_TEXT_UNITS) {
		const margin = Math.floor((MAXIMUM_ACCESSIBLE_INPUT_TEXT_UNITS - selectionLength) / 2);
		let startOffset = Math.max(0, selectionStartOffset - margin);
		startOffset = Math.min(startOffset, modelLength - MAXIMUM_ACCESSIBLE_INPUT_TEXT_UNITS);
		if (selectionEndOffset > startOffset + MAXIMUM_ACCESSIBLE_INPUT_TEXT_UNITS) startOffset = selectionEndOffset - MAXIMUM_ACCESSIBLE_INPUT_TEXT_UNITS;
		return { startOffset, endOffset: startOffset + MAXIMUM_ACCESSIBLE_INPUT_TEXT_UNITS };
	}
	const startOffset = Math.min(Math.max(0, activeOffset - Math.floor(MAXIMUM_ACCESSIBLE_INPUT_TEXT_UNITS / 2)), modelLength - MAXIMUM_ACCESSIBLE_INPUT_TEXT_UNITS);
	return { startOffset, endOffset: startOffset + MAXIMUM_ACCESSIBLE_INPUT_TEXT_UNITS };
}

function clampOffset(offset: number, textLength: number): number {
	return Math.min(Math.max(0, offset), textLength);
}

function validateIndentationOptions(options: TextInputIndentationOptions | undefined): void {
	if (options === undefined) return;
	if (typeof options !== "object" || options === null) throw new TypeError("Editor indentation options must be an object");
	if (options.kind !== undefined && options.kind !== "tabs" && options.kind !== "spaces") throw new TypeError("Unknown editor indentation kind");
	if (options.tabSize !== undefined && (!Number.isSafeInteger(options.tabSize) || options.tabSize < 1 || options.tabSize > 32)) throw new RangeError("Editor tab size must be a safe integer between 1 and 32");
}

function isUndoKeybinding(event: Pick<KeyboardEvent, "key" | "ctrlKey" | "shiftKey" | "altKey" | "metaKey">): boolean {
	return hasPrimaryModifier(event) && !event.shiftKey && event.key.toLowerCase() === "z";
}

function isRedoKeybinding(event: Pick<KeyboardEvent, "key" | "ctrlKey" | "shiftKey" | "altKey" | "metaKey">): boolean {
	if (!hasPrimaryModifier(event)) return false;
	const key = event.key.toLowerCase();
	return (key === "z" && event.shiftKey) || (key === "y" && !event.shiftKey);
}

function hasPrimaryModifier(event: Pick<KeyboardEvent, "ctrlKey" | "altKey" | "metaKey">): boolean {
	return !event.altKey && event.ctrlKey !== event.metaKey;
}
