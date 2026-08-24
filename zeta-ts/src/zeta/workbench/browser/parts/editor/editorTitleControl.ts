import "./media/editorTitleControl.css";
import type { IContextMenuProvider } from "../../../../base/browser/contextmenu.js";
import { DisposableOwner } from "../../../../base/common/lifecycle.js";
import { MenuWorkbenchToolBar, WorkbenchToolBar } from "../../../../platform/actions/browser/toolbar.js";
import { MenuId } from "../../../../platform/actions/common/actions.js";
import type { IMenuService } from "../../../../platform/actions/common/menuService.js";
import type { IContextKeyService } from "../../../../platform/contextkey/common/contextkey.js";
import type { EditorInput } from "./editorInput.js";
import { EditorTabsControl, type EditorTabDescriptor, type EditorTabsDelegate } from "./editorTabsControl.js";
import { MultiEditorTabsControl } from "./multiEditorTabsControl.js";
import { h } from "../../../../base/browser/dom.js";

/** Platform services used to populate the Editor title toolbar. */
export interface EditorTitleActions {
	readonly menuService: IMenuService;
	readonly contextMenuProvider: IContextMenuProvider;
	readonly contextKeyService?: IContextKeyService;
}

/** Hosts one group's Editor tabs and its independent action toolbar. */
export class EditorTitleControl extends DisposableOwner {
	static readonly HEIGHT = 35;

	readonly domNode: HTMLDivElement;
	private readonly tabs: EditorTabsControl;
	private readonly toolbar: WorkbenchToolBar;

	constructor(
		container: HTMLElement,
		delegate: EditorTabsDelegate,
		titleActions?: EditorTitleActions,
	) {
		super();
		const ownerDocument = container.ownerDocument;
		this.domNode = h(ownerDocument, "div");
		this.domNode.className = "zeta-editor-title-control";
		container.append(this.domNode);
		const tabsAndActionsDomNode = h(ownerDocument, "div");
		tabsAndActionsDomNode.className = "zeta-editor-tabs-and-actions";
		this.domNode.append(tabsAndActionsDomNode);
		this.tabs = this.own(new MultiEditorTabsControl(tabsAndActionsDomNode, delegate));
		const actionsDomNode = h(ownerDocument, "div");
		actionsDomNode.className = "zeta-editor-title-actions";
		tabsAndActionsDomNode.append(actionsDomNode);
		this.toolbar = this.own(titleActions
			? new MenuWorkbenchToolBar(
				actionsDomNode,
				titleActions.menuService,
				titleActions.contextMenuProvider,
				MenuId.EditorTitle,
				{
					highlightToggledItems: true,
					contextKeyService: titleActions.contextKeyService,
				},
			)
			: new WorkbenchToolBar(
				actionsDomNode,
				emptyEditorToolbarContextMenuProvider,
				{
					ariaLabel: "Editor actions",
					highlightToggledItems: true,
				},
			));
		this.defer(() => this.domNode.remove());
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
