import "./editorpart.css";
import type { IContextMenuProvider } from "../../../../base/browser/contextmenu.js";
import { Dimension, type IDimension } from "../../../../base/browser/geometry.js";
import { SplitView, type ISplitViewView } from "../../../../base/browser/ui/splitview/splitview.js";
import type { IMenuService } from "../../../../platform/actions/common/menuService.js";
import type { IConfigurationService } from "../../../../platform/configuration/common/configuration.js";
import { createServiceIdentifier } from "../../../../platform/instantiation/common/instantiation.js";
import type { IKeybindingService } from "../../../../platform/keybinding/common/keybinding.js";
import { type ITextFileService } from "../../../services/textfile/common/textFileService.js";
import { WorkbenchPart } from "../../part.js";
import { EditorGroup, type EditorGroupOptions, type IEditorGroup } from "./editorGroup.js";
import type { EditorInput, EditorOpenOptions } from "./editorInput.js";
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
  readonly textFileService?: ITextFileService;
  readonly registry?: EditorPaneRegistry;
  readonly titleActions?: {
    readonly menuService: IMenuService;
    readonly contextMenuProvider: IContextMenuProvider;
  };
}

/** Owns EditorGroup layout and delegates editor behavior to the active group. */
export class EditorPart extends WorkbenchPart implements IEditorPart {
  private readonly splitView: SplitView;
  private readonly groupOptions: Omit<EditorGroupOptions, "ownerDocument" | "onDidActivate">;
  private readonly _groups: EditorGroupHost[] = [];
  private _activeGroup: EditorGroup;
  private dimension = Dimension.Zero;

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
      textFileService: options.textFileService,
      titleActions: options.titleActions,
    };
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

  private createGroup(): EditorGroupHost {
    let group: EditorGroup;
    group = this.own(new EditorGroup({
      ownerDocument: this.element.ownerDocument,
      ...this.groupOptions,
      onDidActivate: () => {
        this._activeGroup = group;
      },
    }));
    return {
      group,
      view: new EditorGroupSplitView(group),
    };
  }
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
