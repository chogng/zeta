import { type IDimension } from "../../../base/browser/geometry.js";
import { isCancellationError } from "../../../base/common/cancellation.js";
import { type Event } from "../../../base/common/event.js";
import { DisposableOwner, type IDisposable } from "../../../base/common/lifecycle.js";
import { type EditorInput } from "../../../workbench/browser/parts/editor/editorInput.js";
import { type TextModelReference } from "../common/services/textModelService.js";
import { CodeEditorWidget } from "./widget/codeEditor/codeEditorWidget.js";
import { type AlphaEditorTextDirection, type AlphaEditorViewport, type AlphaEditorViewportPresentation } from "./view/editorViewport.js";
import { AlphaDecorationPresentation, createAlphaDecorationSource } from "./view/decorationPresentation.js";
import { AlphaCursorUndoController } from "../contrib/cursorUndo/browser/cursorUndoController.js";
import { AlphaDiagnosticNavigationController } from "../contrib/gotoError/browser/gotoError.js";
import { AlphaDiagnosticHoverController } from "../contrib/hover/browser/diagnosticHoverController.js";
import { AlphaHoverController } from "../contrib/hover/browser/hoverController.js";
import { AlphaFormatController } from "../contrib/format/browser/formatController.js";
import { AlphaRenameController } from "../contrib/rename/browser/renameController.js";
import { AlphaCodeActionController } from "../contrib/codeAction/browser/codeActionController.js";
import { AlphaLinksController } from "../contrib/links/browser/linksController.js";
import { AlphaInlayHintsController } from "../contrib/inlayHints/browser/inlayHintsController.js";
import { AlphaInlineCompletionsController } from "../contrib/inlineCompletions/browser/inlineCompletionsController.js";
import { AlphaParameterHintsController } from "../contrib/parameterHints/browser/parameterHintsController.js";
import { AlphaGotoSymbolController } from "../contrib/gotoSymbol/browser/gotoSymbolController.js";
import { AlphaEditorStateController } from "../contrib/editorState/browser/editorStateController.js";
import { EditorStateModel } from "../contrib/editorState/common/editorState.js";
import { AlphaBracketMatchController } from "../contrib/bracketMatching/browser/bracketMatchController.js";
import { AlphaBracketColorizationSource } from "../contrib/bracketMatching/browser/bracketColorizationPresentation.js";
import { AlphaBracketEditingController } from "../contrib/bracketMatching/browser/bracketEditingController.js";
import { AlphaBracketNavigationController } from "../contrib/bracketMatching/browser/bracketNavigationController.js";
import { AlphaBlockCommentController } from "../contrib/comment/browser/blockCommentController.js";
import { AlphaEditingCommandController } from "./editingCommandController.js";
import { AlphaFoldingController } from "../contrib/folding/browser/folding.js";
import { AlphaFindController } from "../contrib/find/browser/findController.js";
import { AlphaGotoLineController } from "../contrib/quickAccess/browser/quickAccessController.js";
import { AlphaLineCommentController } from "../contrib/comment/browser/lineCommentController.js";
import { AlphaLineJoinController } from "../contrib/linesOperations/browser/lineJoinController.js";
import { AlphaLineOperationsController } from "../contrib/linesOperations/browser/lineOperationsController.js";
import { AlphaMultiCursorController } from "../contrib/multicursor/browser/multiCursorController.js";
import { AlphaOccurrenceSelectionController } from "../contrib/multicursor/browser/occurrenceSelectionController.js";
import { AlphaOccurrenceHighlightController } from "../contrib/wordHighlighter/browser/wordHighlighterController.js";
import { createAlphaLanguageDiagnosticSource } from "../contrib/gotoError/browser/languageDiagnosticPresentation.js";
import { AlphaSaveController } from "./saveController.js";
import { createAlphaSemanticTokenSource } from "../contrib/semanticTokens/browser/semanticTokenPresentation.js";
import { type AlphaTextInputController } from "./input/textInputController.js";
import { AlphaTransposeController } from "../contrib/transpose/browser/transposeController.js";
import { AlphaWordWrapController } from "../contrib/wordWrap/browser/wordWrapController.js";
import { AlphaReadOnlyMessageController } from "../contrib/readOnlyMessage/browser/readOnlyMessageController.js";
import { AlphaInsertFinalNewLineController } from "../contrib/insertFinalNewLine/browser/insertFinalNewLineController.js";
import { type AlphaEditorLineWrapping } from "./view/visualLineProjection.js";
import { type LanguageAnalysisService, type LanguageAnalysisWorkerFactory } from "../common/languages/analysis/languageAnalysisService.js";
import { LanguageCompletionSessionController } from "../contrib/suggest/common/suggestModel.js";
import { type LanguageCompletionWorkerFactory } from "../common/languages/completion/languageCompletionService.js";
import { LanguageFeaturesService, type ILanguageFeaturesService } from "../common/services/languageService.js";
import { LanguageBracketMatcher } from "../contrib/bracketMatching/common/bracketMatching.js";
import { LanguageBracketColorizationIndex } from "../contrib/bracketMatching/common/bracketColorization.js";
import { LanguageLexicalContextIndex } from "../common/languages/languageLexicalContext.js";
import { LanguageDiagnosticDecorationBridge } from "../contrib/gotoError/common/diagnosticDecorations.js";
import { LanguageTokenLineIndex } from "../common/tokens/languageTokenLineIndex.js";
import { EditorSelectionController } from "../common/cursor/editorSelectionController.js";
import { EditorFoldingModel } from "../contrib/folding/browser/foldingModel.js";
import { EditorHiddenRangeModel } from "../contrib/folding/browser/hiddenRangeModel.js";
import { computeEditorIndentFoldingRanges } from "../contrib/folding/browser/indentRangeProvider.js";
import { computeEditorLanguageFoldingRanges, mergeEditorFoldingRanges } from "../contrib/folding/browser/syntaxRangeProvider.js";
import { type EditorIndentationOptions } from "../contrib/indentation/common/indentation.js";
import { TextDecorationCollection } from "../common/model/decorationCollection.js";
import { TextSelection, TextSelectionSet } from "../common/core/selection.js";
import { TextPosition } from "../common/core/text.js";
import { TokenizationTextModelPart } from "../contrib/tokenization/common/tokenizationTextModelPart.js";
import { AlphaTokenizationController } from "../contrib/tokenization/browser/tokenizationController.js";
import { AlphaAnchorSelectController } from "../contrib/anchorSelect/browser/anchorSelectController.js";
import { AlphaSmartSelectController } from "../contrib/smartSelect/browser/smartSelectController.js";
import { AlphaInPlaceReplaceController } from "../contrib/inPlaceReplace/browser/inPlaceReplaceController.js";
import { AlphaContextMenuController, type AlphaContextMenuRequest } from "../contrib/contextmenu/browser/contextMenuController.js";
import { AlphaFontZoomController } from "../contrib/fontZoom/browser/fontZoomController.js";
import { AlphaMiddleScrollController } from "../contrib/middleScroll/browser/middleScrollController.js";
import { AlphaPlaceholderTextController } from "../contrib/placeholderText/browser/placeholderTextController.js";
import { AlphaToggleTabFocusModeController } from "../contrib/toggleTabFocusMode/browser/toggleTabFocusModeController.js";
import { AlphaUnicodeHighlighterController } from "../contrib/unicodeHighlighter/browser/unicodeHighlighterController.js";
import { AlphaUnusualLineTerminatorsController } from "../contrib/unusualLineTerminators/browser/unusualLineTerminatorsController.js";
import { type AlphaUnicodeHighlight } from "../contrib/unicodeHighlighter/common/unicodeHighlighter.js";
import { AlphaStickyScrollController } from "../contrib/stickyScroll/browser/stickyScrollController.js";
import { AlphaSectionHeadersController } from "../contrib/sectionHeaders/browser/sectionHeadersController.js";
import { AlphaSymbolIconsController } from "../contrib/symbolIcons/browser/symbolIconsController.js";
import { AlphaMessageController } from "../contrib/message/browser/messageController.js";
import { AlphaInlineProgressController } from "../contrib/inlineProgress/browser/inlineProgressController.js";
import { AlphaColorPickerController } from "../contrib/colorPicker/browser/colorPickerController.js";
import { AlphaLinkedEditingController } from "../contrib/linkedEditing/browser/linkedEditingController.js";
import { AlphaCodeLensController, type AlphaExecuteCodeLensCommand } from "../contrib/codelens/browser/codelensController.js";

export interface AlphaEditorSessionOptions {
  readonly container: HTMLElement;
  readonly input: EditorInput;
  readonly languageId: string;
  /** Optional shared language registrations and providers for this editor host. */
  readonly languageFeaturesService?: ILanguageFeaturesService;
  readonly modelReference: TextModelReference;
  readonly analysisWorkerFactory?: LanguageAnalysisWorkerFactory;
  readonly completionWorkerFactory?: LanguageCompletionWorkerFactory;
  readonly languageSupport?: IDisposable;
  readonly onDidChangeLanguageSupport?: Event<void>;
  readonly whenLanguageSupportReady?: () => Promise<unknown>;
  readonly onLanguageError?: (error: unknown) => void;
  readonly onSaveError?: (error: unknown) => void;
  readonly onSave?: () => Promise<void | boolean>;
  readonly onRevert?: () => Promise<void>;
  readonly indentation?: EditorIndentationOptions;
  readonly lineWrapping?: AlphaEditorLineWrapping;
  /** Applies a single LF at the save boundary when the document has content and no final LF. */
  readonly insertFinalNewLine?: boolean;
  /** Browser paragraph direction for this editor session's DOM projection. */
  readonly textDirection?: AlphaEditorTextDirection;
  readonly presentation?: AlphaEditorViewportPresentation;
  /** Host-owned link opening callback; Alpha never opens external targets directly. */
  readonly onOpenLink?: (target: string) => void | Promise<void>;
  /** Host-owned context-menu composition; Alpha supplies only editor hit-test data. */
  readonly onShowContextMenu?: (request: AlphaContextMenuRequest) => void | Promise<void>;
  /** Host-owned execution for provider commands such as code lenses. */
  readonly onExecuteEditorCommand?: AlphaExecuteCodeLensCommand;
  readonly placeholder?: string;
  readonly showUnicodeHighlights?: boolean;
  readonly fontZoom?: { readonly initialScale?: number };
}

/** Owns all per-pane state projected over one shared Alpha text model reference. */
export class AlphaEditorSession extends DisposableOwner {
  readonly onDidChange: Event<void>;
  readonly codeEditor: CodeEditorWidget;
  readonly viewport: AlphaEditorViewport;
  readonly selections: EditorSelectionController;
  readonly textInput: AlphaTextInputController;
  readonly find: AlphaFindController;
  private readonly analysis: LanguageAnalysisService;
  private readonly languageId: string;
  private readonly whenLanguageSupportReady: () => Promise<unknown>;
  private readonly onLanguageError: (error: unknown) => void;
  private readonly onSaveError: (error: unknown) => void;
  private readonly modelReference: TextModelReference;
  private readonly onSave: (() => Promise<void | boolean>) | undefined;
  private readonly onRevert: (() => Promise<void>) | undefined;
  private readonly beforeSave: (() => void) | undefined;
  private analysisGeneration = 0;
  private disposed = false;

  constructor(options: AlphaEditorSessionOptions) {
    super();
    try {
      validateOptions(options);
      this.languageId = options.languageId;
      this.whenLanguageSupportReady = options.whenLanguageSupportReady ?? (() => Promise.resolve());
      this.onLanguageError = options.onLanguageError ?? reportLanguageError;
      this.onSaveError = options.onSaveError ?? reportSaveError;
      this.onSave = options.onSave;
      this.onRevert = options.onRevert;
      if (options.languageSupport) this.own(options.languageSupport);
      const modelReference = this.modelReference = this.own(options.modelReference);
      const model = modelReference.model;
      this.onDidChange = listener => model.onDidChange(() => listener());
      const languageFeaturesService = options.languageFeaturesService ?? this.own(new LanguageFeaturesService());
      const configurations = languageFeaturesService.configurations;
      this.selections = this.own(new EditorSelectionController(
        model,
        TextSelectionSet.single(TextSelection.collapsedAt(TextPosition.at(0, 0))),
        { readOnly: options.input.readOnly },
      ));
      const finalNewLine = options.insertFinalNewLine ? this.own(new AlphaInsertFinalNewLineController(this.selections)) : undefined;
      this.beforeSave = finalNewLine ? () => finalNewLine.prepareSave() : undefined;
      const folding = this.own(new EditorFoldingModel(model));
      const hiddenRanges = this.own(new EditorHiddenRangeModel(model, folding));
      const updateFolding = () => folding.setProviderRanges(mergeEditorFoldingRanges(
        computeEditorLanguageFoldingRanges(model, this.languageId, configurations),
        computeEditorIndentFoldingRanges(model),
      ));
      updateFolding();
      this.own(model.onDidChange(updateFolding));

      this.analysis = this.own(languageFeaturesService.createAnalysisService(model, {
        ...(options.analysisWorkerFactory ? { workerFactory: options.analysisWorkerFactory } : {}),
      }));
      const tokenLines = new LanguageTokenLineIndex(this.analysis.tokens);
      const tokenization = this.own(new TokenizationTextModelPart(tokenLines));
      const diagnostics = this.own(new LanguageDiagnosticDecorationBridge(this.analysis.diagnostics));
      const searchDecorations = this.own(new TextDecorationCollection<void>(model));
      const occurrenceDecorations = this.own(new TextDecorationCollection<void>(model));
      const bracketDecorations = this.own(new TextDecorationCollection<void>(model));
      const unicodeDecorations = this.own(new TextDecorationCollection<AlphaUnicodeHighlight>(model));
      const unusualLineTerminatorDecorations = this.own(new TextDecorationCollection<void>(model));
      if (options.showUnicodeHighlights !== false) this.own(new AlphaUnicodeHighlighterController(model, unicodeDecorations));
      this.own(new AlphaUnusualLineTerminatorsController(model, unusualLineTerminatorDecorations));
      const bracketMatcher = this.own(new LanguageBracketMatcher(model, this.languageId, configurations));
      const lexicalContext = this.own(new LanguageLexicalContextIndex(model, this.languageId, configurations));
      const bracketColorizations = this.own(new LanguageBracketColorizationIndex(model, lexicalContext));

      const completions = this.own(languageFeaturesService.createCompletionService(model, {
        ...(options.completionWorkerFactory ? { workerFactory: options.completionWorkerFactory } : {}),
      }));
      const hover = this.own(languageFeaturesService.createHoverService(model));
      const formatting = this.own(languageFeaturesService.createFormatService(model));
      const rename = this.own(languageFeaturesService.createRenameService(model));
      const codeActions = this.own(languageFeaturesService.createCodeActionService(model));
      const links = this.own(languageFeaturesService.createLinkService(model));
      const inlayHints = this.own(languageFeaturesService.createInlayHintsService(model));
      const inlineCompletions = this.own(languageFeaturesService.createInlineCompletionsService(model));
      const parameterHints = this.own(languageFeaturesService.createParameterHintsService(model));
      const gotoSymbol = this.own(languageFeaturesService.createGotoSymbolService(model));
      const documentSymbols = this.own(languageFeaturesService.createDocumentSymbolService(model));
      const codeLenses = this.own(languageFeaturesService.createCodeLensService(model));
      const colors = this.own(languageFeaturesService.createColorService(model));
      const linkedEditing = this.own(languageFeaturesService.createLinkedEditingService(model));
      const editorState = this.own(new EditorStateModel(model, this.selections.selections));
      const completionSession = this.own(new LanguageCompletionSessionController(
        completions.results,
        this.selections,
        {
          resolver: completions,
          onResolveError: this.onLanguageError,
          snippetVariables: createAlphaSnippetVariables(options.input),
        },
      ));
      const semanticTokens = createAlphaSemanticTokenSource(tokenization);
      const ariaLabel = editorLabel(options.input);

      this.codeEditor = this.own(new CodeEditorWidget({
        container: options.container,
        model,
        lineHeight: 20,
        selectionController: this.selections,
        ariaLabel,
        viewport: {
          foldingModel: folding,
          hiddenRangeModel: hiddenRanges,
          decorationSources: [
            createAlphaLanguageDiagnosticSource(diagnostics.decorations),
            createAlphaDecorationSource(searchDecorations, () => AlphaDecorationPresentation.SearchMatch),
            createAlphaDecorationSource(occurrenceDecorations, () => AlphaDecorationPresentation.OccurrenceHighlight),
            createAlphaDecorationSource(bracketDecorations, () => AlphaDecorationPresentation.BracketMatch),
            createAlphaDecorationSource(unicodeDecorations, () => AlphaDecorationPresentation.UnicodeHighlight, decoration => `${decoration.metadata.kind} Unicode character U+${decoration.metadata.character.codePointAt(0)!.toString(16).toUpperCase()}`),
            createAlphaDecorationSource(unusualLineTerminatorDecorations, () => AlphaDecorationPresentation.UnusualLineTerminator, () => "Unusual line terminator"),
          ],
          semanticTokenSource: semanticTokens,
          bracketColorizationSource: new AlphaBracketColorizationSource(bracketColorizations),
          lineWrapping: options.lineWrapping,
          textDirection: options.textDirection,
          presentation: options.presentation,
          indentation: options.indentation,
        },
        textInput: {
          clipboard: { semanticTokens },
          language: {
            languageId: this.languageId,
            configurations,
            lexicalContext,
          },
          completion: {
            session: completionSession,
            requests: {
              service: completions,
              languageId: this.languageId,
              onRequestError: this.onLanguageError,
            },
          },
          indentation: options.indentation,
        },
        keyboardNavigation: {
          wordPattern: () => configurations.getLanguageConfiguration(this.languageId).wordPattern,
        },
        pointerSelection: {
          wordPattern: () => configurations.getLanguageConfiguration(this.languageId).wordPattern,
        },
      }));
      this.viewport = this.codeEditor.viewport;
      this.textInput = this.codeEditor.textInput;
      this.own(new AlphaTokenizationController(this.viewport, tokenization));
      this.own(new AlphaEditorStateController(this.textInput.element, this.viewport, this.selections, editorState));
      if (options.input.readOnly) this.own(new AlphaReadOnlyMessageController(this.textInput.element, this.viewport));
      this.own(modelReference.onDidChangeExternalChange(() => {
        if (modelReference.hasExternalChange) {
          this.viewport.announceAccessibilityStatus("File changed on disk. Local edits are preserved.");
        }
      }));
      if (this.onSave) this.own(new AlphaSaveController(this.textInput.element, {
        save: this.onSave,
        beforeSave: this.beforeSave,
        onSaveSuccess: () => this.viewport.announceAccessibilityStatus("Saved"),
        onSaveError: error => {
          this.viewport.announceAccessibilityStatus(`Save failed: ${saveErrorMessage(error)}`);
          this.onSaveError(error);
        },
      }));
      this.own(new AlphaDiagnosticNavigationController(this.textInput.element, this.viewport, this.selections, diagnostics.decorations));
      this.own(new AlphaDiagnosticHoverController(this.viewport));
      this.own(new AlphaHoverController(this.viewport, hover, this.languageId));
      this.own(new AlphaFormatController(this.textInput.element, this.viewport, this.selections, formatting, this.languageId, {
        formattingOptions: { tabSize: options.indentation?.tabSize ?? 4, insertSpaces: options.indentation?.kind !== "tabs" },
        onError: this.onLanguageError,
      }));
      this.own(new AlphaRenameController(this.textInput.element, this.viewport, this.selections, rename, this.languageId, this.onLanguageError));
      this.own(new AlphaCodeActionController(this.textInput.element, this.viewport, this.selections, codeActions, diagnostics.decorations, this.languageId, this.onLanguageError));
      if (options.onOpenLink) this.own(new AlphaLinksController(this.viewport, links, this.languageId, options.onOpenLink, this.onLanguageError));
      this.own(new AlphaInlayHintsController(this.viewport, inlayHints, this.languageId, this.onLanguageError));
      this.own(new AlphaInlineCompletionsController(this.textInput.element, this.viewport, this.selections, inlineCompletions, this.languageId, this.onLanguageError));
      this.own(new AlphaParameterHintsController(this.textInput.element, this.viewport, this.selections, parameterHints, this.languageId, this.onLanguageError));
      this.own(new AlphaGotoSymbolController(this.textInput.element, this.viewport, this.selections, gotoSymbol, this.languageId, this.onLanguageError));
      this.own(new AlphaAnchorSelectController(this.textInput.element, this.viewport, this.selections, () => configurations.getLanguageConfiguration(this.languageId).wordPattern));
      this.own(new AlphaSmartSelectController(this.textInput.element, this.viewport, this.selections, () => configurations.getLanguageConfiguration(this.languageId).wordPattern));
      this.own(new AlphaInPlaceReplaceController(this.textInput.element, this.viewport, this.selections));
      this.own(new AlphaFontZoomController(this.textInput.element, this.viewport, { baseLineHeight: 20, initialScale: options.fontZoom?.initialScale }));
      this.own(new AlphaMiddleScrollController(this.viewport));
      this.own(new AlphaToggleTabFocusModeController(this.textInput.element, this.viewport));
      if (options.placeholder) this.own(new AlphaPlaceholderTextController(this.viewport, options.placeholder));
      if (options.onShowContextMenu) this.own(new AlphaContextMenuController(this.viewport, options.onShowContextMenu, this.onLanguageError));
      this.own(new AlphaStickyScrollController(this.viewport, folding));
      this.own(new AlphaSectionHeadersController(this.viewport, folding));
      this.own(new AlphaSymbolIconsController(this.viewport, documentSymbols, this.languageId, this.onLanguageError));
      this.own(new AlphaCodeLensController(this.viewport, codeLenses, this.languageId, options.onExecuteEditorCommand, this.onLanguageError));
      this.own(new AlphaColorPickerController(this.textInput.element, this.viewport, this.selections, colors, this.languageId, this.onLanguageError));
      this.own(new AlphaLinkedEditingController(this.textInput.element, this.viewport, this.selections, linkedEditing, this.languageId, this.onLanguageError));
      this.own(new AlphaMessageController(this.viewport));
      this.own(new AlphaInlineProgressController(this.viewport));
      this.find = this.own(new AlphaFindController(this.textInput.element, this.viewport, this.selections, searchDecorations));
      this.own(new AlphaGotoLineController(this.textInput.element, this.viewport, this.selections));
      this.own(new AlphaBracketMatchController(this.selections, bracketMatcher, bracketDecorations));
      this.own(new AlphaBracketNavigationController(this.textInput.element, this.viewport, this.selections, bracketMatcher));
      this.own(new AlphaBracketEditingController(this.textInput.element, this.viewport, this.selections, bracketMatcher));
      this.own(new AlphaEditingCommandController(this.textInput.element, this.viewport, this.selections));
      this.own(new AlphaFoldingController(this.textInput.element, this.viewport, this.selections, folding));
      this.own(new AlphaLineCommentController(this.textInput.element, this.viewport, this.selections, {
        languageId: this.languageId,
        configurations,
      }));
      this.own(new AlphaBlockCommentController(this.textInput.element, this.viewport, this.selections, {
        languageId: this.languageId,
        configurations,
      }));
      this.own(new AlphaLineOperationsController(this.textInput.element, this.viewport, this.selections, { indentation: options.indentation }));
      this.own(new AlphaLineJoinController(this.textInput.element, this.viewport, this.selections));
      this.own(new AlphaTransposeController(this.textInput.element, this.viewport, this.selections));
      this.own(new AlphaWordWrapController(this.textInput.element, this.viewport));
      this.own(new AlphaMultiCursorController(this.textInput.element, this.viewport, this.selections));
      this.own(new AlphaCursorUndoController(this.textInput.element, this.viewport, this.selections));
      this.own(new AlphaOccurrenceSelectionController(this.textInput.element, this.viewport, this.selections, {
        wordPattern: () => configurations.getLanguageConfiguration(this.languageId).wordPattern,
      }));
      this.own(new AlphaOccurrenceHighlightController(this.selections, occurrenceDecorations, {
        wordPattern: () => configurations.getLanguageConfiguration(this.languageId).wordPattern,
      }));
      this.own(model.onDidChange(() => this.scheduleAnalysis()));
      if (options.onDidChangeLanguageSupport) {
        this.own(options.onDidChangeLanguageSupport(() => this.scheduleAnalysis()));
      }
      this.defer(() => {
        this.disposed = true;
        this.analysisGeneration += 1;
      });
      this.scheduleAnalysis();
    } catch (error) {
      this.dispose();
      throw error;
    }
  }

  layout(dimension: IDimension): void {
    this.codeEditor.layout(dimension);
  }

  focus(): void {
    this.codeEditor.focus();
  }

  getValue(): string {
    return this.viewport.textModel.getText();
  }

  setValue(value: string): void {
    if (this.getValue() === value) return;
    this.modelReference.model.reset(value);
  }

  get isDirty(): boolean {
    return this.modelReference.isDirty;
  }

  get hasExternalChange(): boolean {
    return this.modelReference.hasExternalChange;
  }

  async save(): Promise<void> {
    this.beforeSave?.();
    await this.onSave?.();
  }

  async revert(): Promise<void> {
    await this.onRevert?.();
  }

  private scheduleAnalysis(): void {
    const generation = ++this.analysisGeneration;
    queueMicrotask(() => {
      void this.runAnalysis(generation);
    });
  }

  private async runAnalysis(generation: number): Promise<void> {
    try {
      await this.whenLanguageSupportReady();
      if (this.disposed || generation !== this.analysisGeneration) return;
      await this.analysis.requestAll(this.languageId);
    } catch (error) {
      if (this.disposed || generation !== this.analysisGeneration || isCancellationError(error) || isAbortError(error)) return;
      this.onLanguageError(error);
    }
  }
}

function validateOptions(options: AlphaEditorSessionOptions): void {
  if (!options || typeof options !== "object" || !options.container || !options.modelReference) {
    throw new TypeError("Alpha editor session requires a container and model reference");
  }
  if (options.input?.readOnly !== undefined && typeof options.input.readOnly !== "boolean") {
    throw new TypeError("Alpha editor input read-only mode must be boolean");
  }
  if (options.whenLanguageSupportReady !== undefined && typeof options.whenLanguageSupportReady !== "function") {
    throw new TypeError("Alpha language readiness must be a function");
  }
  if (options.onLanguageError !== undefined && typeof options.onLanguageError !== "function") {
    throw new TypeError("Alpha language error handler must be a function");
  }
  if (options.onSaveError !== undefined && typeof options.onSaveError !== "function") {
    throw new TypeError("Alpha save error handler must be a function");
  }
  if (options.onSave !== undefined && typeof options.onSave !== "function") {
    throw new TypeError("Alpha editor save must be a function");
  }
  if (options.onRevert !== undefined && typeof options.onRevert !== "function") {
    throw new TypeError("Alpha editor revert must be a function");
  }
  if (options.insertFinalNewLine !== undefined && typeof options.insertFinalNewLine !== "boolean") {
    throw new TypeError("Alpha final newline option must be boolean");
  }
}

function editorLabel(input: EditorInput): string {
  if (input.label?.trim()) return input.label;
  const path = decodeURIComponent(input.resource.path);
  return path.slice(path.lastIndexOf("/") + 1) || "Alpha editor";
}

function createAlphaSnippetVariables(input: EditorInput): { readonly resolveVariable: (name: string) => string | undefined } {
  const filePath = decodeURIComponent(input.resource.path);
  const separator = filePath.lastIndexOf("/");
  const filename = filePath.slice(separator + 1);
  const extension = filename.lastIndexOf(".");
  const filenameBase = extension > 0 ? filename.slice(0, extension) : filename;
  const directory = separator > 0 ? filePath.slice(0, separator) : "/";
  return Object.freeze({
    resolveVariable(name: string): string | undefined {
      switch (name) {
        case "TM_FILENAME": return filename;
        case "TM_FILENAME_BASE": return filenameBase;
        case "TM_DIRECTORY": return directory;
        case "TM_FILEPATH": return filePath;
        default: return undefined;
      }
    },
  });
}

function isAbortError(error: unknown): boolean {
  return error instanceof Error && error.name === "AbortError";
}

function reportLanguageError(error: unknown): void {
  console.error("Alpha language request failed", error);
}

function reportSaveError(error: unknown): void {
  console.error("Alpha editor save failed", error);
}

function saveErrorMessage(error: unknown): string {
  return error instanceof Error && error.message.trim().length > 0
    ? error.message.trim()
    : "unknown error";
}
