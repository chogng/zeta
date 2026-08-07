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
import type { DocumentSelection } from "../common/core/documentSelection.js";
import type { DocumentOutline } from "../common/model/documentOutline.js";
import { GAMA_EDITOR_ID } from "./editorInput.js";
import { EditorWidget, type EditorWidgetOptions } from "./editorWidget.js";

/** Workbench-only services that complement one Gama editor. */
export interface EditorPaneOptions extends EditorWidgetOptions {
  readonly workingCopyService?: IWorkingCopyService;
}

/** Workbench pane that hosts one Gama structured editor. */
export class EditorPane extends DisposableOwner implements IEditorPane {
  readonly id = GAMA_EDITOR_ID;

  private readonly editor: EditorWidget;
  private container: HTMLDivElement | undefined;
  private dimension: IDimension = { width: 0, height: 0 };

  get workingCopy(): IWorkingCopy | undefined {
    return this.editor.workingCopy;
  }

  constructor(textFiles: ITextFileService, options: EditorPaneOptions = {}) {
    super();
    const modelService = this.own(new BrowserDocumentModelService(textFiles, options.workingCopyService));
    const collaborationService = options.documentCollaborationService ? this.own(options.documentCollaborationService) : undefined;
    this.editor = this.own(new EditorWidget(modelService, { ...options, ...(collaborationService ? { documentCollaborationService: collaborationService } : {}) }));
  }

  create(parent: HTMLElement): void {
    if (this.container) throw new ReferenceError("Gama editor pane has already been created");
    const container = parent.ownerDocument.createElement("div");
    container.className = "zeta-structured-editor-pane";
    parent.append(container);
    this.container = container;
    this.editor.create(container);
  }

  async setInput(input: EditorInput, signal: AbortSignal): Promise<void> {
    this.requireContainer();
    await this.editor.setInput(input, signal);
    this.editor.layout(this.dimension);
  }

  clearInput(): void {
    this.editor.clearInput();
  }

  layout(dimension: IDimension): void {
    this.dimension = { width: Math.max(0, dimension.width), height: Math.max(0, dimension.height) };
    this.editor.layout(this.dimension);
  }

  setVisible(visibility: EditorPaneVisibility): void {
    if (this.container) this.container.hidden = visibility === EditorPaneVisibility.Hidden;
    this.editor.setVisible(visibility);
  }

  focus(): void {
    this.editor.focus();
  }

  async save(): Promise<void> {
    await this.editor.save();
  }

  async saveAs(resource: URI): Promise<void> {
    await this.editor.saveAs(resource);
  }

  async revert(): Promise<void> {
    await this.editor.revert();
  }

  get isDirty(): boolean {
    return this.editor.isDirty;
  }

  get hasExternalChange(): boolean {
    return this.editor.hasExternalChange;
  }

  getDocument(): DocumentNode {
    return this.editor.getDocument();
  }

  /** Returns the current structured-document selection of the hosted Gama editor. */
  getDocumentSelection(): DocumentSelection | undefined {
    return this.editor.getDocumentSelection();
  }

  getOutline(): DocumentOutline {
    return this.editor.getOutline();
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
