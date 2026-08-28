import { addDisposableListener, ModifierKeyEmitter } from "../../../base/browser/dom.js";
import { setAriaAttribute } from "../../../base/browser/ui/aria/aria.js";
import { ActionViewItem, ButtonActionViewItem, type ActionViewItemOptions } from "../../../base/browser/ui/actionbar/actionViewItems.js";
import type { IContextMenuProvider } from "../../../base/browser/contextmenu.js";
import { DropdownMenuActionViewItem } from "../../../base/browser/ui/dropdown/dropdownMenuActionViewItem.js";
import { SubmenuAction, type IAction } from "../../../base/common/actions.js";
import { isLinux, isWindows } from "../../../base/common/platform.js";
import { MenuItemAction, SubmenuItemAction } from "../common/actions.js";

export function shouldUseAlternativeMenuActions(targetWindow: Window): boolean {
	const status = ModifierKeyEmitter.getInstance(targetWindow).keyStatus;
	return status.altKey || ((isWindows || isLinux) && status.shiftKey);
}

export function resolveAlternativeMenuActions(
	actions: readonly IAction[],
	useAlternative: boolean,
): readonly IAction[] {
	if (!useAlternative) return actions;
	return actions.map(resolveAlternativeMenuAction);
}

function resolveAlternativeMenuAction(action: IAction): IAction {
	if (action instanceof MenuItemAction) return action.alt ?? action;
	if (!(action instanceof SubmenuAction)) return action;
	return new SubmenuAction(
		action.id,
		action.label,
		action.actions.map(resolveAlternativeMenuAction),
		action.icon,
	);
}

/**
 * ActionBar representation of one command resolved from a menu contribution.
 *
 * Popup-menu rows use base Menu's private view items instead; this platform
 * representation is for contributed actions rendered in toolbars and other
 * ActionBar hosts.
 */
export class MenuEntryActionViewItem extends ButtonActionViewItem {
	private readonly menuItemAction: MenuItemAction;
	private activeAction: MenuItemAction;
	private isMouseOver = false;

	constructor(action: MenuItemAction, options: ActionViewItemOptions = {}) {
		super(action, options);
		this.menuItemAction = action;
		this.activeAction = action;
	}

	override render(container: HTMLElement): void {
		super.render(container);
		container.classList.add("zeta-menu-entry");
		if (!this.menuItemAction.alt) return;
		const targetWindow = container.ownerDocument.defaultView;
		if (!targetWindow) return;
		const modifiers = ModifierKeyEmitter.getInstance(targetWindow);
		const update = (): void => this.updateAlternativeState(modifiers);
		this._register(modifiers.event(update));
		this._register(addDisposableListener(container, "mouseenter", () => {
			this.isMouseOver = true;
			update();
		}));
		this._register(addDisposableListener(container, "mouseleave", () => {
			this.isMouseOver = false;
			update();
		}));
		update();
	}

	protected override runAction(): unknown {
		return this.activeAction.run();
	}

	private updateAlternativeState(modifiers: ModifierKeyEmitter): void {
		const status = modifiers.keyStatus;
		const alternate = this.menuItemAction.alt;
		const nextAction = alternate?.enabled && (status.altKey || (status.shiftKey && this.isMouseOver))
			? alternate
			: this.menuItemAction;
		if (this.activeAction === nextAction) return;
		this.activeAction = nextAction;
		this.button.label = nextAction.label;
		this.button.icon = nextAction.icon;
		this.button.enabled = nextAction.enabled;
		this.button.checked = nextAction.checked;
		this.button.setTitle(nextAction.tooltip);
		setAriaAttribute(this.button.domNode, "label", nextAction.label);
	}
}

/**
 * ActionBar representation of a contributed submenu.
 *
 * The trigger retains toolbar semantics while its popup delegates all menu-row
 * rendering, keyboard navigation, and nested submenus to base Menu.
 */
export class SubmenuEntryActionViewItem
	extends DropdownMenuActionViewItem {
	constructor(
		action: SubmenuItemAction,
		contextMenuProvider: IContextMenuProvider,
		options: ActionViewItemOptions = {},
	) {
		super(action, () => action.actions, contextMenuProvider, options);
	}

	override render(container: HTMLElement): void {
		super.render(container);
		container.classList.add("zeta-menu-entry");
	}
}

/**
 * Creates the ActionBar representation for a platform menu contribution.
 *
 * Returning undefined lets ActionBar use its base representation for actions
 * that were not produced by the platform menu service.
 */
export function createMenuEntryActionViewItem(
	action: IAction,
	contextMenuProvider: IContextMenuProvider,
	options: ActionViewItemOptions = {},
): ActionViewItem | undefined {
	if (action instanceof MenuItemAction) {
		return new MenuEntryActionViewItem(action, options);
	}
	if (action instanceof SubmenuItemAction) {
		return new SubmenuEntryActionViewItem(action, contextMenuProvider, options);
	}
	return undefined;
}
