import { type StandardKeyboardEvent } from '../../../base/browser/keyboardEvent.js';
import { type TextPosition, type TextRange } from '../../common/core/text.js';
import { EditorHitTargetKind } from '../../common/viewModel/pointerHitTest.js';

/** Callback shape used by the view/input bridge, matching VS Code's boundary. */
export interface EventCallback<T> {
	(event: T): void;
}

/** Browser-owned regions are kept distinct from common-layer hit-test targets. */
export type EditorViewMouseTargetKind =
	| EditorHitTargetKind
	| 'lineNumber'
	| 'gutterDecoration'
	| 'widget'
	| 'scrollbar'
	| 'viewZone';

/** Mouse target exposed by the browser view without depending on controller policy. */
export interface EditorViewMouseTarget {
	readonly kind: EditorViewMouseTargetKind;
	readonly position?: TextPosition;
	readonly range?: TextRange;
	readonly element?: Element;
	readonly detail?: unknown;
}

/** A mouse event whose target has already been resolved by the browser view. */
export interface EditorViewMouseEvent {
	readonly event: MouseEvent | PointerEvent;
	readonly target?: EditorViewMouseTarget;
}

/** Events such as leave/drop may omit a semantic target. */
export type EditorViewPartialMouseEvent = EditorViewMouseEvent;

/**
 * Transport between browser view input producers and editor-facing consumers.
 *
 * VS Code performs view-to-model coordinate conversion here. Stanza's hit-test
 * contract already returns model-relative TextPosition values, so this class
 * preserves that contract and only isolates the target object at the boundary.
 * It does not own pointer gestures, selection, editing, or drag/drop policy.
 */
export class ViewUserInputEvents {
	public onKeyDown: EventCallback<StandardKeyboardEvent> | null = null;
	public onKeyUp: EventCallback<KeyboardEvent> | null = null;
	public onContextMenu: EventCallback<EditorViewMouseEvent> | null = null;
	public onMouseMove: EventCallback<EditorViewMouseEvent> | null = null;
	public onMouseLeave: EventCallback<EditorViewPartialMouseEvent> | null = null;
	public onMouseDown: EventCallback<EditorViewMouseEvent> | null = null;
	public onMouseUp: EventCallback<EditorViewMouseEvent> | null = null;
	public onMouseDrag: EventCallback<EditorViewMouseEvent> | null = null;
	public onMouseDrop: EventCallback<EditorViewPartialMouseEvent> | null = null;
	public onMouseDropCanceled: EventCallback<void> | null = null;
	public onMouseWheel: EventCallback<WheelEvent> | null = null;

	public emitKeyDown(event: StandardKeyboardEvent): void {
		this.onKeyDown?.(event);
	}

	public emitKeyUp(event: KeyboardEvent): void {
		this.onKeyUp?.(event);
	}

	public emitContextMenu(event: EditorViewMouseEvent): void {
		this.onContextMenu?.(this.convertViewToModelMouseEvent(event));
	}

	public emitMouseMove(event: EditorViewMouseEvent): void {
		this.onMouseMove?.(this.convertViewToModelMouseEvent(event));
	}

	public emitMouseLeave(event: EditorViewPartialMouseEvent): void {
		this.onMouseLeave?.(this.convertViewToModelMouseEvent(event));
	}

	public emitMouseDown(event: EditorViewMouseEvent): void {
		this.onMouseDown?.(this.convertViewToModelMouseEvent(event));
	}

	public emitMouseUp(event: EditorViewMouseEvent): void {
		this.onMouseUp?.(this.convertViewToModelMouseEvent(event));
	}

	public emitMouseDrag(event: EditorViewMouseEvent): void {
		this.onMouseDrag?.(this.convertViewToModelMouseEvent(event));
	}

	public emitMouseDrop(event: EditorViewPartialMouseEvent): void {
		this.onMouseDrop?.(this.convertViewToModelMouseEvent(event));
	}

	public emitMouseDropCanceled(): void {
		this.onMouseDropCanceled?.();
	}

	public emitMouseWheel(event: WheelEvent): void {
		this.onMouseWheel?.(event);
	}

	private convertViewToModelMouseEvent<T extends EditorViewMouseEvent>(event: T): T {
		if (!event.target) return event;
		return Object.freeze({
			...event,
			target: Object.freeze({ ...event.target }),
		}) as T;
	}
}
