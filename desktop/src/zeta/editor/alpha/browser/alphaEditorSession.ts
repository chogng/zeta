import { type IDimension } from "../../../base/browser/geometry.js";
import { isCancellationError } from "../../../base/common/cancellation.js";
import { type Event } from "../../../base/common/event.js";
import { DisposableOwner, type IDisposable } from "../../../base/common/lifecycle.js";
import { type EditorInput } from "../../../workbench/browser/parts/editor/editorInput.js";
import { type AlphaTextModelReference } from "./alphaTextModelService.js";
import { AlphaEditorViewport } from "./alphaEditorViewport.js";
import { AlphaKeyboardNavigationController } from "./keyboardNavigationController.js";
import { createAlphaLanguageDiagnosticSource } from "./languageDiagnosticPresentation.js";
import { AlphaPointerSelectionController } from "./pointerSelectionController.js";
import { createAlphaSemanticTokenSource } from "./semanticTokenPresentation.js";
import { AlphaTextInputController } from "./textInputController.js";
import { LanguageAnalysisProviderRegistry } from "../common/languageAnalysisProviders.js";
import { LanguageAnalysisService, type LanguageAnalysisWorkerFactory } from "../common/languageAnalysisService.js";
import { registerAlphaBuiltinLanguageConfigurations } from "../common/languageBuiltinConfigurations.js";
import { LanguageCompletionProviderRegistry } from "../common/languageCompletionProviders.js";
import { LanguageCompletionService, type LanguageCompletionWorkerFactory } from "../common/languageCompletionService.js";
import { LanguageCompletionSessionController } from "../common/languageCompletionSession.js";
import { LanguageConfigurationRegistry } from "../common/languageConfiguration.js";
import { LanguageDiagnosticDecorationBridge } from "../common/languageDiagnosticDecorations.js";
import { createLanguageLexicalAnalysisProvider } from "../common/languageLexicalAnalysisProvider.js";
import { LanguageTokenLineIndex } from "../common/languageTokenLineIndex.js";
import { createLanguageWordCompletionProvider } from "../common/languageWordCompletionProvider.js";
import { EditorSelectionController } from "../common/editorSelectionController.js";
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
}

/** Owns all per-pane state projected over one shared Alpha text model reference. */
export class AlphaEditorSession extends DisposableOwner {
  readonly viewport: AlphaEditorViewport;
  readonly selections: EditorSelectionController;
  readonly textInput: AlphaTextInputController;
  private readonly analysis: LanguageAnalysisService;
  private readonly languageId: string;
  private readonly whenLanguageSupportReady: () => Promise<unknown>;
  private readonly onLanguageError: (error: unknown) => void;
  private analysisGeneration = 0;
  private disposed = false;

  constructor(options: AlphaEditorSessionOptions) {
    super();
    try {
      validateOptions(options);
      this.languageId = options.languageId;
      this.whenLanguageSupportReady = options.whenLanguageSupportReady ?? (() => Promise.resolve());
      this.onLanguageError = options.onLanguageError ?? reportLanguageError;
      if (options.languageSupport) this.own(options.languageSupport);
      const modelReference = this.own(options.modelReference);
      const model = modelReference.model;
      const configurations = this.own(new LanguageConfigurationRegistry());
      this.own(registerAlphaBuiltinLanguageConfigurations(configurations));
      this.selections = this.own(new EditorSelectionController(
        model,
        TextSelectionSet.single(TextSelection.collapsedAt(TextPosition.at(0, 0))),
      ));

      const analysisProviders = this.own(new LanguageAnalysisProviderRegistry());
      if (!options.analysisWorkerFactory) {
        this.own(analysisProviders.register(createLanguageLexicalAnalysisProvider({ languageConfigurations: configurations })));
      }
      this.analysis = this.own(new LanguageAnalysisService(model, analysisProviders, {
        ...(options.analysisWorkerFactory ? { workerFactory: options.analysisWorkerFactory } : {}),
      }));
      const tokenLines = this.own(new LanguageTokenLineIndex(this.analysis.tokens));
      const diagnostics = this.own(new LanguageDiagnosticDecorationBridge(this.analysis.diagnostics));

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
        { resolver: completions, onResolveError: this.onLanguageError },
      ));

      this.viewport = this.own(new AlphaEditorViewport({
        container: options.container,
        model,
        lineHeight: 20,
        ariaLabel: editorLabel(options.input),
        selectionController: this.selections,
        decorationSources: [createAlphaLanguageDiagnosticSource(diagnostics.decorations)],
        semanticTokenSource: createAlphaSemanticTokenSource(tokenLines),
      }));
      this.textInput = this.own(new AlphaTextInputController(this.viewport, this.selections, {
        ariaLabel: editorLabel(options.input),
        language: {
          languageId: this.languageId,
          configurations,
        },
        completion: {
          session: completionSession,
          requests: {
            service: completions,
            languageId: this.languageId,
            onRequestError: this.onLanguageError,
          },
        },
      }));
      this.own(new AlphaKeyboardNavigationController(this.viewport, this.selections));
      this.own(new AlphaPointerSelectionController(this.viewport, this.selections));
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
    this.viewport.layout({
      width: Math.max(0, dimension.width),
      height: Math.max(0, dimension.height),
    });
  }

  focus(): void {
    this.textInput.focus();
  }

  getValue(): string {
    return this.viewport.textModel.getText();
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
  if (options.whenLanguageSupportReady !== undefined && typeof options.whenLanguageSupportReady !== "function") {
    throw new TypeError("Alpha language readiness must be a function");
  }
  if (options.onLanguageError !== undefined && typeof options.onLanguageError !== "function") {
    throw new TypeError("Alpha language error handler must be a function");
  }
}

function editorLabel(input: EditorInput): string {
  if (input.label?.trim()) return input.label;
  const path = decodeURIComponent(input.resource.path);
  return path.slice(path.lastIndexOf("/") + 1) || "Alpha editor";
}

function isAbortError(error: unknown): boolean {
  return error instanceof Error && error.name === "AbortError";
}

function reportLanguageError(error: unknown): void {
  console.error("Alpha language request failed", error);
}
