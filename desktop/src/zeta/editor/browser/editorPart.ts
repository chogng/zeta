import { type IDimension } from "../../base/browser/geometry.js";
import { type Event } from "../../base/common/event.js";
import { DisposableOwner, type IDisposable } from "../../base/common/lifecycle.js";
import { type ISyntaxApi } from "../../platform/syntax/common/syntaxApi.js";
import { type EditorInput } from "../../workbench/browser/parts/editor/editorInput.js";
import { type EditorSelectionController } from "../common/cursor/editorSelectionController.js";
import { type TextPosition } from "../common/core/text.js";
import { type LanguageCompletionWorkerFactory } from "../common/languages/completion/languageCompletionService.js";
import { type SyntaxWorkerFactory } from "../common/languages/syntax/syntaxService.js";
import { type ILanguageFeaturesService } from "../common/services/languageService.js";
import { type TextModelReference } from "../common/services/textModelService.js";
import { type EditorIndentationOptions } from "../contrib/indentation/common/indentation.js";
import { type TextInputController } from "./input/textInputController.js";
import { type CodeEditorWidget } from "./widget/codeEditor/codeEditorWidget.js";
import { type EditorHitTarget } from "./view/pointerHitTest.js";
import { type EditorTextDirection, type EditorViewport, type EditorViewportPresentation } from "./view/editorViewport.js";
import { type EditorLineWrapping } from "./view/visualLineProjection.js";

export interface EditorContextMenuRequest {
  readonly position: TextPosition;
  readonly target: EditorHitTarget | undefined;
  readonly clientX: number;
  readonly clientY: number;
}

export interface EditorPartOptions {
  readonly container: HTMLElement;
  readonly input: EditorInput;
  readonly languageId: string;
  /** Optional shared language registrations and providers for this editor host. */
  readonly languageFeaturesService?: ILanguageFeaturesService;
  /** Optional Rust-backed syntax facts used for parser-grade fold ranges. */
  readonly syntaxApi?: ISyntaxApi;
  readonly modelReference: TextModelReference;
  readonly syntaxWorkerFactory?: SyntaxWorkerFactory;
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
  /** Browser paragraph direction for this editor part's DOM projection. */
  readonly textDirection?: EditorTextDirection;
  readonly presentation?: EditorViewportPresentation;
  /** Host-owned link opening callback; the editor never opens external targets directly. */
  readonly onOpenLink?: (target: string) => void | Promise<void>;
  /** Host-owned context-menu composition; the editor supplies only hit-test data. */
  readonly onShowContextMenu?: (request: EditorContextMenuRequest) => void | Promise<void>;
  /** Host-owned execution for provider commands such as code lenses. */
  readonly onExecuteEditorCommand?: (id: string, args: readonly unknown[] | undefined) => void | Promise<void>;
  readonly placeholder?: string;
  readonly showUnicodeHighlights?: boolean;
  readonly fontZoom?: { readonly initialScale?: number };
}

/** Runtime created by one statically selected line-editor contribution bundle. */
export interface IEditorPartRuntime extends IDisposable {
  readonly onDidChange: Event<void>;
  readonly codeEditor: CodeEditorWidget;
  readonly viewport: EditorViewport;
  readonly selections: EditorSelectionController;
  readonly textInput: TextInputController;
  layout(dimension: IDimension): void;
  focus(): void;
  getValue(): string;
  setValue(value: string): void;
  readonly isDirty: boolean;
  readonly hasExternalChange: boolean;
  save(): Promise<void>;
  revert(): Promise<void>;
}

/** Creates one line-editor runtime from the contributions selected by a product bundle. */
export type EditorPartFactory = (options: EditorPartOptions) => IEditorPartRuntime;

let editorPartFactory: EditorPartFactory | undefined;

/** Installs the canonical line-editor composition before a product registers its pane. */
export function registerEditorPartFactory(factory: EditorPartFactory): void {
  if (typeof factory !== "function") throw new TypeError("Editor part factory must be a function");
  if (editorPartFactory && editorPartFactory !== factory) throw new Error("Editor part factory is already registered");
  editorPartFactory = factory;
}

/** Product-neutral editor host that delegates feature assembly to the selected bundle. */
export class EditorPart extends DisposableOwner implements IEditorPartRuntime {
  private readonly runtime: IEditorPartRuntime;
  readonly onDidChange: Event<void>;
  readonly codeEditor: CodeEditorWidget;
  readonly viewport: EditorViewport;
  readonly selections: EditorSelectionController;
  readonly textInput: TextInputController;

  constructor(options: EditorPartOptions) {
    super();
    const factory = editorPartFactory;
    if (!factory) {
      this.dispose();
      throw new Error("No editor part contributions are registered; import a product editor bundle first");
    }
    try {
      this.runtime = this.own(factory(options));
      this.onDidChange = this.runtime.onDidChange;
      this.codeEditor = this.runtime.codeEditor;
      this.viewport = this.runtime.viewport;
      this.selections = this.runtime.selections;
      this.textInput = this.runtime.textInput;
    } catch (error) {
      this.dispose();
      throw error;
    }
  }

  layout(dimension: IDimension): void { this.runtime.layout(dimension); }
  focus(): void { this.runtime.focus(); }
  getValue(): string { return this.runtime.getValue(); }
  setValue(value: string): void { this.runtime.setValue(value); }
  get isDirty(): boolean { return this.runtime.isDirty; }
  get hasExternalChange(): boolean { return this.runtime.hasExternalChange; }
  save(): Promise<void> { return this.runtime.save(); }
  revert(): Promise<void> { return this.runtime.revert(); }
}
