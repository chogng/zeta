import { type IDimension } from "../../../base/browser/geometry.js";
import { isCancellationError } from "../../../base/common/cancellation.js";
import { type Event } from "../../../base/common/event.js";
import { DisposableOwner, type IDisposable } from "../../../base/common/lifecycle.js";
import { type ISyntaxApi } from "../../../platform/syntax/common/syntaxApi.js";
import { type EditorInput } from "../../../workbench/browser/parts/editor/editorInput.js";
import { type TextModelReference } from "../common/services/textModelService.js";
import { CodeEditorWidget } from "./widget/codeEditor/codeEditorWidget.js";
import { type EditorTextDirection, type EditorViewport, type EditorViewportPresentation } from "./view/editorViewport.js";
import { DecorationPresentation, createAlphaDecorationSource } from "./view/decorationPresentation.js";
import { CursorUndoController } from "../contrib/cursorUndo/browser/cursorUndoController.js";
import { DiagnosticNavigationController } from "../contrib/gotoError/browser/gotoError.js";
import { DiagnosticHoverController } from "../contrib/hover/browser/diagnosticHoverController.js";
import { HoverController } from "../contrib/hover/browser/hoverController.js";
import { FormatController } from "../contrib/format/browser/formatController.js";
import { RenameController } from "../contrib/rename/browser/renameController.js";
import { CodeActionController } from "../contrib/codeAction/browser/codeActionController.js";
import { LinksController } from "../contrib/links/browser/linksController.js";
import { InlayHintsController } from "../contrib/inlayHints/browser/inlayHintsController.js";
import { InlineCompletionsController } from "../contrib/inlineCompletions/browser/inlineCompletionsController.js";
import { ParameterHintsController } from "../contrib/parameterHints/browser/parameterHintsController.js";
import { GotoSymbolController } from "../contrib/gotoSymbol/browser/gotoSymbolController.js";
import { EditorStateController } from "../contrib/editorState/browser/editorStateController.js";
import { EditorStateModel } from "../contrib/editorState/common/editorState.js";
import { BracketMatchController } from "../contrib/bracketMatching/browser/bracketMatchController.js";
import { BracketColorizationSource } from "../contrib/bracketMatching/browser/bracketColorizationPresentation.js";
import { BracketEditingController } from "../contrib/bracketMatching/browser/bracketEditingController.js";
import { BracketNavigationController } from "../contrib/bracketMatching/browser/bracketNavigationController.js";
import { BlockCommentController } from "../contrib/comment/browser/blockCommentController.js";
import { EditingCommandController } from "./editingCommandController.js";
import { FoldingController } from "../contrib/folding/browser/folding.js";
import { FindController } from "../contrib/find/browser/findController.js";
import { GotoLineController } from "../contrib/quickAccess/browser/quickAccessController.js";
import { LineCommentController } from "../contrib/comment/browser/lineCommentController.js";
import { LineJoinController } from "../contrib/linesOperations/browser/lineJoinController.js";
import { LineOperationsController } from "../contrib/linesOperations/browser/lineOperationsController.js";
import { MultiCursorController } from "../contrib/multicursor/browser/multiCursorController.js";
import { OccurrenceSelectionController } from "../contrib/multicursor/browser/occurrenceSelectionController.js";
import { OccurrenceHighlightController } from "../contrib/wordHighlighter/browser/wordHighlighterController.js";
import { createAlphaLanguageDiagnosticSource } from "../contrib/gotoError/browser/languageDiagnosticPresentation.js";
import { SaveController } from "./saveController.js";
import { createAlphaSemanticTokenSource } from "../contrib/semanticTokens/browser/semanticTokenPresentation.js";
import { type TextInputController } from "./input/textInputController.js";
import { TransposeController } from "../contrib/transpose/browser/transposeController.js";
import { WordWrapController } from "../contrib/wordWrap/browser/wordWrapController.js";
import { ReadOnlyMessageController } from "../contrib/readOnlyMessage/browser/readOnlyMessageController.js";
import { InsertFinalNewLineController } from "../contrib/insertFinalNewLine/browser/insertFinalNewLineController.js";
import { type EditorLineWrapping } from "./view/visualLineProjection.js";
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
import { TokenizationController } from "../contrib/tokenization/browser/tokenizationController.js";
import { AnchorSelectController } from "../contrib/anchorSelect/browser/anchorSelectController.js";
import { SmartSelectController } from "../contrib/smartSelect/browser/smartSelectController.js";
import { InPlaceReplaceController } from "../contrib/inPlaceReplace/browser/inPlaceReplaceController.js";
import { ContextMenuController, type ContextMenuRequest } from "../contrib/contextmenu/browser/contextMenuController.js";
import { FontZoomController } from "../contrib/fontZoom/browser/fontZoomController.js";
import { MiddleScrollController } from "../contrib/middleScroll/browser/middleScrollController.js";
import { PlaceholderTextController } from "../contrib/placeholderText/browser/placeholderTextController.js";
import { ToggleTabFocusModeController } from "../contrib/toggleTabFocusMode/browser/toggleTabFocusModeController.js";
import { UnicodeHighlighterController } from "../contrib/unicodeHighlighter/browser/unicodeHighlighterController.js";
import { UnusualLineTerminatorsController } from "../contrib/unusualLineTerminators/browser/unusualLineTerminatorsController.js";
import { type UnicodeHighlight } from "../contrib/unicodeHighlighter/common/unicodeHighlighter.js";
import { StickyScrollController } from "../contrib/stickyScroll/browser/stickyScrollController.js";
import { SectionHeadersController } from "../contrib/sectionHeaders/browser/sectionHeadersController.js";
import { SymbolIconsController } from "../contrib/symbolIcons/browser/symbolIconsController.js";
import { MessageController } from "../contrib/message/browser/messageController.js";
import { InlineProgressController } from "../contrib/inlineProgress/browser/inlineProgressController.js";
import { ColorPickerController } from "../contrib/colorPicker/browser/colorPickerController.js";
import { LinkedEditingController } from "../contrib/linkedEditing/browser/linkedEditingController.js";
import { CodeLensController, type ExecuteCodeLensCommand } from "../contrib/codelens/browser/codelensController.js";
import { RustSyntaxAnalysisWorker, RustSyntaxDocumentSymbolProvider, RustSyntaxFactsService } from "./services/rustSyntaxFactsService.js";
import { RustSyntaxFoldingService } from "./services/rustSyntaxFoldingService.js";

export interface EditorSessionOptions {
  readonly container: HTMLElement;
  readonly input: EditorInput;
  readonly languageId: string;
  /** Optional shared language registrations and providers for this editor host. */
  readonly languageFeaturesService?: ILanguageFeaturesService;
  /** Optional Rust-backed syntax analysis used for parser-grade fold ranges. */
  readonly syntaxApi?: ISyntaxApi;
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
  readonly lineWrapping?: EditorLineWrapping;
  /** Applies a single LF at the save boundary when the document has content and no final LF. */
  readonly insertFinalNewLine?: boolean;
  /** Browser paragraph direction for this editor session's DOM projection. */
  readonly textDirection?: EditorTextDirection;
  readonly presentation?: EditorViewportPresentation;
  /** Host-owned link opening callback; Alpha never opens external targets directly. */
  readonly onOpenLink?: (target: string) => void | Promise<void>;
  /** Host-owned context-menu composition; Alpha supplies only editor hit-test data. */
  readonly onShowContextMenu?: (request: ContextMenuRequest) => void | Promise<void>;
  /** Host-owned execution for provider commands such as code lenses. */
  readonly onExecuteEditorCommand?: ExecuteCodeLensCommand;
  readonly placeholder?: string;
  readonly showUnicodeHighlights?: boolean;
  readonly fontZoom?: { readonly initialScale?: number };
}

/** Owns all per-pane state projected over one shared Alpha text model reference. */
export class EditorSession extends DisposableOwner {
  readonly onDidChange: Event<void>;
  readonly codeEditor: CodeEditorWidget;
  readonly viewport: EditorViewport;
  readonly selections: EditorSelectionController;
  readonly textInput: TextInputController;
  readonly find: FindController;
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

  constructor(options: EditorSessionOptions) {
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
      const rustSyntaxFacts = options.syntaxApi ? this.own(new RustSyntaxFactsService(options.syntaxApi)) : undefined;
      this.onDidChange = listener => model.onDidChange(() => listener());
      const languageFeaturesService = options.languageFeaturesService ?? this.own(new LanguageFeaturesService());
      const configurations = languageFeaturesService.configurations;
      this.selections = this.own(new EditorSelectionController(
        model,
        TextSelectionSet.single(TextSelection.collapsedAt(TextPosition.at(0, 0))),
        { readOnly: options.input.readOnly },
      ));
      const finalNewLine = options.insertFinalNewLine ? this.own(new InsertFinalNewLineController(this.selections)) : undefined;
      this.beforeSave = finalNewLine ? () => finalNewLine.prepareSave() : undefined;
      const folding = this.own(new EditorFoldingModel(model));
      const hiddenRanges = this.own(new EditorHiddenRangeModel(model, folding));
      let syntaxFolding: RustSyntaxFoldingService | undefined;
      const updateFolding = () => folding.setProviderRanges(mergeEditorFoldingRanges(
        syntaxFolding?.ranges ?? [],
        computeEditorLanguageFoldingRanges(model, this.languageId, configurations),
        computeEditorIndentFoldingRanges(model),
      ));
      if (rustSyntaxFacts) {
        syntaxFolding = this.own(new RustSyntaxFoldingService(
          model,
          this.languageId,
          rustSyntaxFacts,
          updateFolding,
          this.onLanguageError,
        ));
      }
      updateFolding();
      this.own(model.onDidChange(updateFolding));

      this.analysis = this.own(languageFeaturesService.createAnalysisService(model, {
        ...(options.analysisWorkerFactory ? { workerFactory: options.analysisWorkerFactory } : {}),
        ...(rustSyntaxFacts ? { workerDecorator: fallback => new RustSyntaxAnalysisWorker(rustSyntaxFacts, fallback) } : {}),
      }));
      const tokenLines = new LanguageTokenLineIndex(this.analysis.tokens);
      const tokenization = this.own(new TokenizationTextModelPart(tokenLines));
      const diagnostics = this.own(new LanguageDiagnosticDecorationBridge(this.analysis.diagnostics));
      const searchDecorations = this.own(new TextDecorationCollection<void>(model));
      const occurrenceDecorations = this.own(new TextDecorationCollection<void>(model));
      const bracketDecorations = this.own(new TextDecorationCollection<void>(model));
      const unicodeDecorations = this.own(new TextDecorationCollection<UnicodeHighlight>(model));
      const unusualLineTerminatorDecorations = this.own(new TextDecorationCollection<void>(model));
      if (options.showUnicodeHighlights !== false) this.own(new UnicodeHighlighterController(model, unicodeDecorations));
      this.own(new UnusualLineTerminatorsController(model, unusualLineTerminatorDecorations));
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
      const documentSymbolOptions = rustSyntaxFacts
        ? { fallbackProviders: [new RustSyntaxDocumentSymbolProvider(rustSyntaxFacts)] }
        : undefined;
      const gotoSymbol = this.own(languageFeaturesService.createGotoSymbolService(model, documentSymbolOptions));
      const documentSymbols = this.own(languageFeaturesService.createDocumentSymbolService(model, documentSymbolOptions));
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
            createAlphaDecorationSource(searchDecorations, () => DecorationPresentation.SearchMatch),
            createAlphaDecorationSource(occurrenceDecorations, () => DecorationPresentation.OccurrenceHighlight),
            createAlphaDecorationSource(bracketDecorations, () => DecorationPresentation.BracketMatch),
            createAlphaDecorationSource(unicodeDecorations, () => DecorationPresentation.UnicodeHighlight, decoration => `${decoration.metadata.kind} Unicode character U+${decoration.metadata.character.codePointAt(0)!.toString(16).toUpperCase()}`),
            createAlphaDecorationSource(unusualLineTerminatorDecorations, () => DecorationPresentation.UnusualLineTerminator, () => "Unusual line terminator"),
          ],
          semanticTokenSource: semanticTokens,
          bracketColorizationSource: new BracketColorizationSource(bracketColorizations),
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
      this.own(new TokenizationController(this.viewport, tokenization));
      this.own(new EditorStateController(this.textInput.element, this.viewport, this.selections, editorState));
      if (options.input.readOnly) this.own(new ReadOnlyMessageController(this.textInput.element, this.viewport));
      this.own(modelReference.onDidChangeExternalChange(() => {
        if (modelReference.hasExternalChange) {
          this.viewport.announceAccessibilityStatus("File changed on disk. Local edits are preserved.");
        }
      }));
      if (this.onSave) this.own(new SaveController(this.textInput.element, {
        save: this.onSave,
        beforeSave: this.beforeSave,
        onSaveSuccess: () => this.viewport.announceAccessibilityStatus("Saved"),
        onSaveError: error => {
          this.viewport.announceAccessibilityStatus(`Save failed: ${saveErrorMessage(error)}`);
          this.onSaveError(error);
        },
      }));
      this.own(new DiagnosticNavigationController(this.textInput.element, this.viewport, this.selections, diagnostics.decorations));
      this.own(new DiagnosticHoverController(this.viewport));
      this.own(new HoverController(this.viewport, hover, this.languageId));
      this.own(new FormatController(this.textInput.element, this.viewport, this.selections, formatting, this.languageId, {
        formattingOptions: { tabSize: options.indentation?.tabSize ?? 4, insertSpaces: options.indentation?.kind !== "tabs" },
        onError: this.onLanguageError,
      }));
      this.own(new RenameController(this.textInput.element, this.viewport, this.selections, rename, this.languageId, this.onLanguageError));
      this.own(new CodeActionController(this.textInput.element, this.viewport, this.selections, codeActions, diagnostics.decorations, this.languageId, this.onLanguageError));
      if (options.onOpenLink) this.own(new LinksController(this.viewport, links, this.languageId, options.onOpenLink, this.onLanguageError));
      this.own(new InlayHintsController(this.viewport, inlayHints, this.languageId, this.onLanguageError));
      this.own(new InlineCompletionsController(this.textInput.element, this.viewport, this.selections, inlineCompletions, this.languageId, this.onLanguageError));
      this.own(new ParameterHintsController(this.textInput.element, this.viewport, this.selections, parameterHints, this.languageId, this.onLanguageError));
      this.own(new GotoSymbolController(this.textInput.element, this.viewport, this.selections, gotoSymbol, this.languageId, this.onLanguageError));
      this.own(new AnchorSelectController(this.textInput.element, this.viewport, this.selections, () => configurations.getLanguageConfiguration(this.languageId).wordPattern));
      this.own(new SmartSelectController(this.textInput.element, this.viewport, this.selections, () => configurations.getLanguageConfiguration(this.languageId).wordPattern));
      this.own(new InPlaceReplaceController(this.textInput.element, this.viewport, this.selections));
      this.own(new FontZoomController(this.textInput.element, this.viewport, { baseLineHeight: 20, initialScale: options.fontZoom?.initialScale }));
      this.own(new MiddleScrollController(this.viewport));
      this.own(new ToggleTabFocusModeController(this.textInput.element, this.viewport));
      if (options.placeholder) this.own(new PlaceholderTextController(this.viewport, options.placeholder));
      if (options.onShowContextMenu) this.own(new ContextMenuController(this.viewport, options.onShowContextMenu, this.onLanguageError));
      this.own(new StickyScrollController(this.viewport, folding));
      this.own(new SectionHeadersController(this.viewport, folding));
      this.own(new SymbolIconsController(this.viewport, documentSymbols, this.languageId, this.onLanguageError));
      this.own(new CodeLensController(this.viewport, codeLenses, this.languageId, options.onExecuteEditorCommand, this.onLanguageError));
      this.own(new ColorPickerController(this.textInput.element, this.viewport, this.selections, colors, this.languageId, this.onLanguageError));
      this.own(new LinkedEditingController(this.textInput.element, this.viewport, this.selections, linkedEditing, this.languageId, this.onLanguageError));
      this.own(new MessageController(this.viewport));
      this.own(new InlineProgressController(this.viewport));
      this.find = this.own(new FindController(this.textInput.element, this.viewport, this.selections, searchDecorations));
      this.own(new GotoLineController(this.textInput.element, this.viewport, this.selections));
      this.own(new BracketMatchController(this.selections, bracketMatcher, bracketDecorations));
      this.own(new BracketNavigationController(this.textInput.element, this.viewport, this.selections, bracketMatcher));
      this.own(new BracketEditingController(this.textInput.element, this.viewport, this.selections, bracketMatcher));
      this.own(new EditingCommandController(this.textInput.element, this.viewport, this.selections));
      this.own(new FoldingController(this.textInput.element, this.viewport, this.selections, folding));
      this.own(new LineCommentController(this.textInput.element, this.viewport, this.selections, {
        languageId: this.languageId,
        configurations,
      }));
      this.own(new BlockCommentController(this.textInput.element, this.viewport, this.selections, {
        languageId: this.languageId,
        configurations,
      }));
      this.own(new LineOperationsController(this.textInput.element, this.viewport, this.selections, { indentation: options.indentation }));
      this.own(new LineJoinController(this.textInput.element, this.viewport, this.selections));
      this.own(new TransposeController(this.textInput.element, this.viewport, this.selections));
      this.own(new WordWrapController(this.textInput.element, this.viewport));
      this.own(new MultiCursorController(this.textInput.element, this.viewport, this.selections));
      this.own(new CursorUndoController(this.textInput.element, this.viewport, this.selections));
      this.own(new OccurrenceSelectionController(this.textInput.element, this.viewport, this.selections, {
        wordPattern: () => configurations.getLanguageConfiguration(this.languageId).wordPattern,
      }));
      this.own(new OccurrenceHighlightController(this.selections, occurrenceDecorations, {
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

function validateOptions(options: EditorSessionOptions): void {
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
