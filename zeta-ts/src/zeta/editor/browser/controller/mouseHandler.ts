import { addDisposableListener } from '../../../base/browser/dom.js';
import { StandardMouseEvent, StandardWheelEvent } from '../../../base/browser/mouseEvent.js';
import { Disposable, DisposableStore, toDisposable } from "../../../base/common/lifecycle.js";
import { Range } from "../../common/core/range.js";
import { type View } from "../view.js";
import { DragScrolling } from "./dragScrolling.js";
import { PointerHandler } from "./pointerHandler.js";
import { MouseTargetFactory, MouseTargetKind } from "./mouseTarget.js";
import { type MouseTarget } from './mouseTarget.js';
import { EditorHitTargetKind, type EditorHitTarget } from "../../common/viewModel/pointerHitTest.js";
import { type IMouseDispatchData, type ViewController } from '../view/viewController.js';
import { type IEditorMouseEvent, type IMouseTarget, MouseTargetType } from '../editorBrowser.js';
import { NavigationCommandRevealType } from '../coreCommands.js';

interface PointerGesture {
	readonly pointerId: number | undefined;
	readonly startedOnLineNumbers: boolean;
	readonly mouseDownCount: number;
	readonly altKey: boolean;
	readonly ctrlKey: boolean;
	readonly metaKey: boolean;
	readonly shiftKey: boolean;
	readonly leftButton: boolean;
	readonly middleButton: boolean;
}

/** Owns pointer capture, drag scrolling, target resolution, and browser event publication. */
export class MouseHandler extends Disposable {
	private readonly dragListeners =
		this._register(new DisposableStore());
	private readonly pointerHandler: PointerHandler;
	private readonly mouseTargetFactory: MouseTargetFactory;
	private gesture: PointerGesture | undefined;
	private autoScroller: DragScrolling | undefined;

	constructor(
		private readonly viewport: View,
		private readonly viewController: ViewController,
	) {
		super();
		this.pointerHandler = this._register(new PointerHandler(viewport.element));
		this.mouseTargetFactory = new MouseTargetFactory(viewport);
		this._register(this.pointerHandler.onDidPointerDown(event => this.beginPointerSelection(event)));
		this._register(this.pointerHandler.onDidContextMenu(event => this.handleContextMenu(event)));
		this._register(addDisposableListener<MouseEvent>(viewport.element, 'mousemove', event => this.handleMouseMove(event)));
		this._register(addDisposableListener<MouseEvent>(viewport.element, 'mouseleave', event => this.handleMouseLeave(event)));
		this._register(addDisposableListener<WheelEvent>(viewport.element, 'wheel', event => this.viewController.emitMouseWheel(new StandardWheelEvent(event, { lineHeight: viewport.currentLayout.lineHeight }))));
		this._register(addDisposableListener<DragEvent>(viewport.element, 'drop', event => {
			const target = Number.isFinite(event.clientX) && Number.isFinite(event.clientY)
				? this.mouseTargetFactory.create(event, true)
				: undefined;
			this.viewController.emitMouseDrop({ event: new StandardMouseEvent(event), target: target ? this.toViewMouseTarget(target) : null });
		}));
		this._register(toDisposable(() => this.stopPointerSelection()));
	}

	private beginPointerSelection(event: PointerEvent): void {
		const target = this.mouseTargetFactory.create(event);
		const editorEvent = this.toEditorMouseEvent(event, target);
		if (event.defaultPrevented || (event.button !== 0 && event.button !== 1) || !target || target.kind === MouseTargetKind.Scrollbar || target.kind === MouseTargetKind.Widget || target.kind === MouseTargetKind.ViewZone) {
			this.viewController.emitMouseDown(editorEvent);
			return;
		}
		const hitTarget = target.editorTarget;
		if (!hitTarget) {
			this.viewController.emitMouseDown(editorEvent);
			return;
		}
		event.preventDefault();
		this.viewport.element.focus({ preventScroll: true });
		this.stopPointerSelection();
		const pointerId = readPointerId(event);
		try {
			this.gesture = {
				pointerId,
				startedOnLineNumbers: target.kind === MouseTargetKind.LineNumber,
				mouseDownCount: readClickCount(event),
				altKey: event.altKey,
				ctrlKey: event.ctrlKey,
				metaKey: event.metaKey,
				shiftKey: event.shiftKey,
				leftButton: event.button === 0,
				middleButton: event.button === 1,
			};
			this.dispatchTarget(hitTarget, false);
			this.pointerHandler.capturePointer(pointerId);

			const targetWindow = this.pointerHandler.targetWindow;
			this.autoScroller = this.dragListeners.add(
				new DragScrolling(
					targetWindow,
					this.viewport,
					target => this.dispatchTarget(target, true),
				),
			);
			this.dragListeners.add(this.pointerHandler.startTracking(pointerId, {
				onMove: event => this.updatePointerSelection(event),
				onUp: event => this.finishPointerSelection(event),
				onCancel: event => this.cancelPointerSelection(event),
				onBlur: () => this.stopPointerSelection(),
			}));
		} catch (error) {
			this.stopPointerSelection();
			throw error;
		}
		this.viewController.emitMouseDown(editorEvent);
	}

	private handleContextMenu(event: MouseEvent): void {
		const target = this.mouseTargetFactory.create(event, true);
		this.viewController.emitContextMenu(this.toEditorMouseEvent(event, target));
	}

	private handleMouseMove(event: MouseEvent): void {
		if (this.gesture) return;
		this.viewController.emitMouseMove(this.toEditorMouseEvent(event, this.mouseTargetFactory.create(event, true)));
	}

	private handleMouseLeave(event: MouseEvent): void {
		this.viewController.emitMouseLeave({ event: new StandardMouseEvent(event), target: null });
	}

	private updatePointerSelection(event: PointerEvent): void {
		if (!this.accepts(event)) return;
		const hitTarget = this.viewport.getNearestTargetAtClientPoint(event);
		if (hitTarget) this.dispatchTarget(hitTarget, true);
		this.autoScroller?.updatePointer(event);
		this.viewController.emitMouseDrag(this.toEditorMouseEvent(event, this.mouseTargetFactory.create(event, true)));
	}

	private finishPointerSelection(event: PointerEvent): void {
		if (!this.accepts(event)) return;
		const hitTarget = this.viewport.getNearestTargetAtClientPoint(event);
		if (hitTarget) this.dispatchTarget(hitTarget, true);
		this.viewController.emitMouseUp(this.toEditorMouseEvent(event, this.mouseTargetFactory.create(event, true)));
		this.stopPointerSelection();
	}

	private cancelPointerSelection(event: PointerEvent): void {
		if (!this.accepts(event)) return;
		this.stopPointerSelection();
	}

	private dispatchTarget(hitTarget: EditorHitTarget, inSelectionMode: boolean): void {
		const gesture = this.gesture;
		if (!gesture) return;
		const position = this.viewport.coordinatesConverter.convertModelPositionToViewPosition(hitTarget.position);
		const data: IMouseDispatchData = {
			position,
			mouseColumn: position.column,
			revealType: NavigationCommandRevealType.Minimal,
			startedOnLineNumbers: gesture.startedOnLineNumbers,
			inSelectionMode,
			mouseDownCount: gesture.mouseDownCount,
			altKey: gesture.altKey,
			ctrlKey: gesture.ctrlKey,
			metaKey: gesture.metaKey,
			shiftKey: gesture.shiftKey,
			leftButton: gesture.leftButton,
			middleButton: gesture.middleButton,
			onInjectedText: false,
		};
		this.viewController.dispatchMouse(data);
	}

	private accepts(event: PointerEvent): boolean {
		const gesture = this.gesture;
		if (!gesture) return false;
		const pointerId = readPointerId(event);
		return gesture.pointerId === undefined ||
			pointerId === undefined ||
			pointerId === gesture.pointerId;
	}

	private stopPointerSelection(): void {
		const gesture = this.gesture;
		this.gesture = undefined;
		this.autoScroller = undefined;
		this.dragListeners.clear();
		const pointerId = gesture?.pointerId;
		this.pointerHandler.releasePointer(pointerId);
	}

	private toEditorMouseEvent(event: MouseEvent | PointerEvent, target: MouseTarget | undefined): IEditorMouseEvent {
		return { event: new StandardMouseEvent(event), target: this.toViewMouseTarget(target) };
	}

	private toViewMouseTarget(target: MouseTarget | undefined): IMouseTarget {
		const elementConstructor = this.viewport.element.ownerDocument.defaultView?.HTMLElement;
		const element = elementConstructor && target?.element instanceof elementConstructor ? target.element as HTMLElement : null;
		const modelPosition = target?.editorTarget?.position;
		const position = modelPosition ? this.viewport.coordinatesConverter.convertModelPositionToViewPosition(modelPosition) : null;
		const range = position ? Range.fromPositions(position) : null;
		const mouseColumn = position?.column ?? 0;
		if (!target) return { type: MouseTargetType.UNKNOWN, element, mouseColumn, position, range };
		if (!position || !range) {
			if (target.kind === MouseTargetKind.Widget) {
				return { type: MouseTargetType.CONTENT_WIDGET, element, mouseColumn, position: null, range: null, detail: element?.id ?? '' };
			}
			return { type: MouseTargetType.UNKNOWN, element, mouseColumn, position, range };
		}
		switch (target.kind) {
			case MouseTargetKind.Text:
				return { type: MouseTargetType.CONTENT_TEXT, element, mouseColumn, position, range, detail: { mightBeForeignElement: false, injectedText: null } };
			case MouseTargetKind.EmptyContent:
			case MouseTargetKind.AfterLines:
				return { type: MouseTargetType.CONTENT_EMPTY, element, mouseColumn, position, range, detail: { isAfterLines: target.kind === MouseTargetKind.AfterLines } };
			case MouseTargetKind.Gutter:
			case MouseTargetKind.LineNumber:
			case MouseTargetKind.GutterDecoration: {
				const layout = this.viewport.getLayoutInfo();
				const type = target.kind === MouseTargetKind.LineNumber
					? MouseTargetType.GUTTER_LINE_NUMBERS
					: target.glyphMarginLane === undefined ? MouseTargetType.GUTTER_LINE_DECORATIONS : MouseTargetType.GUTTER_GLYPH_MARGIN;
				return { type, element, mouseColumn, position, range, detail: { isAfterLines: target.editorTarget?.kind === EditorHitTargetKind.AfterLines, glyphMarginLeft: layout.glyphMarginLeft, glyphMarginWidth: layout.glyphMarginWidth, glyphMarginLane: target.glyphMarginLane, lineNumbersWidth: layout.lineNumbersWidth, offsetX: 0 } };
			}
			case MouseTargetKind.Widget:
				return { type: MouseTargetType.CONTENT_WIDGET, element, mouseColumn, position: null, range: null, detail: element?.id ?? '' };
			case MouseTargetKind.Scrollbar:
				return position && range
					? { type: MouseTargetType.SCROLLBAR, element, mouseColumn, position, range }
					: { type: MouseTargetType.UNKNOWN, element, mouseColumn, position, range };
			case MouseTargetKind.ViewZone:
				return { type: MouseTargetType.UNKNOWN, element, mouseColumn, position, range };
		}
	}
}

function readPointerId(event: PointerEvent): number | undefined {
	return Number.isFinite(event.pointerId)
		? event.pointerId
		: undefined;
}

function readClickCount(event: PointerEvent): number {
	return Number.isSafeInteger(event.detail) && event.detail > 0
		? Math.min(event.detail, 4)
		: 1;
}
