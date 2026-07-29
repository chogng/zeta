import "./editorpart.css";
import {
  type IDimension,
} from "../../../../base/browser/geometry.js";
import type {
  IContextMenuProvider,
} from "../../../../base/browser/contextmenu.js";
import {
  createServiceIdentifier,
} from "../../../../platform/instantiation/common/instantiation.js";
import type {
  IKeybindingService,
} from "../../../../platform/keybinding/common/keybinding.js";
import type {
  IMenuService,
} from "../../../../platform/actions/common/menuService.js";
import { WorkbenchPart } from "../../part.js";
import {
  EditorGroup,
  type IEditorGroup,
} from "./editorGroup.js";
import type {
  EditorInput,
  EditorOpenOptions,
} from "./editorInput.js";
import type {
  IEditorPane,
} from "./editorPane.js";
import {
  EditorPaneRegistry,
  EditorPanes,
} from "./editorRegistry.js";

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
  layout(dimension: IDimension): void;
  focus(): void;
}

export const IEditorPart =
  createServiceIdentifier<IEditorPart>("editorPart");

/** Named collaborators used to construct the editor region. */
export interface IEditorPartOptions {
  readonly keybindingService?: IKeybindingService;
  readonly registry?: EditorPaneRegistry;
  readonly titleActions?: {
    readonly menuService: IMenuService;
    readonly contextMenuProvider: IContextMenuProvider;
  };
}

/** Owns EditorGroup layout and delegates editor behavior to the active group. */
export class EditorPart extends WorkbenchPart implements IEditorPart {
  readonly #group: EditorGroup;

  override get minimumWidth(): number { return 120; }
  override get minimumHeight(): number { return 119; }

  constructor(
    ownerDocument: Document,
    options: IEditorPartOptions = {},
  ) {
    super("editor", ownerDocument);
    this.titleElement.remove();
    this.element.setAttribute("aria-label", "Editor");
    this.#group = this.own(new EditorGroup({
      ownerDocument,
      registry: options.registry ?? EditorPanes,
      keybindingService: options.keybindingService,
      titleActions: options.titleActions,
    }));
    this.contentElement.append(this.#group.element);
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
    return [this.#group];
  }

  get activeGroup(): IEditorGroup {
    return this.#group;
  }

  get activeInput(): EditorInput | undefined {
    return this.#group.activeInput;
  }

  get activePane(): IEditorPane | undefined {
    return this.#group.activePane;
  }

  openEditor(
    input: EditorInput,
    options: EditorOpenOptions = {},
  ): Promise<IEditorPane> {
    return this.#group.openEditor(input, options);
  }

  activateEditor(input: EditorInput): IEditorPane {
    return this.#group.activateEditor(input);
  }

  closeEditor(input: EditorInput): void {
    this.#group.closeEditor(input);
  }

  setContent(content: Element): void {
    this.#group.setContent(content);
  }

  override layout(dimension: IDimension): void {
    this.#group.layout(dimension);
  }

  focus(): void {
    this.#group.focus();
  }
}
