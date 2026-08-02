import "./editorTitleControl.css";
import type { IContextMenuProvider } from "../../../../base/browser/contextmenu.js";
import { DisposableOwner } from "../../../../base/common/lifecycle.js";
import { MenuWorkbenchToolBar, WorkbenchToolBar } from "../../../../platform/actions/browser/toolbar.js";
import { MenuId } from "../../../../platform/actions/common/actions.js";
import type { IMenuService } from "../../../../platform/actions/common/menuService.js";
import type { EditorInput } from "./editorInput.js";
import { EditorTabsControl, type EditorTabDescriptor, type EditorTabsDelegate } from "./editorTabsControl.js";

/** Platform services used to populate the Editor title toolbar. */
export interface EditorTitleActions {
  readonly menuService: IMenuService;
  readonly contextMenuProvider: IContextMenuProvider;
}

/** Hosts one group's Editor tabs and its independent action toolbar. */
export class EditorTitleControl extends DisposableOwner {
  static readonly HEIGHT = 35;

  readonly element: HTMLDivElement;
  private readonly tabs: EditorTabsControl;
  private readonly toolbar: WorkbenchToolBar;

  constructor(
    ownerDocument: Document,
    delegate: EditorTabsDelegate,
    titleActions?: EditorTitleActions,
  ) {
    super();
    this.element = ownerDocument.createElement("div");
    this.element.className = "zeta-editor-title-control";
    this.tabs = this.own(new EditorTabsControl(ownerDocument, delegate));
    this.toolbar = this.own(titleActions
      ? new MenuWorkbenchToolBar(
        titleActions.menuService,
        titleActions.contextMenuProvider,
        MenuId.EditorTitle,
        ownerDocument,
        { highlightToggledItems: true },
      )
      : new WorkbenchToolBar(
        emptyEditorToolbarContextMenuProvider,
        ownerDocument,
        {
          ariaLabel: "Editor actions",
          highlightToggledItems: true,
        },
      ));
    const actions = ownerDocument.createElement("div");
    actions.className = "zeta-editor-title-actions";
    actions.append(this.toolbar.element);
    const tabsAndActions = ownerDocument.createElement("div");
    tabsAndActions.className = "zeta-editor-tabs-and-actions";
    tabsAndActions.append(this.tabs.element, actions);
    this.element.append(tabsAndActions);
    this.defer(() => this.element.remove());
  }

  setEditors(
    editors: readonly EditorTabDescriptor[],
    activeInput: EditorInput | undefined,
  ): void {
    this.tabs.setEditors(editors, activeInput);
  }
}

const emptyEditorToolbarContextMenuProvider: IContextMenuProvider = {
  showContextMenu(): never {
    throw new Error(
      "The empty Editor toolbar cannot present secondary actions",
    );
  },
};
