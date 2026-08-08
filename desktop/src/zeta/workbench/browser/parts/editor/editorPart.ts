import "./media/editorpart.css";
import type { IContextMenuProvider } from "../../../../base/browser/contextmenu.js";
import type { URI } from "../../../../base/common/uri.js";
import { Dimension, type IDimension } from "../../../../base/browser/geometry.js";
import { SplitView, type ISplitViewView } from "../../../../base/browser/ui/splitview/splitview.js";
import type { IMenuService } from "../../../../platform/actions/common/menuService.js";
import type { IConfigurationService } from "../../../../platform/configuration/common/configuration.js";
import { createServiceIdentifier } from "../../../../platform/instantiation/common/instantiation.js";
import type { IKeybindingService } from "../../../../platform/keybinding/common/keybinding.js";
import { type ITextFileService } from "../../../services/textfile/common/textFileService.js";
import type { IFileService } from "../../../../platform/files/common/files.js";
import { type ITextMateService } from "../../../services/textMate/common/textMateService.js";
import { type ILanguageFeaturesService } from "../../../services/language/common/languageFeaturesService.js";
import type { IDiffApi } from "../../../../platform/diff/common/diffApi.js";
import type { ISyntaxApi } from "../../../../platform/syntax/common/syntaxApi.js";
import type { IDocumentCollaborationApi } from "../../../../platform/collaboration/common/documentCollaborationApi.js";
import type { IServerEventApi } from "../../../../platform/app-server/common/appServerApi.js";
import { WorkbenchPart } from "../../part.js";
import { EditorGroup, type EditorGroupOptions, type IEditorGroup } from "./editorGroup.js";
import { EditorTabDragAndDropController, type EditorTabDropEvent } from "./editorTabDragAndDrop.js";
import type { EditorInput, EditorOpenOptions } from "./editorInput.js";
import type { TextResourceLanguageResolver } from "../../../../platform/language/common/textResourceLanguage.js";
import type { IWorkingCopyService } from "../../../services/workingCopy/common/workingCopyService.js";
import type { IEditorPane } from "./editorPane.js";
import { EditorPaneRegistry, EditorPanes } from "./editorRegistry.js";

export { EditorOpenSupersededError } from "./editorGroup.js";

/** Editor-region operations available to Workbench contributions. */
export interface IEditorPart {
  readonly element: HTMLElement;
  readonly groups: readonly IEditorGroup[];
  readonly activeGroup: IEditorGroup;
  readonly activeInput: EditorInput | undefined;
  readonly activePane: IEditorPane | undefined;

  openEditor(
    input: EditorInput,
    options?: EditorOpenOptions,
  ): Promise<IEditorPane>;
  activateEditor(input: EditorInput): IEditorPane;
  closeEditor(input: EditorInput): void;
  saveActiveEditor(): Promise<void>;
  setContent(content: Element): void;
  splitActiveGroupHorizontal(): Promise<void>;
  layout(dimension: IDimension): void;
  focus(): void;
}

export const IEditorPart =
  createServiceIdentifier<IEditorPart>("editorPart");

/** Named collaborators used to construct the editor region. */
export interface IEditorPartOptions {
  readonly configurationService?: IConfigurationService;
  readonly keybindingService?: IKeybindingService;
  readonly fileService?: IFileService;
  readonly textFileService?: ITextFileService;
  readonly textMateService?: ITextMateService;
  readonly languageFeaturesService?: ILanguageFeaturesService;
  readonly languageResolver?: TextResourceLanguageResolver;
  readonly diffApi?: IDiffApi;
  readonly syntaxApi?: ISyntaxApi;
  readonly documentCollaborationApi?: IDocumentCollaborationApi;
  readonly serverEvents?: IServerEventApi;
  readonly workingCopyService?: IWorkingCopyService;
  readonly registry?: EditorPaneRegistry;
  readonly titleActions?: {
    readonly menuService: IMenuService;
    readonly contextMenuProvider: IContextMenuProvider;
  };
  readonly saveAsResource?: (defaultName: string) => Promise<URI | undefined>;
}

/** Owns EditorGroup layout and delegates editor behavior to the active group. */
export class EditorPart extends WorkbenchPart implements IEditorPart {
  private readonly splitView: SplitView;
  private readonly groupOptions: Omit<EditorGroupOptions, "ownerDocument" | "onDidActivate" | "dragAndDrop">;
  private readonly _groups: EditorGroupHost[] = [];
  private _activeGroup: EditorGroup;
  private readonly tabDragAndDrop: EditorTabDragAndDropController;
  private dimension = Dimension.Zero;
  private readonly saveAsResource: ((defaultName: string) => Promise<URI | undefined>) | undefined;

  override get minimumWidth(): number { return 120; }
  override get minimumHeight(): number { return 119; }

  constructor(
    ownerDocument: Document,
    options: IEditorPartOptions = {},
  ) {
    super("editor", ownerDocument);
    this.titleElement.remove();
    this.element.setAttribute("aria-label", "Editor");
    this.groupOptions = {
      registry: options.registry ?? EditorPanes,
      configurationService: options.configurationService,
      keybindingService: options.keybindingService,
      fileService: options.fileService,
      textFileService: options.textFileService,
      textMateService: options.textMateService,
      languageFeaturesService: options.languageFeaturesService,
      languageResolver: options.languageResolver,
      diffApi: options.diffApi,
      syntaxApi: options.syntaxApi,
      documentCollaborationApi: options.documentCollaborationApi,
      serverEvents: options.serverEvents,
      workingCopyService: options.workingCopyService,
      titleActions: options.titleActions,
      ...(options.saveAsResource ? {
        onSave: (group: IEditorGroup, input: EditorInput, pane: IEditorPane) => this.saveEditor(group, input, pane),
      } : {}),
    };
    this.saveAsResource = options.saveAsResource;
    this.tabDragAndDrop = new EditorTabDragAndDropController((event) => {
      this.dropEditor(event);
    });
    this.splitView = this.own(new SplitView(
      "horizontal",
      ownerDocument,
    ));
    const initial = this.createGroup();
    this._groups.push(initial);
    this._activeGroup = initial.group;
    this.splitView.addView(initial.view);
    this.contentElement.append(this.splitView.element);
    const ResizeObserverConstructor =
      ownerDocument.defaultView?.ResizeObserver;
    if (ResizeObserverConstructor) {
      const observer = new ResizeObserverConstructor(([entry]) => {
        if (!entry) return;
        const borderBox = entry.borderBoxSize[0];
        this.layout({
          width: borderBox?.inlineSize ?? entry.contentRect.width,
          height: borderBox?.blockSize ?? entry.contentRect.height,
        });
      });
      observer.observe(this.contentElement, { box: "border-box" });
      this.defer(() => observer.disconnect());
    }
  }

  get groups(): readonly IEditorGroup[] {
    return this._groups.map(({ group }) => group);
  }

  get activeGroup(): IEditorGroup {
    return this._activeGroup;
  }

  get activeInput(): EditorInput | undefined {
    return this._activeGroup.activeInput;
  }

  get activePane(): IEditorPane | undefined {
    return this._activeGroup.activePane;
  }

  openEditor(
    input: EditorInput,
    options: EditorOpenOptions = {},
  ): Promise<IEditorPane> {
    return this._activeGroup.openEditor(input, options);
  }

  activateEditor(input: EditorInput): IEditorPane {
    return this._activeGroup.activateEditor(input);
  }

  closeEditor(input: EditorInput): void {
    this._activeGroup.closeEditor(input);
  }

  async saveActiveEditor(): Promise<void> {
    await this.activePane?.save?.();
  }

  setContent(content: Element): void {
    this._activeGroup.setContent(content);
  }

  async splitActiveGroupHorizontal(): Promise<void> {
    const source = this._activeGroup;
    const sourceIndex = this._groups.findIndex(
      ({ group }) => group === source,
    );
    if (sourceIndex < 0) {
      throw new Error("Active EditorGroup is not owned by EditorPart");
    }
    const created = this.createGroup();
    const targetIndex = sourceIndex + 1;
    this._groups.splice(targetIndex, 0, created);
    this.splitView.addView(
      created.view,
      { type: "split", index: sourceIndex },
      targetIndex,
    );
    this.splitView.distributeViewSizes();
    this._activeGroup = created.group;
    try {
      if (source.activeInput) {
        await created.group.openEditor(source.activeInput);
      }
      created.group.focus();
    } catch (error) {
      this.splitView.removeView(targetIndex);
      this._groups.splice(targetIndex, 1);
      created.group.dispose();
      this._activeGroup = source;
      throw error;
    }
  }

  override layout(dimension: IDimension): void {
    this.dimension = new Dimension(dimension.width, dimension.height);
    this.splitView.layout(
      this.dimension.width,
      this.dimension.height,
    );
  }

  focus(): void {
    this._activeGroup.focus();
  }

  private async saveEditor(group: IEditorGroup, input: EditorInput, pane: IEditorPane): Promise<boolean> {
    if (input.resource.scheme !== "untitled") throw new Error("Save As is only available for untitled editors");
    if (!this.saveAsResource) throw new Error("Editor Save As is unavailable in this host");
    if (!pane.saveAs) throw new Error("The active editor cannot save this document");
    const target = await this.saveAsResource(editorInputLabel(input));
    if (!target) return false;
    await pane.saveAs(target);
    await group.replaceEditor(input, {
      resource: target,
      label: editorInputLabel({ resource: target }),
    });
    return true;
  }

  private createGroup(): EditorGroupHost {
    let group: EditorGroup;
    group = this.own(new EditorGroup({
      ownerDocument: this.element.ownerDocument,
      ...this.groupOptions,
      onDidActivate: () => {
        this._activeGroup = group;
      },
      dragAndDrop: {
        start: (source, input) => this.tabDragAndDrop.start(source, input),
        isDragging: () => this.tabDragAndDrop.isDragging(),
        drop: (target, targetInput, position) => this.tabDragAndDrop.drop(target, targetInput, position),
        end: () => this.tabDragAndDrop.end(),
      },
    }));
    return {
      group,
      view: new EditorGroupSplitView(group),
    };
  }

  private dropEditor(event: EditorTabDropEvent): void {
    const targetIndex = event.target.getEditorInsertionIndex(event.targetInput, event.position);
    if (event.source === event.target) {
      event.target.moveEditor(event.input, targetIndex);
      this._activeGroup = event.target;
      event.target.focus();
      return;
    }
    void event.source.moveEditorTo(event.input, event.target, targetIndex)
      .then(() => {
        this._activeGroup = event.target;
        event.target.focus();
      })
      .catch((error) => {
        console.error("Failed to move Editor tab", error);
      });
  }
}

function editorInputLabel(input: Pick<EditorInput, "resource" | "label">): string {
  if (input.label?.trim()) return input.label;
  const path = decodeURIComponent(input.resource.path).replace(/\/+$/, "");
  const separator = path.lastIndexOf("/");
  return path.slice(separator + 1) || input.resource.toString();
}

interface EditorGroupHost {
  readonly group: EditorGroup;
  readonly view: EditorGroupSplitView;
}

class EditorGroupSplitView implements ISplitViewView {
  readonly minimumSize = 120;
  readonly maximumSize = Infinity;

  constructor(readonly group: EditorGroup) {}

  get element(): HTMLElement {
    return this.group.element;
  }

  layout(size: number, _offset: number, orthogonalSize: number): void {
    this.group.layout({
      width: size,
      height: orthogonalSize,
    });
  }
}
