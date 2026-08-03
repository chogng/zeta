import "./media/alphaDiffEditorPane.css";
import { type IDimension } from "../../../base/browser/geometry.js";
import { throwIfCancelled } from "../../../base/common/cancellation.js";
import { DisposableOwner, DisposableSlot } from "../../../base/common/lifecycle.js";
import { assertDefined } from "../../../base/common/types.js";
import { type IEditorPane } from "../../../workbench/browser/parts/editor/editorPane.js";
import { EditorPaneVisibility } from "../../../workbench/browser/parts/editor/editorPane.js";
import { type EditorInput } from "../../../workbench/browser/parts/editor/editorInput.js";
import { type ITextFileService } from "../../../workbench/services/textfile/common/textFileService.js";
import { isAlphaDiffEditorInput, ALPHA_DIFF_EDITOR_ID } from "../common/alphaDiffEditorInput.js";
import { DiffModel } from "../common/models/diff/diffModel.js";
import { BrowserDiffComputationService } from "./diff/browserDiffComputationService.js";
import { DiffEditorWidget } from "./widget/diffEditor/diffEditorWidget.js";
import { AlphaTextModels, type AlphaTextModelReference, type AlphaTextModelService } from "./alphaTextModelService.js";

export interface AlphaDiffEditorPaneOptions {
  readonly modelService?: AlphaTextModelService;
}

/** Workbench pane that acquires two Alpha text references for a read-only comparison. */
export class AlphaDiffEditorPane extends DisposableOwner implements IEditorPane {
  readonly id = ALPHA_DIFF_EDITOR_ID;
  private readonly session = this.own(new DisposableSlot<AlphaDiffEditorPaneSession>());
  private readonly modelService: AlphaTextModelService;
  private container: HTMLDivElement | undefined;
  private dimension: IDimension = { width: 0, height: 0 };

  constructor(private readonly textFiles: ITextFileService, options: AlphaDiffEditorPaneOptions = {}) {
    super();
    if (!textFiles || typeof textFiles.resolve !== "function") {
      this.dispose();
      throw new TypeError("Alpha diff editor pane requires a text file service");
    }
    this.modelService = options.modelService ?? AlphaTextModels;
  }

  create(parent: HTMLElement): void {
    if (this.container) throw new ReferenceError("AlphaDiffEditorPane has already been created");
    const container = parent.ownerDocument.createElement("div");
    container.className = "zeta-alpha-diff-editor-pane";
    parent.append(container);
    this.container = container;
    this.defer(() => {
      container.remove();
      this.container = undefined;
    });
  }

  async setInput(input: EditorInput, signal: AbortSignal): Promise<void> {
    if (!isAlphaDiffEditorInput(input)) {
      throw new TypeError("Alpha diff editor pane requires an Alpha diff editor input");
    }
    const container = this.requireContainer();
    throwIfCancelled(signal, "Alpha diff editor input loading was cancelled");
    const original = await this.modelService.acquire(input.original, this.textFiles, signal);
    let modified: AlphaTextModelReference | undefined;
    let next: AlphaDiffEditorPaneSession | undefined;
    try {
      throwIfCancelled(signal, "Alpha diff editor input loading was cancelled");
      modified = await this.modelService.acquire(input.modified, this.textFiles, signal);
      throwIfCancelled(signal, "Alpha diff editor input loading was cancelled");
      next = new AlphaDiffEditorPaneSession(container, original, modified, input.original.label, input.modified.label);
      throwIfCancelled(signal, "Alpha diff editor input loading was cancelled");
    } catch (error) {
      next?.dispose();
      if (!next) {
        modified?.dispose();
        original.dispose();
      }
      throw error;
    }
    this.session.replace(next);
    next.layout(this.dimension);
  }

  clearInput(): void {
    this.session.clear();
  }

  layout(dimension: IDimension): void {
    this.dimension = { width: Math.max(0, dimension.width), height: Math.max(0, dimension.height) };
    this.session.value?.layout(this.dimension);
  }

  setVisible(visibility: EditorPaneVisibility): void {
    if (!this.container) return;
    this.container.hidden = visibility === EditorPaneVisibility.Hidden;
    if (visibility === EditorPaneVisibility.Visible) this.session.value?.layout(this.dimension);
  }

  focus(): void {
    this.session.value?.focus();
  }

  private requireContainer(): HTMLDivElement {
    assertDefined(this.container, new ReferenceError("Alpha diff editor pane has not been created"));
    return this.container;
  }
}

class AlphaDiffEditorPaneSession extends DisposableOwner {
  readonly editor: DiffEditorWidget;

  constructor(container: HTMLElement, original: AlphaTextModelReference, modified: AlphaTextModelReference, originalLabel: string | undefined, modifiedLabel: string | undefined) {
    super();
    this.own(original);
    this.own(modified);
    const computationService = this.own(new BrowserDiffComputationService());
    const model = this.own(new DiffModel({
      original: original.model,
      modified: modified.model,
      computationService,
    }));
    this.editor = this.own(new DiffEditorWidget({
      container,
      model,
      originalAriaLabel: originalLabel,
      modifiedAriaLabel: modifiedLabel,
    }));
  }

  layout(dimension: IDimension): void {
    this.editor.layout(dimension);
  }

  focus(): void {
    this.editor.element.focus({ preventScroll: true });
  }
}
