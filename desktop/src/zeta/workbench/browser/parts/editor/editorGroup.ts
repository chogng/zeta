import { DragAndDropObserver } from "../../../../base/browser/dnd.js";
import { DndCssClasses } from "../../../../base/browser/ui/dnd/dnd.js";
import { addDisposableListener } from "../../../../base/browser/dom.js";
import { Dimension, type IDimension } from "../../../../base/browser/geometry.js";
import { DisposableOwner, setDisposableOwner } from "../../../../base/common/lifecycle.js";
import type { URI } from "../../../../base/common/uri.js";
import type { IKeybindingService } from "../../../../platform/keybinding/common/keybinding.js";
import type { IConfigurationService } from "../../../../platform/configuration/common/configurationService.js";
import { type ITextFileService } from "../../../services/textfile/common/textFileService.js";
import type { IFileService } from "../../../../platform/files/common/files.js";
import { type ITextMateService } from "../../../services/textMate/common/textMateService.js";
import { type ILanguageFeaturesService } from "../../../services/language/common/languageFeaturesService.js";
import type { IWorkingCopyService } from "../../../services/workingCopy/common/workingCopyService.js";
import type { IDiffApi } from "../../../../platform/diff/common/diffApi.js";
import type { ISyntaxApi } from "../../../../platform/syntax/common/syntaxApi.js";
import type { IDocumentCollaborationApi } from "../../../../platform/collaboration/common/documentCollaborationApi.js";
import type { IServerEventApi } from "../../../../platform/app-server/common/appServerApi.js";
import type { EditorInput, EditorOpenOptions } from "./editorInput.js";
import type { TextResourceLanguageResolver } from "../../../../platform/language/common/textResourceLanguage.js";
import { type IEditorPane, EditorPaneVisibility } from "./editorPane.js";
import { extractExternalEditorInputs } from "./editorDropData.js";
import { EditorPaneRegistry } from "./editorRegistry.js";
import type { IEditorTabDragAndDrop, EditorTabDropPosition } from "./editorTabDragAndDrop.js";
import { EditorGroupWatermark } from "./editorGroupWatermark.js";
import { editorInputKey, type EditorTabDescriptor } from "./editorTabsControl.js";
import { EditorTitleControl, type EditorTitleActions } from "./editorTitleControl.js";
import type { LanguageLocation } from "../../../../editor/contrib/gotoSymbol/common/languageNavigation.js";
import type { LanguageWorkspaceEdit } from "../../../../editor/common/languages/languageWorkspaceEdit.js";
import type { ILanguageDiagnosticsService } from "../../../../editor/common/services/languageDiagnosticsService.js";
import type { EditorLineGutterDecoration } from "../../../../editor/browser/view/lineGutterDecoration.js";

/** Operations and state owned independently by one EditorGroup. */
export interface IEditorGroup {
  readonly element: HTMLElement;
  readonly inputs: readonly EditorInput[];
  readonly activeInput: EditorInput | undefined;
  readonly activePane: IEditorPane | undefined;

  openEditor(
    input: EditorInput,
    options?: EditorOpenOptions,
  ): Promise<IEditorPane>;
  activateEditor(input: EditorInput): IEditorPane;
  closeEditor(input: EditorInput): void;
  replaceEditor(input: EditorInput, replacement: EditorInput): Promise<void>;
  setContent(content: Element): void;
  layout(dimension: IDimension): void;
  focus(): void;
}

/** Construction inputs for one independently navigable EditorGroup. */
export interface EditorGroupOptions {
  readonly ownerDocument: Document;
  readonly registry: EditorPaneRegistry;
  readonly configurationService?: IConfigurationService;
  readonly keybindingService?: IKeybindingService;
  readonly fileService?: IFileService;
  readonly textFileService?: ITextFileService;
  readonly textMateService?: ITextMateService;
  readonly languageFeaturesService?: ILanguageFeaturesService;
  readonly languageResolver?: TextResourceLanguageResolver;
  readonly diffApi?: IDiffApi;
  readonly syntaxApi?: ISyntaxApi;
  readonly languageDiagnosticsService?: ILanguageDiagnosticsService;
  readonly documentCollaborationApi?: IDocumentCollaborationApi;
  readonly serverEvents?: IServerEventApi;
  readonly workingCopyService?: IWorkingCopyService;
  readonly onSave?: (group: IEditorGroup, input: EditorInput, pane: IEditorPane) => Promise<boolean>;
  readonly onOpenLocation?: (location: LanguageLocation) => void | Promise<void>;
  readonly onApplyWorkspaceEdit?: (edit: LanguageWorkspaceEdit) => void | Promise<void>;
  readonly createLineGutterDecorations?: (resource: URI) => readonly EditorLineGutterDecoration[];
  readonly titleActions?: EditorTitleActions;
  readonly onDidActivate?: () => void;
  readonly dragAndDrop?: IEditorTabDragAndDrop;
}

interface EditorGroupEntry extends EditorTabDescriptor {
  paneInstance: EditorPaneInstance;
  input: EditorInput;
}

/**
 * Owns an ordered set of Editor inputs, their Pane lifetimes, and title UI.
 *
 * EditorPart owns group layout. This class owns only the behavior that remains
 * independent when the Part later contains multiple split groups.
 */
export class EditorGroup extends DisposableOwner implements IEditorGroup {
  readonly element: HTMLElement;
  private readonly contentElement: HTMLDivElement;
  private readonly registry: EditorPaneRegistry;
  private readonly configurationService: IConfigurationService | undefined;
  private readonly fileService: IFileService | undefined;
  private readonly textFileService: ITextFileService | undefined;
  private readonly textMateService: ITextMateService | undefined;
  private readonly languageFeaturesService: ILanguageFeaturesService | undefined;
  private readonly languageResolver: TextResourceLanguageResolver | undefined;
  private readonly diffApi: IDiffApi | undefined;
  private readonly syntaxApi: ISyntaxApi | undefined;
  private readonly languageDiagnosticsService: ILanguageDiagnosticsService | undefined;
  private readonly documentCollaborationApi: IDocumentCollaborationApi | undefined;
  private readonly serverEvents: IServerEventApi | undefined;
  private readonly workingCopyService: IWorkingCopyService | undefined;
  private readonly onSave: ((group: IEditorGroup, input: EditorInput, pane: IEditorPane) => Promise<boolean>) | undefined;
  private readonly onOpenLocation: ((location: LanguageLocation) => void | Promise<void>) | undefined;
  private readonly onApplyWorkspaceEdit: ((edit: LanguageWorkspaceEdit) => void | Promise<void>) | undefined;
  private readonly createLineGutterDecorations: ((resource: URI) => readonly EditorLineGutterDecoration[]) | undefined;
  private readonly titleControl: EditorTitleControl;
  private readonly watermarkElement: HTMLElement;
  private readonly entries: EditorGroupEntry[] = [];
  private activeEntry: EditorGroupEntry | undefined;
  private ordinaryContent: Element | undefined;
  private dimension: IDimension = Dimension.Zero;
  private openSequence = 0;
  private pendingPane: EditorPaneInstance | undefined;

  constructor(options: EditorGroupOptions) {
    super();
    this.registry = options.registry;
    this.configurationService = options.configurationService;
    this.fileService = options.fileService;
    this.textFileService = options.textFileService;
    this.textMateService = options.textMateService;
    this.languageFeaturesService = options.languageFeaturesService;
    this.languageResolver = options.languageResolver;
    this.diffApi = options.diffApi;
    this.syntaxApi = options.syntaxApi;
    this.languageDiagnosticsService = options.languageDiagnosticsService;
    this.documentCollaborationApi = options.documentCollaborationApi;
    this.serverEvents = options.serverEvents;
    this.workingCopyService = options.workingCopyService;
    this.onSave = options.onSave;
    this.onOpenLocation = options.onOpenLocation;
    this.onApplyWorkspaceEdit = options.onApplyWorkspaceEdit;
    this.createLineGutterDecorations = options.createLineGutterDecorations;
    this.element = options.ownerDocument.createElement("section");
    this.element.className = "zeta-editor-group";
    this.element.setAttribute("aria-label", "Editor group");
    this.own(new DragAndDropObserver(this.element, {
      onDragOver: (event) => {
        if (!options.dragAndDrop?.isDragging() || this.dragIsOverTitle(event)) return;
        if (event.dataTransfer) event.dataTransfer.dropEffect = "move";
        this.element.classList.add(DndCssClasses.DropTarget);
      },
      onDragLeave: () => this.element.classList.remove(DndCssClasses.DropTarget),
      onDrop: (event) => {
        if (!options.dragAndDrop?.isDragging() || this.dragIsOverTitle(event)) return;
        event.stopPropagation();
        this.element.classList.remove(DndCssClasses.DropTarget);
        options.dragAndDrop.drop(this, undefined, "after");
        options.dragAndDrop.end();
      },
      onDragEnd: () => this.element.classList.remove(DndCssClasses.DropTarget),
    }));
    if (options.onDidActivate) {
      this.own(addDisposableListener(this.element, "focusin", () => {
        options.onDidActivate?.();
      }));
    }
    this.titleControl = this.own(new EditorTitleControl(
      options.ownerDocument,
      {
        activate: (input) => {
          this.activateEntry(this.requireEntry(input), true);
        },
        preview: (input) => this.activateEntry(this.requireEntry(input), false),
        close: (input) => this.closeEditor(input),
        startDrag: (input) => options.dragAndDrop?.start(this, input),
        isDragging: () => options.dragAndDrop?.isDragging() ?? false,
        drop: (target, position) => options.dragAndDrop?.drop(this, target, position),
        dropExternal: (event, target, position) => {
          void this.openExternalEditors(event.dataTransfer, target, position).catch((error: unknown) => {
            console.error("Failed to open dropped editor resources", error);
          });
        },
        endDrag: () => options.dragAndDrop?.end(),
      },
      options.titleActions,
    ));
    this.contentElement = options.ownerDocument.createElement("div");
    this.contentElement.className = "zeta-editor-group-content";
    const watermark = options.keybindingService
      ? this.own(new EditorGroupWatermark(
        options.ownerDocument,
        options.keybindingService,
      ))
      : undefined;
    this.watermarkElement = watermark?.element ??
      options.ownerDocument.createElement("div");
    this.watermarkElement.classList.add("zeta-editor-group-watermark");
    this.contentElement.append(this.watermarkElement);
    this.element.append(
      this.titleControl.element,
      this.contentElement,
    );
    this.defer(() => {
      this.cancelPendingOpen();
      for (const entry of this.entries) entry.paneInstance.dispose();
      this.entries.length = 0;
    });
    this.defer(() => this.element.remove());
    this.renderChrome();
  }

  get inputs(): readonly EditorInput[] {
    return this.entries.map(({ input }) => input);
  }

  get activeInput(): EditorInput | undefined {
    return this.activeEntry?.input;
  }

  get activePane(): IEditorPane | undefined {
    return this.activeEntry?.paneInstance.pane;
  }

  async openEditor(
    input: EditorInput,
    options: EditorOpenOptions = {},
  ): Promise<IEditorPane> {
    const sequence = ++this.openSequence;
    this.cancelPendingOpen();
    const matchInput = this.languageResolver
      ? { ...input, languageId: this.languageResolver.resolveLanguageId({ resource: input.resource, ...(input.contentType === undefined ? {} : { contentType: input.contentType }) }) }
      : input;
    const descriptor = this.registry.resolve(matchInput, options);
    const existing = this.entry(input);
    if (existing?.paneInstance.pane.id === descriptor.id) {
      existing.input = input;
      this.moveEntry(existing, options.index);
      this.activateEntry(existing, false);
      applyEditorOpenOptions(existing.paneInstance.pane, options);
      return existing.paneInstance.pane;
    }

    let createdPane: IEditorPane | undefined;
    const pane = descriptor.create({
      ownerDocument: this.element.ownerDocument,
      input,
      configurationService: this.configurationService,
      fileService: this.fileService,
      textFileService: this.textFileService,
      textMateService: this.textMateService,
      languageFeaturesService: this.languageFeaturesService,
      diffApi: this.diffApi,
      syntaxApi: this.syntaxApi,
      languageDiagnosticsService: this.languageDiagnosticsService,
      documentCollaborationApi: this.documentCollaborationApi,
      serverEvents: this.serverEvents,
      workingCopyService: this.workingCopyService,
      onOpenLocation: this.onOpenLocation,
      onApplyWorkspaceEdit: this.onApplyWorkspaceEdit,
      createLineGutterDecorations: this.createLineGutterDecorations,
      ...(this.onSave ? {
        onSave: () => {
          if (!createdPane) return Promise.reject(new Error("Editor save is unavailable"));
          return this.onSave!(this, input, createdPane);
        },
      } : {}),
    });
    createdPane = pane;
    if (pane.id !== descriptor.id) {
      pane.dispose();
      throw new TypeError(
        `Editor pane factory '${descriptor.id}' created '${pane.id}'`,
      );
    }
    const paneInstance = new EditorPaneInstance(
      pane,
      this.element.ownerDocument,
    );
    setDisposableOwner(paneInstance, this);
    this.pendingPane = paneInstance;
    this.contentElement.append(paneInstance.element);
    try {
      pane.create(paneInstance.element);
      paneInstance.setVisible(EditorPaneVisibility.Hidden);
      await pane.setInput(input, paneInstance.signal);
    } catch (error) {
      if (this.pendingPane === paneInstance) {
        this.pendingPane = undefined;
      }
      paneInstance.dispose();
      if (sequence !== this.openSequence) {
        throw new EditorOpenSupersededError(input);
      }
      throw error;
    }

    if (
      sequence !== this.openSequence ||
      this.pendingPane !== paneInstance
    ) {
      paneInstance.dispose();
      throw new EditorOpenSupersededError(input);
    }
    this.pendingPane = undefined;

    let entry: EditorGroupEntry = {
      input,
      panelId: paneInstance.panelId,
      tabId: paneInstance.tabId,
      paneInstance,
    };
    if (existing) {
      const index = this.entries.indexOf(existing);
      existing.paneInstance.setVisible(EditorPaneVisibility.Hidden);
      existing.paneInstance.dispose();
      if (this.activeEntry === existing) this.activeEntry = undefined;
      this.entries[index] = entry;
    } else {
      this.insertEntry(entry, options.index);
    }
    this.ordinaryContent = undefined;
    this.activateEntry(entry, false);
    applyEditorOpenOptions(pane, options);
    return pane;
  }

  activateEditor(input: EditorInput): IEditorPane {
    const entry = this.requireEntry(input);
    this.activateEntry(entry, false);
    return entry.paneInstance.pane;
  }

  closeEditor(input: EditorInput): void {
    const index = this.entries.findIndex(
      (candidate) => editorInputKey(candidate.input) === editorInputKey(input),
    );
    if (index < 0) return;
    const [entry] = this.entries.splice(index, 1);
    if (!entry) return;
    const wasActive = this.activeEntry === entry;
    if (wasActive) {
      this.activeEntry = undefined;
      entry.paneInstance.setVisible(EditorPaneVisibility.Hidden);
    }
    entry.paneInstance.dispose();
    if (wasActive) {
      const next = this.entries[index] ?? this.entries[index - 1];
      if (next) this.activateEntry(next, true);
    }
    this.renderContent();
    this.renderChrome();
  }

  async replaceEditor(input: EditorInput, replacement: EditorInput): Promise<void> {
    const index = this.entries.findIndex(
      (candidate) => editorInputKey(candidate.input) === editorInputKey(input),
    );
    if (index < 0) throw new RangeError(`Editor is not open in this group: ${input.resource}`);
    await this.openEditor(replacement, { index });
    this.closeEditor(input);
  }

  getEditorInsertionIndex(target: EditorInput | undefined, position: EditorTabDropPosition): number {
    if (!target) return this.entries.length;
    const index = this.entries.findIndex(
      (candidate) => editorInputKey(candidate.input) === editorInputKey(target),
    );
    if (index < 0) return this.entries.length;
    return position === "before" ? index : index + 1;
  }

  moveEditor(input: EditorInput, targetIndex: number): void {
    const sourceIndex = this.entries.findIndex(
      (candidate) => editorInputKey(candidate.input) === editorInputKey(input),
    );
    if (sourceIndex < 0) return;
    const [entry] = this.entries.splice(sourceIndex, 1);
    if (!entry) return;
    const adjustedIndex = Math.min(
      Math.max(0, targetIndex > sourceIndex ? targetIndex - 1 : targetIndex),
      this.entries.length,
    );
    this.entries.splice(adjustedIndex, 0, entry);
    this.renderContent();
    this.renderChrome();
  }

  private async openExternalEditors(dataTransfer: DataTransfer | null, target: EditorInput | undefined, position: EditorTabDropPosition): Promise<void> {
    if (!dataTransfer) return;
    const inputs = await extractExternalEditorInputs(dataTransfer);
    let index = this.getEditorInsertionIndex(target, position);
    for (const input of inputs) {
      await this.openEditor(input, { index });
      index += 1;
    }
  }

  async moveEditorTo(input: EditorInput, target: EditorGroup, targetIndex: number): Promise<void> {
    if (target === this) {
      this.moveEditor(input, targetIndex);
      return;
    }
    this.requireEntry(input);
    await target.openEditor(input, { index: targetIndex });
    this.closeEditor(input);
    target.activateEditor(input);
  }

  setContent(content: Element): void {
    this.openSequence += 1;
    this.cancelPendingOpen();
    for (const entry of this.entries) {
      entry.paneInstance.setVisible(EditorPaneVisibility.Hidden);
      entry.paneInstance.dispose();
    }
    this.entries.length = 0;
    this.activeEntry = undefined;
    this.ordinaryContent = content;
    this.renderContent();
    this.renderChrome();
  }

  layout(dimension: IDimension): void {
    this.dimension = new Dimension(
      dimension.width,
      Math.max(0, dimension.height - EditorTitleControl.HEIGHT),
    );
    this.activePane?.layout(this.dimension);
  }

  focus(): void {
    this.activePane?.focus();
  }

  private activateEntry(entry: EditorGroupEntry, focus: boolean): void {
    if (this.activeEntry !== entry) {
      this.activeEntry?.paneInstance.setVisible(EditorPaneVisibility.Hidden);
      this.activeEntry = entry;
    }
    this.ordinaryContent = undefined;
    this.renderContent();
    entry.paneInstance.pane.layout(this.dimension);
    entry.paneInstance.setVisible(EditorPaneVisibility.Visible);
    this.renderChrome();
    if (focus) entry.paneInstance.pane.focus();
  }

  private renderContent(): void {
    const children: Element[] = [];
    if (this.ordinaryContent) {
      children.push(this.ordinaryContent);
    } else {
      this.watermarkElement.hidden = this.entries.length > 0;
      children.push(
        this.watermarkElement,
        ...this.entries.map(({ paneInstance }) => paneInstance.element),
      );
    }
    if (this.pendingPane) children.push(this.pendingPane.element);
    this.contentElement.replaceChildren(...children);
  }

  private renderChrome(): void {
    this.titleControl.setEditors(this.entries, this.activeInput);
  }

  private dragIsOverTitle(event: DragEvent): boolean {
    const target = event.target as Node | null;
    return target ? this.titleControl.element.contains(target) : false;
  }

  private insertEntry(entry: EditorGroupEntry, index: number | undefined): void {
    const targetIndex = index === undefined
      ? this.entries.length
      : Math.min(Math.max(0, index), this.entries.length);
    this.entries.splice(targetIndex, 0, entry);
  }

  private moveEntry(entry: EditorGroupEntry, index: number | undefined): void {
    if (index === undefined) return;
    const currentIndex = this.entries.indexOf(entry);
    if (currentIndex < 0) return;
    this.entries.splice(currentIndex, 1);
    const targetIndex = Math.min(Math.max(0, index), this.entries.length);
    this.entries.splice(targetIndex, 0, entry);
  }

  private entry(input: EditorInput): EditorGroupEntry | undefined {
    const key = editorInputKey(input);
    return this.entries.find(
      (candidate) => editorInputKey(candidate.input) === key,
    );
  }

  private requireEntry(input: EditorInput): EditorGroupEntry {
    const entry = this.entry(input);
    if (!entry) {
      throw new RangeError(
        `Editor is not open in this group: ${input.resource}`,
      );
    }
    return entry;
  }

  private cancelPendingOpen(): void {
    const pending = this.pendingPane;
    this.pendingPane = undefined;
    pending?.dispose();
  }
}

function applyEditorOpenOptions(pane: IEditorPane, options: EditorOpenOptions): void {
  if (options.selection) pane.revealRange?.(options.selection);
}

let editorPaneId = 0;

class EditorPaneInstance extends DisposableOwner {
  readonly element: HTMLDivElement;
  readonly signal: AbortSignal;
  readonly panelId: string;
  readonly tabId: string;

  constructor(
    readonly pane: IEditorPane,
    ownerDocument: Document,
  ) {
    super();
    const id = ++editorPaneId;
    this.panelId = `zeta-editor-pane-${id}`;
    this.tabId = `zeta-editor-tab-${id}`;
    const AbortControllerConstructor =
      ownerDocument.defaultView?.AbortController ?? AbortController;
    const abortController = new AbortControllerConstructor();
    this.signal = abortController.signal;
    this.element = ownerDocument.createElement("div");
    this.element.id = this.panelId;
    this.element.className = "zeta-editor-pane-host";
    this.element.setAttribute("role", "tabpanel");
    this.element.setAttribute("aria-labelledby", this.tabId);
    this.defer(() => this.element.remove());
    this.own(pane);
    this.defer(() => pane.clearInput());
    this.defer(() => pane.setVisible(EditorPaneVisibility.Hidden));
    this.defer(() => abortController.abort());
  }

  setVisible(visibility: EditorPaneVisibility): void {
    this.element.hidden = visibility === EditorPaneVisibility.Hidden;
    this.pane.setVisible(visibility);
  }
}

export class EditorOpenSupersededError extends Error {
  constructor(readonly input: EditorInput) {
    super(`Editor opening was superseded: ${input.resource}`);
    this.name = "EditorOpenSupersededError";
  }
}
