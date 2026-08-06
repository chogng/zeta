import { assertDefined } from "../../../base/common/types.js";
import { DisposableOwner } from "../../../base/common/lifecycle.js";
import type { IDimension } from "../../../base/browser/geometry.js";
import type { ITextFileService } from "../../../workbench/services/textfile/common/textFileService.js";
import { EditorPaneVisibility, type IEditorPane } from "../../../workbench/browser/parts/editor/editorPane.js";
import type { EditorInput } from "../../../workbench/browser/parts/editor/editorInput.js";
import type { URI } from "../../../base/common/uri.js";
import type { IWorkingCopy } from "../../../workbench/services/workingCopy/common/workingCopyService.js";
import type { IWorkingCopyService } from "../../../workbench/services/workingCopy/common/workingCopyService.js";
import { BrowserDocumentModelService } from "./services/browserDocumentModelService.js";
import type { DocumentNode } from "../common/model/document.js";
import type { DocumentOutline } from "../common/model/documentOutline.js";
import { GAMA_EDITOR_ID } from "./editorInput.js";
import { GamaEditorSession, type GamaEditorSessionOptions } from "./gamaEditorSession.js";

/** Workbench-only services that complement one Gama editor session. */
export interface GamaEditorPaneOptions extends GamaEditorSessionOptions {
  readonly workingCopyService?: IWorkingCopyService;
}

/** Workbench pane that hosts one Gama structured-editor session. */
export class GamaEditorPane extends DisposableOwner implements IEditorPane {
  readonly id = GAMA_EDITOR_ID;

  private readonly session: GamaEditorSession;
  private container: HTMLDivElement | undefined;
  private dimension: IDimension = { width: 0, height: 0 };

  get workingCopy(): IWorkingCopy | undefined {
    return this.session.workingCopy;
  }

  constructor(textFiles: ITextFileService, options: GamaEditorPaneOptions = {}) {
    super();
    const modelService = this.own(new BrowserDocumentModelService(textFiles, options.workingCopyService));
    this.session = this.own(new GamaEditorSession(modelService, options));
  }

  create(parent: HTMLElement): void {
    if (this.container) throw new ReferenceError("Gama editor pane has already been created");
    const container = parent.ownerDocument.createElement("div");
    container.className = "zeta-gama-editor-pane";
    parent.append(container);
    this.container = container;
    this.session.create(container);
  }

  async setInput(input: EditorInput, signal: AbortSignal): Promise<void> {
    this.requireContainer();
    await this.session.setInput(input, signal);
    this.session.layout(this.dimension);
  }

  clearInput(): void {
    this.session.clearInput();
  }

  layout(dimension: IDimension): void {
    this.dimension = { width: Math.max(0, dimension.width), height: Math.max(0, dimension.height) };
    this.session.layout(this.dimension);
  }

  setVisible(visibility: EditorPaneVisibility): void {
    if (this.container) this.container.hidden = visibility === EditorPaneVisibility.Hidden;
    this.session.setVisible(visibility);
  }

  focus(): void {
    this.session.focus();
  }

  async save(): Promise<void> {
    await this.session.save();
  }

  async saveAs(resource: URI): Promise<void> {
    await this.session.saveAs(resource);
  }

  async revert(): Promise<void> {
    await this.session.revert();
  }

  get isDirty(): boolean {
    return this.session.isDirty;
  }

  get hasExternalChange(): boolean {
    return this.session.hasExternalChange;
  }

  getDocument(): DocumentNode {
    return this.session.getDocument();
  }

  getOutline(): DocumentOutline {
    return this.session.getOutline();
  }

  override dispose(): void {
    this.container?.remove();
    this.container = undefined;
    super.dispose();
  }

  [Symbol.dispose](): void {
    this.dispose();
  }

  private requireContainer(): HTMLDivElement {
    assertDefined(this.container, new ReferenceError("Gama editor pane has not been created"));
    return this.container;
  }
}
