import "./media/alphaEditorPane.css";
import { type IDimension } from "../../../base/browser/geometry.js";
import { throwIfCancelled } from "../../../base/common/cancellation.js";
import { DisposableOwner, DisposableSlot, type IDisposable } from "../../../base/common/lifecycle.js";
import { assertDefined } from "../../../base/common/types.js";
import { type ITextFileService } from "../../../workbench/services/textfile/common/textFileService.js";
import { type ITextMateService } from "../../../workbench/services/textMate/common/textMateService.js";
import { type ILanguageFeaturesService } from "../../../workbench/services/language/common/languageFeaturesService.js";
import { type EditorInput } from "../../../workbench/browser/parts/editor/editorInput.js";
import { type IEditorPane } from "../../../workbench/browser/parts/editor/editorPane.js";
import { EditorPaneVisibility } from "../../../workbench/browser/parts/editor/editorPane.js";
import { ALPHA_EDITOR_ID, alphaLanguageForInput } from "../common/alphaEditorInput.js";
import { AlphaEditorSession, type AlphaEditorSessionOptions } from "./alphaEditorSession.js";
import { AlphaTextModels, type AlphaTextModelService } from "./alphaTextModelService.js";
import { type AlphaEditorTextDirection } from "./alphaEditorViewport.js";
import { type AlphaEditorLineWrapping } from "./visualLineProjection.js";

export interface AlphaEditorPaneSession extends IDisposable {
  layout(dimension: IDimension): void;
  focus(): void;
  getValue(): string;
  readonly isDirty?: boolean;
  readonly hasExternalChange?: boolean;
  save?(): Promise<void>;
  revert?(): Promise<void>;
}

export interface AlphaEditorPaneSessionOptions extends AlphaEditorSessionOptions {
  readonly textMateService?: ITextMateService;
  readonly languageFeaturesService?: ILanguageFeaturesService;
}

export interface AlphaEditorPaneOptions {
  readonly modelService?: AlphaTextModelService;
  readonly createSession?: (options: AlphaEditorPaneSessionOptions) => AlphaEditorPaneSession;
  readonly textMateService?: ITextMateService;
  readonly languageFeaturesService?: ILanguageFeaturesService;
  readonly lineWrapping?: AlphaEditorLineWrapping;
  /** Browser paragraph direction forwarded to every created Alpha session. */
  readonly textDirection?: AlphaEditorTextDirection;
}

/** Workbench pane that composes Alpha's native model, input, view, and language services. */
export class AlphaEditorPane extends DisposableOwner implements IEditorPane {
  readonly id = ALPHA_EDITOR_ID;
  private readonly sessions = this.own(new DisposableSlot<AlphaEditorPaneSession>());
  private readonly modelService: AlphaTextModelService;
  private readonly createSession: (options: AlphaEditorPaneSessionOptions) => AlphaEditorPaneSession;
  private container: HTMLDivElement | undefined;
  private dimension: IDimension = { width: 0, height: 0 };

  constructor(private readonly textFiles: ITextFileService, private readonly options: AlphaEditorPaneOptions = {}) {
    super();
    if (!textFiles || typeof textFiles.resolve !== "function") {
      this.dispose();
      throw new TypeError("Alpha editor pane requires a text file service");
    }
    this.modelService = options.modelService ?? AlphaTextModels;
    this.createSession = options.createSession ?? (sessionOptions => new AlphaEditorSession(sessionOptions));
  }

  create(parent: HTMLElement): void {
    if (this.container) throw new ReferenceError("AlphaEditorPane has already been created");
    const container = parent.ownerDocument.createElement("div");
    container.className = "zeta-alpha-editor-pane";
    parent.append(container);
    this.container = container;
    this.defer(() => {
      container.remove();
      this.container = undefined;
    });
  }

  async setInput(input: EditorInput, signal: AbortSignal): Promise<void> {
    const container = this.requireContainer();
    throwIfCancelled(signal, "Alpha editor input loading was cancelled");
    const modelReference = await this.modelService.acquire(input, this.textFiles, signal);
    let session: AlphaEditorPaneSession | undefined;
    try {
      throwIfCancelled(signal, "Alpha editor input loading was cancelled");
      session = this.createSession({
        container,
        input,
        languageId: alphaLanguageForInput(input),
        modelReference,
        textMateService: this.options.textMateService,
        languageFeaturesService: this.options.languageFeaturesService,
        lineWrapping: this.options.lineWrapping,
        textDirection: this.options.textDirection,
        onSave: () => modelReference.save(this.textFiles, new AbortController().signal),
        onRevert: () => modelReference.revert(this.textFiles, new AbortController().signal),
      });
      throwIfCancelled(signal, "Alpha editor input loading was cancelled");
    } catch (error) {
      session?.dispose();
      if (!session) modelReference.dispose();
      throw error;
    }
    this.sessions.replace(session);
    session.layout(this.dimension);
  }

  clearInput(): void {
    this.sessions.clear();
  }

  layout(dimension: IDimension): void {
    this.dimension = {
      width: Math.max(0, dimension.width),
      height: Math.max(0, dimension.height),
    };
    this.sessions.value?.layout(this.dimension);
  }

  setVisible(visibility: EditorPaneVisibility): void {
    if (!this.container) return;
    this.container.hidden = visibility === EditorPaneVisibility.Hidden;
    if (visibility === EditorPaneVisibility.Visible) this.sessions.value?.layout(this.dimension);
  }

  focus(): void {
    this.sessions.value?.focus();
  }

  getValue(): string {
    return this.sessions.value?.getValue() ?? "";
  }

  get isDirty(): boolean {
    return this.sessions.value?.isDirty ?? false;
  }

  get hasExternalChange(): boolean {
    return this.sessions.value?.hasExternalChange ?? false;
  }

  async save(): Promise<void> {
    await this.sessions.value?.save?.();
  }

  async revert(): Promise<void> {
    await this.sessions.value?.revert?.();
  }

  private requireContainer(): HTMLDivElement {
    assertDefined(this.container, new ReferenceError("AlphaEditorPane has not been created"));
    return this.container;
  }
}
