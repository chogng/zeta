import type { IAction } from "../common/actions.js";

export interface IContextMenuPoint {
	readonly x: number;
	readonly y: number;
}

export type ContextMenuAnchor = Element | IContextMenuPoint;

/** Action-based context menu request understood by base UI controls. */
export interface IActionContextMenuOptions {
	readonly anchor: ContextMenuAnchor;
	readonly actions: readonly IAction[];
	readonly onHide?: (didCancel: boolean) => void;
}

/**
 * Presents action menus without exposing a platform service dependency.
 *
 * Base controls use this contract to delegate menu policy to the host.
 */
export interface IContextMenuProvider {
	showContextMenu(options: IActionContextMenuOptions): void;
}
