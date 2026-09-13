import { type IKeyboardEvent } from '../../../base/browser/keyboardEvent.js';
import { type IMouseWheelEvent } from '../../../base/browser/mouseEvent.js';
import { type ICoordinatesConverter } from '../../common/coordinatesConverter.js';
import { Position } from '../../common/core/position.js';
import { type IEditorMouseEvent, type IMouseTarget, type IMouseTargetViewZoneData, type IPartialEditorMouseEvent, MouseTargetType } from '../editorBrowser.js';

export interface EventCallback<T> {
	(event: T): void;
}

export class ViewUserInputEvents {
	public onKeyDown: EventCallback<IKeyboardEvent> | null = null;
	public onKeyUp: EventCallback<IKeyboardEvent> | null = null;
	public onContextMenu: EventCallback<IEditorMouseEvent> | null = null;
	public onMouseMove: EventCallback<IEditorMouseEvent> | null = null;
	public onMouseLeave: EventCallback<IPartialEditorMouseEvent> | null = null;
	public onMouseDown: EventCallback<IEditorMouseEvent> | null = null;
	public onMouseUp: EventCallback<IEditorMouseEvent> | null = null;
	public onMouseDrag: EventCallback<IEditorMouseEvent> | null = null;
	public onMouseDrop: EventCallback<IPartialEditorMouseEvent> | null = null;
	public onMouseDropCanceled: EventCallback<void> | null = null;
	public onMouseWheel: EventCallback<IMouseWheelEvent> | null = null;

	constructor(private readonly coordinatesConverter: ICoordinatesConverter) {}

	public emitKeyDown(event: IKeyboardEvent): void {
		this.onKeyDown?.(event);
	}

	public emitKeyUp(event: IKeyboardEvent): void {
		this.onKeyUp?.(event);
	}

	public emitContextMenu(event: IEditorMouseEvent): void {
		this.onContextMenu?.(this.convertViewToModelMouseEvent(event));
	}

	public emitMouseMove(event: IEditorMouseEvent): void {
		this.onMouseMove?.(this.convertViewToModelMouseEvent(event));
	}

	public emitMouseLeave(event: IPartialEditorMouseEvent): void {
		this.onMouseLeave?.(this.convertViewToModelMouseEvent(event));
	}

	public emitMouseDown(event: IEditorMouseEvent): void {
		this.onMouseDown?.(this.convertViewToModelMouseEvent(event));
	}

	public emitMouseUp(event: IEditorMouseEvent): void {
		this.onMouseUp?.(this.convertViewToModelMouseEvent(event));
	}

	public emitMouseDrag(event: IEditorMouseEvent): void {
		this.onMouseDrag?.(this.convertViewToModelMouseEvent(event));
	}

	public emitMouseDrop(event: IPartialEditorMouseEvent): void {
		this.onMouseDrop?.(this.convertViewToModelMouseEvent(event));
	}

	public emitMouseDropCanceled(): void {
		this.onMouseDropCanceled?.();
	}

	public emitMouseWheel(event: IMouseWheelEvent): void {
		this.onMouseWheel?.(event);
	}

	private convertViewToModelMouseEvent(event: IEditorMouseEvent): IEditorMouseEvent;
	private convertViewToModelMouseEvent(event: IPartialEditorMouseEvent): IPartialEditorMouseEvent;
	private convertViewToModelMouseEvent(event: IEditorMouseEvent | IPartialEditorMouseEvent): IEditorMouseEvent | IPartialEditorMouseEvent {
		return event.target === null
			? event
			: { event: event.event, target: this.convertViewToModelMouseTarget(event.target) };
	}

	private convertViewToModelMouseTarget(target: IMouseTarget): IMouseTarget {
		return ViewUserInputEvents.convertViewToModelMouseTarget(target, this.coordinatesConverter);
	}

	public static convertViewToModelMouseTarget(target: IMouseTarget, coordinatesConverter: ICoordinatesConverter): IMouseTarget {
		const position = target.position ? coordinatesConverter.convertViewPositionToModelPosition(target.position) : null;
		const range = target.range ? coordinatesConverter.convertViewRangeToModelRange(target.range) : null;
		if (target.type === MouseTargetType.GUTTER_VIEW_ZONE || target.type === MouseTargetType.CONTENT_VIEW_ZONE) {
			return {
				...target,
				position: position!,
				range: range!,
				detail: this.convertViewToModelViewZoneData(target.detail, coordinatesConverter),
			};
		}
		return { ...target, position, range } as IMouseTarget;
	}

	private static convertViewToModelViewZoneData(data: IMouseTargetViewZoneData, coordinatesConverter: ICoordinatesConverter): IMouseTargetViewZoneData {
		return {
			viewZoneId: data.viewZoneId,
			positionBefore: data.positionBefore ? coordinatesConverter.convertViewPositionToModelPosition(data.positionBefore) : null,
			positionAfter: data.positionAfter ? coordinatesConverter.convertViewPositionToModelPosition(data.positionAfter) : null,
			position: coordinatesConverter.convertViewPositionToModelPosition(data.position),
			afterLineNumber: coordinatesConverter.convertViewPositionToModelPosition(new Position(data.afterLineNumber, 1)).lineNumber,
		};
	}
}
