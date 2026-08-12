import { addDisposableListener, stopEvent } from "../../../base/browser/dom.js";
import { DisposableOwner } from "../../../base/common/lifecycle.js";
import { createBackspaceCommand, createDeleteForwardCommand, createDeleteToLineEndCommand, createDeleteToLineStartCommand } from "../../common/cursor/cursorDeleteOperations.js";
import { createDeleteWordBackwardCommand, createDeleteWordForwardCommand } from "../../common/cursor/cursorWordOperations.js";
import { createTypeTextCommand } from "../../common/cursor/cursorTypeOperations.js";
import { resolveEditorIndentationOptions, type EditorIndentationOptions, type ResolvedEditorIndentationOptions } from "../../contrib/indentation/common/indentation.js";
import { type EditorEditCommand } from "../../common/commands/editorEditCommand.js";
import { type EditorSelectionController } from "../../common/cursor/editorSelectionController.js";
import { LanguageAutoClosingTracker } from "../../contrib/bracketMatching/common/autoClosingTracker.js";
import { type LanguageConfigurationSource, type ResolvedLanguageConfiguration } from "../../common/languages/languageConfiguration.js";
import { type LanguageCompletionSessionController } from "../../contrib/suggest/common/suggestModel.js";
import { type LanguageCompletionService } from "../../common/languages/completion/languageCompletionService.js";
import { createLanguageCompletionIncompleteRefreshContext, createLanguageCompletionInvokeContext, type LanguageCompletionContext } from "../../common/languages/completion/languageCompletionProviders.js";
import { assertLanguageId } from "../../common/languages/languageId.js";
import { createLanguageEnterCommand } from "../../contrib/bracketMatching/common/enter.js";
import { LanguageLexicalContextIndex, type LanguageLexicalContextSource } from "../../common/languages/languageLexicalContext.js";
import { createLanguagePairBackspaceCommand, createLanguagePairTypeCommand, type LanguagePairTypeCommand } from "../../contrib/bracketMatching/common/pairEditing.js";
import { createOvertypeTextCommand } from "../../common/cursor/cursorOvertype.js";
import { type TextModelChange } from "../../common/core/text.js";
import { TextSelection, TextSelectionSet } from "../../common/core/selection.js";
import { type EditorViewport } from "../view/editorViewport.js";
import { ClipboardController, type ClipboardControllerOptions } from "../../contrib/clipboard/browser/clipboardController.js";
import { UriListPasteProvider } from "../../contrib/clipboard/browser/clipboardPasteProvider.js";
import { CompletionWidget } from "../../contrib/suggest/browser/suggestWidget.js";
import { CompositionController } from "./compositionController.js";

export interface TextInputControllerOptions {
  readonly ariaLabel?: string;
  readonly clipboard?: ClipboardControllerOptions;
  readonly completion?: TextInputCompletionOptions;
  readonly indentation?: EditorIndentationOptions;
  readonly language?: TextInputLanguageOptions;
}

export interface TextInputLanguageOptions {
  readonly languageId: string;
  readonly configurations: LanguageConfigurationSource;
  readonly lexicalContext?: LanguageLexicalContextSource;
}

export interface TextInputCompletionOptions {
  readonly session: LanguageCompletionSessionController;
  readonly requests?: TextInputCompletionRequests;
}

export interface TextInputCompletionRequests {
  readonly service: LanguageCompletionService;
  readonly languageId: string;
  readonly onRequestError?: (error: unknown) => void;
}

/**
 * Owns Alpha's hidden textarea and non-composition beforeinput editing.
 */
export class TextInputController extends DisposableOwner {
  readonly element: HTMLTextAreaElement;
  readonly compositionController: CompositionController;
  readonly completionWidget: CompletionWidget | undefined;
  private readonly completionSession: LanguageCompletionSessionController | undefined;
  private readonly completionRequests: TextInputCompletionRequests | undefined;
  private readonly language: TextInputLanguageOptions | undefined;
  private readonly indentation: ResolvedEditorIndentationOptions;
  private readonly languageLexicalContext: LanguageLexicalContextSource | undefined;
  private readonly autoClosingTracker: LanguageAutoClosingTracker | undefined;
  private completionRequest: AbortController | undefined;
  private completionIsIncomplete = false;
  private overtype = false;
  private accessibleInputSyncScheduled = false;
  private disposed = false;

  constructor(
    private readonly viewport: EditorViewport,
    private readonly selectionController: EditorSelectionController,
    options: TextInputControllerOptions = {},
  ) {
    super();
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
      throw new TypeError("Alpha text input dependencies must share one text model and completion result store");
    }
    if (
      options.completion?.requests?.onRequestError !== undefined &&
      typeof options.completion.requests.onRequestError !== "function"
    ) {
      this.dispose();
      throw new TypeError("Alpha completion request error handler must be a function");
    }
    if (options.language) {
      assertLanguageId(options.language.languageId);
      if (!options.language.configurations || typeof options.language.configurations.getLanguageConfiguration !== "function") {
        this.dispose();
        throw new TypeError("Alpha text input language requires a configuration source");
      }
      if (options.completion?.requests && options.completion.requests.languageId !== options.language.languageId) {
        this.dispose();
        throw new TypeError("Alpha text input language and completion request identities must match");
      }
      if (options.language.lexicalContext && (
        options.language.lexicalContext.textModel !== viewport.textModel ||
        options.language.lexicalContext.languageId !== options.language.languageId ||
        typeof options.language.lexicalContext.getStructuralLineContent !== "function" ||
        typeof options.language.lexicalContext.getTokenTypeAt !== "function"
      )) {
        this.dispose();
        throw new TypeError("Alpha text input lexical context must match its model and language");
      }
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
    this.language = options.language;
    try {
      this.indentation = resolveEditorIndentationOptions(options.indentation);
    } catch (error) {
      this.dispose();
      throw error;
    }
    this.languageLexicalContext = options.language
      ? options.language.lexicalContext ?? this.own(new LanguageLexicalContextIndex(viewport.textModel, options.language.languageId, options.language.configurations))
      : undefined;
    this.autoClosingTracker = options.language
      ? this.own(new LanguageAutoClosingTracker(viewport.textModel, selectionController))
      : undefined;
    const ownerDocument = viewport.element.ownerDocument;
    this.element = ownerDocument.createElement("textarea");
    this.element.className = "zeta-alpha-editor-input";
    this.element.tabIndex = -1;
    this.element.spellcheck = false;
    this.element.readOnly = selectionController.readOnly;
    this.element.wrap = "off";
    this.element.dir = viewport.editorTextDirection;
    this.element.autocomplete = "off";
    this.element.setAttribute("autocapitalize", "off");
    this.element.setAttribute("aria-label", options.ariaLabel ?? "Alpha editor input");
    this.element.setAttribute("aria-multiline", "true");
    this.element.setAttribute("aria-roledescription", "code editor");
    this.element.setAttribute("aria-readonly", String(selectionController.readOnly));
    this.completionWidget = this.completionSession
      ? this.own(new CompletionWidget(
        this.element,
        viewport,
        selectionController,
        this.completionSession,
      ))
      : undefined;
    this.compositionController = this.own(new CompositionController(
      this.element,
      viewport,
      selectionController,
    ));
    this.own(new ClipboardController(
      this.element,
      viewport,
      selectionController,
      {
        ...options.clipboard,
        isEditingAllowed: () => !this.compositionController.composing && (options.clipboard?.isEditingAllowed?.() ?? true),
        pasteProviders: [UriListPasteProvider, ...(options.clipboard?.pasteProviders ?? [])],
      },
    ));
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
    let pairCommand: LanguagePairTypeCommand | undefined;
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
          pairCommand = this.language
            ? createLanguagePairTypeCommand(this.viewport.textModel, this.selectionController.selections, event.data, this.readLanguageConfiguration(), {
              autoClosingTrust: this.autoClosingTracker,
              lexicalContext: this.languageLexicalContext,
            })
            : undefined;
          insertedText = pairCommand?.didInsertText === false ? undefined : event.data;
          command = pairCommand?.command ?? (this.overtype
            ? createOvertypeTextCommand(this.viewport.textModel, this.selectionController.selections, event.data)
            : createTypeTextCommand(this.viewport.textModel, this.selectionController.selections, event.data));
        }
        break;
      case "insertLineBreak":
      case "insertParagraph":
        command = this.language
          ? createLanguageEnterCommand(this.viewport.textModel, this.selectionController.selections, this.readLanguageConfiguration(), {
            indentation: this.indentation,
            lexicalContext: this.languageLexicalContext,
          })
          : createTypeTextCommand(this.viewport.textModel, this.selectionController.selections, "\n");
        break;
      case "deleteContentBackward":
        command = (this.language
          ? createLanguagePairBackspaceCommand(this.viewport.textModel, this.selectionController.selections, this.readLanguageConfiguration(), this.autoClosingTracker)
          : undefined) ?? createBackspaceCommand(
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
    const change = this.execute(command);
    if (change && pairCommand?.autoClosingActions.length) {
      this.autoClosingTracker?.record(pairCommand.autoClosingActions, change.version);
    }
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
    const text = model.getText();
    if (this.element.value !== text) this.element.value = text;
    const start = model.offsetAt(selection.range.start);
    const end = model.offsetAt(selection.range.end);
    this.element.setSelectionRange(
      start,
      end,
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
    const startOffset = this.element.selectionStart;
    const endOffset = this.element.selectionEnd;
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

  private readLanguageConfiguration(): ResolvedLanguageConfiguration {
    return this.language!.configurations.getLanguageConfiguration(this.language!.languageId);
  }

  private get currentWordPattern(): RegExp | undefined {
    return this.language ? this.readLanguageConfiguration().wordPattern : undefined;
  }

  private reportCompletionRequestError(error: unknown): void {
    try {
      const handler = this.completionRequests?.onRequestError;
      if (handler) handler(error);
      else console.error("Alpha completion request failed", error);
    } catch (reportingError) {
      console.error("Alpha completion request and error handler both failed", new AggregateError([error, reportingError]));
    }
  }
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
