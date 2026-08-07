import "./media/editorPane.css";
import { type IDimension } from "../../../base/browser/geometry.js";
import { throwIfCancelled } from "../../../base/common/cancellation.js";
import { DisposableOwner, DisposableSlot, type IDisposable } from "../../../base/common/lifecycle.js";
import { assertDefined } from "../../../base/common/types.js";
import type { URI } from "../../../base/common/uri.js";
import { type ITextMateService } from "../../../workbench/services/textMate/common/textMateService.js";
import { type ILanguageFeaturesService } from "../common/services/languageService.js";
import { type EditorInput } from "../../../workbench/browser/parts/editor/editorInput.js";
import { type IEditorPane } from "../../../workbench/browser/parts/editor/editorPane.js";
import { EditorPaneVisibility } from "../../../workbench/browser/parts/editor/editorPane.js";
import { ALPHA_EDITOR_ID, alphaLanguageForInput } from "./editorInput.js";
import { type ITextResourceStore } from "../common/services/textResourceStore.js";
import { EditorSession, type EditorSessionOptions } from "./editorSession.js";
import { type ITextModelService, type TextModelReference } from "../common/services/textModelService.js";
import { type EditorTextDirection } from "./view/editorViewport.js";
import { type EditorLineWrapping } from "./view/visualLineProjection.js";
import { type IWorkingCopy, type IWorkingCopyService } from "../../../workbench/services/workingCopy/common/workingCopyService.js";
import { type ISyntaxApi } from "../../../platform/syntax/common/syntaxApi.js";

export interface EditorPaneSession extends IDisposable {
  layout(dimension: IDimension): void;
  focus(): void;
  getValue(): string;
  readonly isDirty?: boolean;
  readonly hasExternalChange?: boolean;
  save?(): Promise<void>;
  revert?(): Promise<void>;
}

export interface EditorPaneSessionOptions extends EditorSessionOptions {
  readonly textMateService?: ITextMateService;
  readonly languageFeaturesService?: ILanguageFeaturesService;
  readonly syntaxApi?: ISyntaxApi;
}

export interface EditorPaneOptions {
  readonly modelService: ITextModelService;
  readonly workingCopyService?: IWorkingCopyService;
  readonly createSession?: (options: EditorPaneSessionOptions) => EditorPaneSession;
  readonly textMateService?: ITextMateService;
  readonly languageFeaturesService?: ILanguageFeaturesService;
  readonly syntaxApi?: ISyntaxApi;
  readonly lineWrapping?: EditorLineWrapping;
  /** Browser paragraph direction forwarded to every created Alpha session. */
  readonly textDirection?: EditorTextDirection;
  readonly onOpenLink?: (target: string) => void | Promise<void>;
  readonly onShowContextMenu?: EditorSessionOptions["onShowContextMenu"];
  readonly onExecuteEditorCommand?: EditorSessionOptions["onExecuteEditorCommand"];
  readonly placeholder?: string;
  readonly showUnicodeHighlights?: boolean;
  readonly fontZoom?: EditorSessionOptions["fontZoom"];
  readonly onSave?: () => Promise<void | boolean>;
}

/** Workbench pane that composes Alpha's native model, input, view, and language services. */
export class EditorPane extends DisposableOwner implements IEditorPane {
  readonly id = ALPHA_EDITOR_ID;
  private readonly sessions = this.own(new DisposableSlot<EditorPaneSession>());
  private readonly workingCopySlot = this.own(new DisposableSlot<IWorkingCopy>());
  private readonly modelService: ITextModelService;
  private readonly createSession: (options: EditorPaneSessionOptions) => EditorPaneSession;
  private container: HTMLDivElement | undefined;
  private dimension: IDimension = { width: 0, height: 0 };

  get workingCopy(): IWorkingCopy | undefined {
    return this.workingCopySlot.value;
  }

  constructor(private readonly resourceStore: ITextResourceStore, private readonly options: EditorPaneOptions) {
    super();
    if (!resourceStore || typeof resourceStore.resolve !== "function" || typeof resourceStore.save !== "function" || typeof resourceStore.onDidChange !== "function") {
      this.dispose();
      throw new TypeError("Alpha editor pane requires an Alpha text resource store");
    }
    if (!options || !options.modelService || typeof options.modelService.acquire !== "function") {
      this.dispose();
      throw new TypeError("Alpha editor pane requires an Alpha text model service");
    }
    this.modelService = options.modelService;
    this.createSession = options.createSession ?? (sessionOptions => new EditorSession(sessionOptions));
  }

  create(parent: HTMLElement): void {
    if (this.container) throw new ReferenceError("EditorPane has already been created");
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
    const modelReference = await this.modelService.acquire(input, signal);
    let session: EditorPaneSession | undefined;
    try {
      throwIfCancelled(signal, "Alpha editor input loading was cancelled");
      session = this.createSession({
        container,
        input,
        languageId: alphaLanguageForInput(input, this.options.languageFeaturesService),
        modelReference,
        textMateService: this.options.textMateService,
        languageFeaturesService: this.options.languageFeaturesService,
        syntaxApi: this.options.syntaxApi,
        lineWrapping: this.options.lineWrapping,
        textDirection: this.options.textDirection,
        onOpenLink: this.options.onOpenLink,
        onShowContextMenu: this.options.onShowContextMenu,
        onExecuteEditorCommand: this.options.onExecuteEditorCommand,
        placeholder: this.options.placeholder,
        showUnicodeHighlights: this.options.showUnicodeHighlights,
        fontZoom: this.options.fontZoom,
        onSave: input.resource.scheme === "untitled"
          ? this.options.onSave
          : () => modelReference.save(new AbortController().signal),
        onRevert: () => modelReference.revert(new AbortController().signal),
      });
      throwIfCancelled(signal, "Alpha editor input loading was cancelled");
    } catch (error) {
      session?.dispose();
      if (!session) modelReference.dispose();
      throw error;
    }
    this.workingCopySlot.clear();
    this.sessions.replace(session);
    this.workingCopySlot.replace(new EditorWorkingCopy(
      modelReference,
      this.resourceStore,
      input.resource,
      this.options.workingCopyService,
      input.resource.scheme === "untitled" ? this.options.onSave : undefined,
    ));
    session.layout(this.dimension);
  }

  clearInput(): void {
    this.workingCopySlot.clear();
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

  async saveAs(resource: URI): Promise<void> {
    const workingCopy = this.workingCopy;
    if (workingCopy) {
      await workingCopy.saveAs(resource, new AbortController().signal);
      return;
    }
    await this.resourceStore.save({ resource, text: this.getValue() }, new AbortController().signal);
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
    assertDefined(this.container, new ReferenceError("EditorPane has not been created"));
    return this.container;
  }
}

class EditorWorkingCopy extends DisposableOwner implements IWorkingCopy {
  readonly resource: URI;
  readonly onDidChangeDirty: IWorkingCopy["onDidChangeDirty"];
  readonly onDidChangeExternalChange: IWorkingCopy["onDidChangeExternalChange"];

  constructor(
    private readonly reference: TextModelReference,
    private readonly resourceStore: ITextResourceStore,
    resource: URI,
    workingCopyService: IWorkingCopyService | undefined,
    private readonly saveUntitled: (() => Promise<void | boolean>) | undefined,
  ) {
    super();
    this.resource = resource;
    this.onDidChangeDirty = reference.onDidChangeDirty;
    this.onDidChangeExternalChange = reference.onDidChangeExternalChange;
    if (workingCopyService) this.own(workingCopyService.register(this));
  }

  get isDirty(): boolean {
    return this.reference.isDirty;
  }

  get hasExternalChange(): boolean {
    return this.reference.hasExternalChange;
  }

  save(signal: AbortSignal): Promise<void> {
    throwIfCancelled(signal, "Alpha working-copy save was cancelled");
    if (this.resource.scheme === "untitled") return this.saveUntitledDocument();
    return this.reference.save(signal);
  }

  saveAs(resource: URI, signal: AbortSignal): Promise<void> {
    return this.resourceStore.save({ resource, text: this.reference.model.getText() }, signal);
  }

  revert(signal: AbortSignal): Promise<void> {
    return this.reference.revert(signal);
  }

  private async saveUntitledDocument(): Promise<void> {
    const result = await this.saveUntitled?.();
    if (result === false) return;
    if (!this.saveUntitled) throw new Error("Untitled Alpha editor has no save handler");
  }
}
