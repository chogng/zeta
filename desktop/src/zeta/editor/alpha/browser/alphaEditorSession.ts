import { type IDimension } from "../../../base/browser/geometry.js";
import { isCancellationError } from "../../../base/common/cancellation.js";
import { type Event } from "../../../base/common/event.js";
import { DisposableOwner, type IDisposable } from "../../../base/common/lifecycle.js";
import { type EditorInput } from "../../../workbench/browser/parts/editor/editorInput.js";
import { type AlphaTextModelReference } from "./alphaTextModelService.js";
import { CodeEditorWidget } from "./widget/codeEditor/codeEditorWidget.js";
import { type AlphaEditorTextDirection, type AlphaEditorViewport } from "./alphaEditorViewport.js";
import { AlphaDecorationPresentation, createAlphaDecorationSource } from "./decorationPresentation.js";
import { AlphaCursorUndoController } from "./cursorUndoController.js";
import { AlphaDiagnosticNavigationController } from "./diagnosticNavigationController.js";
import { AlphaDiagnosticHoverController } from "./diagnosticHoverController.js";
import { AlphaBracketMatchController } from "./bracketMatchController.js";
import { AlphaBracketColorizationSource } from "./bracketColorizationPresentation.js";
import { AlphaBracketEditingController } from "./bracketEditingController.js";
import { AlphaBracketNavigationController } from "./bracketNavigationController.js";
import { AlphaBlockCommentController } from "./blockCommentController.js";
import { AlphaEditingCommandController } from "./editingCommandController.js";
import { AlphaFoldingController } from "./foldingController.js";
import { AlphaFindController } from "./findController.js";
import { AlphaGotoLineController } from "./gotoLineController.js";
import { AlphaLineCommentController } from "./lineCommentController.js";
import { AlphaLineJoinController } from "./lineJoinController.js";
import { AlphaLineOperationsController } from "./lineOperationsController.js";
import { AlphaMultiCursorController } from "./multiCursorController.js";
import { AlphaOccurrenceSelectionController } from "./occurrenceSelectionController.js";
import { AlphaOccurrenceHighlightController } from "./occurrenceHighlightController.js";
import { createAlphaLanguageDiagnosticSource } from "./languageDiagnosticPresentation.js";
import { AlphaSaveController } from "./saveController.js";
import { createAlphaSemanticTokenSource } from "./semanticTokenPresentation.js";
import { type AlphaTextInputController } from "./textInputController.js";
import { AlphaTransposeController } from "./transposeController.js";
import { AlphaWordWrapController } from "./wordWrapController.js";
import { type AlphaEditorLineWrapping } from "./visualLineProjection.js";
import { LanguageAnalysisProviderRegistry } from "../common/languageAnalysisProviders.js";
import { LanguageAnalysisService, type LanguageAnalysisWorkerFactory } from "../common/languageAnalysisService.js";
import { registerAlphaBuiltinLanguageConfigurations } from "../common/languageBuiltinConfigurations.js";
import { LanguageCompletionProviderRegistry } from "../common/languageCompletionProviders.js";
import { LanguageCompletionService, type LanguageCompletionWorkerFactory } from "../common/languageCompletionService.js";
import { LanguageCompletionSessionController } from "../common/languageCompletionSession.js";
import { LanguageConfigurationRegistry } from "../common/languageConfiguration.js";
import { LanguageBracketMatcher } from "../common/languageBracketMatcher.js";
import { LanguageBracketColorizationIndex } from "../common/languageBracketColorization.js";
import { LanguageLexicalContextIndex } from "../common/languageLexicalContext.js";
import { LanguageDiagnosticDecorationBridge } from "../common/languageDiagnosticDecorations.js";
import { createLanguageLexicalAnalysisProvider } from "../common/languageLexicalAnalysisProvider.js";
import { LanguageTokenLineIndex } from "../common/languageTokenLineIndex.js";
import { createLanguageWordCompletionProvider } from "../common/languageWordCompletionProvider.js";
import { EditorSelectionController } from "../common/editorSelectionController.js";
import { EditorFoldingModel } from "../common/folding.js";
import { computeEditorIndentFoldingRanges } from "../common/indentFolding.js";
import { computeEditorLanguageFoldingRanges, mergeEditorFoldingRanges } from "../common/languageFolding.js";
import { type EditorIndentationOptions } from "../common/editorIndentation.js";
import { TextDecorationCollection } from "../common/decoration.js";
import { TextSelection, TextSelectionSet } from "../common/selection.js";
import { TextPosition } from "../common/text.js";

export interface AlphaEditorSessionOptions {
  readonly container: HTMLElement;
  readonly input: EditorInput;
  readonly languageId: string;
  readonly modelReference: AlphaTextModelReference;
  readonly analysisWorkerFactory?: LanguageAnalysisWorkerFactory;
  readonly completionWorkerFactory?: LanguageCompletionWorkerFactory;
  readonly languageSupport?: IDisposable;
  readonly onDidChangeLanguageSupport?: Event<void>;
  readonly whenLanguageSupportReady?: () => Promise<unknown>;
  readonly onLanguageError?: (error: unknown) => void;
  readonly onSaveError?: (error: unknown) => void;
  readonly onSave?: () => Promise<void>;
  readonly onRevert?: () => Promise<void>;
  readonly indentation?: EditorIndentationOptions;
  readonly lineWrapping?: AlphaEditorLineWrapping;
  /** Browser paragraph direction for this editor session's DOM projection. */
  readonly textDirection?: AlphaEditorTextDirection;
}

/** Owns all per-pane state projected over one shared Alpha text model reference. */
export class AlphaEditorSession extends DisposableOwner {
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
  private readonly modelReference: AlphaTextModelReference;
  private readonly onSave: (() => Promise<void>) | undefined;
  private readonly onRevert: (() => Promise<void>) | undefined;
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
      const configurations = this.own(new LanguageConfigurationRegistry());
      this.own(registerAlphaBuiltinLanguageConfigurations(configurations));
      this.selections = this.own(new EditorSelectionController(
        model,
        TextSelectionSet.single(TextSelection.collapsedAt(TextPosition.at(0, 0))),
        { readOnly: options.input.readOnly },
      ));
      const folding = this.own(new EditorFoldingModel(model));
      const updateFolding = () => folding.setProviderRanges(mergeEditorFoldingRanges(
        computeEditorLanguageFoldingRanges(model, this.languageId, configurations),
        computeEditorIndentFoldingRanges(model),
      ));
      updateFolding();
      this.own(model.onDidChange(updateFolding));

      const analysisProviders = this.own(new LanguageAnalysisProviderRegistry());
      if (!options.analysisWorkerFactory) {
        this.own(analysisProviders.register(createLanguageLexicalAnalysisProvider({ languageConfigurations: configurations })));
      }
      this.analysis = this.own(new LanguageAnalysisService(model, analysisProviders, {
        ...(options.analysisWorkerFactory ? { workerFactory: options.analysisWorkerFactory } : {}),
      }));
      const tokenLines = this.own(new LanguageTokenLineIndex(this.analysis.tokens));
      const diagnostics = this.own(new LanguageDiagnosticDecorationBridge(this.analysis.diagnostics));
      const searchDecorations = this.own(new TextDecorationCollection<void>(model));
      const occurrenceDecorations = this.own(new TextDecorationCollection<void>(model));
      const bracketDecorations = this.own(new TextDecorationCollection<void>(model));
      const bracketMatcher = this.own(new LanguageBracketMatcher(model, this.languageId, configurations));
      const lexicalContext = this.own(new LanguageLexicalContextIndex(model, this.languageId, configurations));
      const bracketColorizations = this.own(new LanguageBracketColorizationIndex(model, lexicalContext));

      const completionProviders = this.own(new LanguageCompletionProviderRegistry());
      if (!options.completionWorkerFactory) {
        this.own(completionProviders.register(createLanguageWordCompletionProvider()));
      }
      const completions = this.own(new LanguageCompletionService(model, completionProviders, {
        ...(options.completionWorkerFactory ? { workerFactory: options.completionWorkerFactory } : {}),
      }));
      const completionSession = this.own(new LanguageCompletionSessionController(
        completions.results,
        this.selections,
        {
          resolver: completions,
          onResolveError: this.onLanguageError,
          snippetVariables: createAlphaSnippetVariables(options.input),
        },
      ));
      const semanticTokens = createAlphaSemanticTokenSource(tokenLines);
      const ariaLabel = editorLabel(options.input);

      this.codeEditor = this.own(new CodeEditorWidget({
        container: options.container,
        model,
        lineHeight: 20,
        selectionController: this.selections,
        ariaLabel,
        viewport: {
          foldingModel: folding,
          decorationSources: [
            createAlphaLanguageDiagnosticSource(diagnostics.decorations),
            createAlphaDecorationSource(searchDecorations, () => AlphaDecorationPresentation.SearchMatch),
            createAlphaDecorationSource(occurrenceDecorations, () => AlphaDecorationPresentation.OccurrenceHighlight),
            createAlphaDecorationSource(bracketDecorations, () => AlphaDecorationPresentation.BracketMatch),
          ],
          semanticTokenSource: semanticTokens,
          bracketColorizationSource: new AlphaBracketColorizationSource(bracketColorizations),
          lineWrapping: options.lineWrapping,
          textDirection: options.textDirection,
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
      this.own(modelReference.onDidChangeExternalChange(() => {
        if (modelReference.hasExternalChange) {
          this.viewport.announceAccessibilityStatus("File changed on disk. Local edits are preserved.");
        }
      }));
      if (this.onSave) this.own(new AlphaSaveController(this.textInput.element, {
        save: this.onSave,
        onSaveSuccess: () => this.viewport.announceAccessibilityStatus("Saved"),
        onSaveError: error => {
          this.viewport.announceAccessibilityStatus(`Save failed: ${saveErrorMessage(error)}`);
          this.onSaveError(error);
        },
      }));
      this.own(new AlphaDiagnosticNavigationController(this.textInput.element, this.viewport, this.selections, diagnostics.decorations));
      this.own(new AlphaDiagnosticHoverController(this.viewport));
      this.find = this.own(new AlphaFindController(this.textInput.element, this.viewport, this.selections, searchDecorations));
      this.own(new AlphaGotoLineController(this.textInput.element, this.viewport, this.selections));
      this.own(new AlphaBracketMatchController(this.selections, bracketMatcher, bracketDecorations));
      this.own(new AlphaBracketNavigationController(this.textInput.element, this.viewport, this.selections, bracketMatcher));
      this.own(new AlphaBracketEditingController(this.textInput.element, this.viewport, this.selections, bracketMatcher));
      this.own(new AlphaEditingCommandController(this.textInput.element, this.viewport, this.selections, { indentation: options.indentation }));
      this.own(new AlphaFoldingController(this.textInput.element, this.viewport, this.selections, folding));
      this.own(new AlphaLineCommentController(this.textInput.element, this.viewport, this.selections, {
        languageId: this.languageId,
        configurations,
      }));
      this.own(new AlphaBlockCommentController(this.textInput.element, this.viewport, this.selections, {
        languageId: this.languageId,
        configurations,
      }));
      this.own(new AlphaLineOperationsController(this.textInput.element, this.viewport, this.selections));
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

  get isDirty(): boolean {
    return this.modelReference.isDirty;
  }

  get hasExternalChange(): boolean {
    return this.modelReference.hasExternalChange;
  }

  async save(): Promise<void> {
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
