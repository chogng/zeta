import { type IDimension } from "../../base/browser/geometry.js";
import { type Event } from "../../base/common/event.js";
import { DisposableOwner, toDisposable, type IDisposable } from "../../base/common/lifecycle.js";
import { type EditorResourceInput } from "../common/editorResource.js";
import { type TextModelReference } from "../common/services/textModelService.js";
import { CodeEditorWidget } from "../browser/widget/codeEditor/codeEditorWidget.js";
import { type EditorViewport } from "../browser/view/editorViewport.js";
import { EditingCommandController } from "../browser/editingCommandController.js";
import { type TextInputController } from "../browser/input/textInputController.js";
import { LanguageFeaturesService } from "../common/services/languageService.js";
import { EditorSelectionController } from "../common/cursor/editorSelectionController.js";
import { TextSelection, TextSelectionSet } from "../common/core/selection.js";
import { TextPosition, type TextRange } from "../common/core/text.js";
import { registerEditorPartFactory, type EditorPartOptions, type IEditorPartRuntime } from "../browser/editorPart.js";
import { getEditorContributions, type EditorCapability } from "../browser/editorContribution.js";
import { type DecorationSource } from "../browser/view/decorationPresentation.js";
import { type EditorLineGutterDecoration } from "../browser/view/lineGutterDecoration.js";
import { type BracketColorizationSource, type SemanticTokenSource } from "../browser/view/semanticTokenPresentation.js";
import { type EditorLineVisibilitySource } from "../common/viewModel/modelLineProjection.js";
import { type LanguageLexicalContextSource } from "../common/languages/languageLexicalContext.js";
import { type TextInputCompletionOptions } from "../browser/input/textInputController.js";
import { type TextInputLanguageEditingAdapter } from "../browser/input/textInputController.js";

/** Owns all per-pane state projected over one shared text model reference. */
class ContributedEditorPart extends DisposableOwner implements IEditorPartRuntime {
  readonly onDidChange: Event<void>;
  readonly codeEditor: CodeEditorWidget;
  readonly viewport: EditorViewport;
  readonly selections: EditorSelectionController;
  readonly textInput: TextInputController;
  private readonly languageId: string;
  private readonly onLanguageError: (error: unknown) => void;
  private readonly modelReference: TextModelReference;
  private readonly onSave: (() => Promise<void | boolean>) | undefined;
  private readonly onRevert: (() => Promise<void>) | undefined;
  private readonly beforeSaveHooks: Array<() => void | Promise<void>> = [];

  constructor(options: EditorPartOptions) {
    super();
    try {
      validateOptions(options);
      this.languageId = options.languageId;
      this.onLanguageError = options.onLanguageError ?? reportLanguageError;
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
      const contributionCapabilities = new Map<string, unknown>();
      const getCapability = <T>(capability: EditorCapability<T>): T => {
        if (!contributionCapabilities.has(capability.id)) throw new ReferenceError(`Text editor capability '${capability.id}' is unavailable`);
        return contributionCapabilities.get(capability.id) as T;
      };
      const getOptionalCapability = <T>(capability: EditorCapability<T>): T | undefined => contributionCapabilities.get(capability.id) as T | undefined;
      const provideCapability = <T>(capability: EditorCapability<T>, value: T): void => {
        if (contributionCapabilities.has(capability.id)) throw new RangeError(`Text editor capability '${capability.id}' is already provided`);
        contributionCapabilities.set(capability.id, value);
      };
      const decorationSources: DecorationSource[] = [];
      let lineProjection: { readonly visibilitySource: EditorLineVisibilitySource; readonly gutterDecoration?: EditorLineGutterDecoration } | undefined;
      let semanticTokenSource: SemanticTokenSource | undefined;
      let bracketColorizationSource: BracketColorizationSource | undefined;
      let languageLexicalContext: LanguageLexicalContextSource | undefined;
      let textInputCompletion: TextInputCompletionOptions | undefined;
      let textInputLanguageEditing: TextInputLanguageEditingAdapter | undefined;
      const selectedContributions = getEditorContributions();
      for (const contribution of selectedContributions) {
        contribution.configure?.({
          kind: "text",
          options,
          model,
          languageId: this.languageId,
          languageFeaturesService,
          configurations,
          selections: this.selections,
          onLanguageError: this.onLanguageError,
          getCapability,
          getOptionalCapability,
          provideCapability,
          addDecorationSource: source => decorationSources.push(source),
          setLineProjection: projection => {
            if (lineProjection) throw new Error("Text editor line projection is already configured");
            lineProjection = projection;
          },
          setSemanticTokenSource: source => {
            if (semanticTokenSource) throw new Error("Text editor semantic-token source is already configured");
            semanticTokenSource = source;
          },
          setBracketColorizationSource: source => {
            if (bracketColorizationSource) throw new Error("Text editor bracket-colorization source is already configured");
            bracketColorizationSource = source;
          },
          setLanguageLexicalContext: source => {
            if (languageLexicalContext) throw new Error("Text editor lexical context is already configured");
            languageLexicalContext = source;
          },
          setTextInputCompletion: completion => {
            if (textInputCompletion) throw new Error("Text editor completion input is already configured");
            textInputCompletion = completion;
          },
          setTextInputLanguageEditing: adapter => {
            if (textInputLanguageEditing) throw new Error("Text editor language editing is already configured");
            textInputLanguageEditing = adapter;
          },
          own: value => this.own(value),
        });
      }
      const ariaLabel = editorLabel(options.input);

      this.codeEditor = this.own(new CodeEditorWidget({
        container: options.container,
        model,
        lineHeight: 20,
        selectionController: this.selections,
        ariaLabel,
        viewport: {
          lineVisibilitySource: lineProjection?.visibilitySource,
          lineGutterDecoration: lineProjection?.gutterDecoration,
          decorationSources,
          semanticTokenSource,
          bracketColorizationSource,
          lineWrapping: options.lineWrapping,
          textDirection: options.textDirection,
          presentation: options.presentation,
          indentation: options.indentation,
        },
        textInput: {
          languageEditing: textInputLanguageEditing,
          wordPattern: () => configurations.getLanguageConfiguration(this.languageId).wordPattern,
          completion: textInputCompletion,
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
      this.own(new EditingCommandController(this.textInput.element, this.viewport, this.selections));
      for (const contribution of selectedContributions) {
        contribution.install?.({
          kind: "text",
          options,
          model,
          languageId: this.languageId,
          languageFeaturesService,
          configurations,
          textInput: this.textInput,
          viewport: this.viewport,
          selections: this.selections,
          onLanguageError: this.onLanguageError,
          getCapability,
          getOptionalCapability,
          registerBeforeSave: hook => {
            if (typeof hook !== "function") throw new TypeError("Editor before-save hook must be a function");
            this.beforeSaveHooks.push(hook);
            return toDisposable(() => {
              const index = this.beforeSaveHooks.indexOf(hook);
              if (index >= 0) this.beforeSaveHooks.splice(index, 1);
            });
          },
          own: value => this.own(value),
        });
      }
    } catch (error) {
      this.dispose();
      throw error;
    }
  }

  layout(dimension: IDimension): void {
    this.codeEditor.layout(dimension);
  }

  announceAccessibilityStatus(message: string): void {
    this.viewport.announceAccessibilityStatus(message);
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

  revealRange(range: TextRange): void {
    this.viewport.textModel.offsetAt(range.start);
    this.viewport.textModel.offsetAt(range.end);
    this.selections.setSelections(TextSelectionSet.single(TextSelection.from(range.start, range.end)));
    this.viewport.revealPosition(range.start);
  }

  get isDirty(): boolean {
    return this.modelReference.isDirty;
  }

  get hasExternalChange(): boolean {
    return this.modelReference.hasExternalChange;
  }

  async save(): Promise<void> {
    for (const hook of [...this.beforeSaveHooks]) await hook();
    await this.onSave?.();
  }

  async revert(): Promise<void> {
    await this.onRevert?.();
  }

}

registerEditorPartFactory(options => new ContributedEditorPart(options));

function validateOptions(options: EditorPartOptions): void {
  if (!options || typeof options !== "object" || !options.container || !options.modelReference) {
    throw new TypeError("Editor part requires a container and model reference");
  }
  if (options.input?.readOnly !== undefined && typeof options.input.readOnly !== "boolean") {
    throw new TypeError("Editor input read-only mode must be boolean");
  }
  if (options.whenLanguageSupportReady !== undefined && typeof options.whenLanguageSupportReady !== "function") {
    throw new TypeError("Editor language readiness must be a function");
  }
  if (options.onLanguageError !== undefined && typeof options.onLanguageError !== "function") {
    throw new TypeError("Editor language error handler must be a function");
  }
  if (options.onSave !== undefined && typeof options.onSave !== "function") {
    throw new TypeError("Editor save must be a function");
  }
  if (options.onRevert !== undefined && typeof options.onRevert !== "function") {
    throw new TypeError("Editor revert must be a function");
  }
  if (options.insertFinalNewLine !== undefined && typeof options.insertFinalNewLine !== "boolean") {
    throw new TypeError("Editor final newline option must be boolean");
  }
}

function editorLabel(input: EditorResourceInput): string {
  if (input.label?.trim()) return input.label;
  const path = decodeURIComponent(input.resource.path);
  return path.slice(path.lastIndexOf("/") + 1) || "Text editor";
}

function reportLanguageError(error: unknown): void {
  console.error("Editor language request failed", error);
}
