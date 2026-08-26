import { addDisposableListener } from '../../../base/browser/dom.js';
import { getWindow } from '../../../base/browser/window.js';
import { Emitter, type Event } from '../../../base/common/event.js';
import { DisposableOwner, type IDisposable } from '../../../base/common/lifecycle.js';

export interface PointerTrackingHandlers {
	readonly onMove: (event: PointerEvent) => void;
	readonly onUp: (event: PointerEvent) => void;
	readonly onCancel: (event: PointerEvent) => void;
	readonly onBlur: () => void;
}

/**
 * Adapts browser pointer dispatch and pointer capture into disposable input
 * sessions. It deliberately does not decide what a pointer gesture means.
 */
/** Owns browser Pointer dispatch, capture, and one-window drag sessions. */
export class PointerHandler extends DisposableOwner {
	private readonly pointerDownEmitter = this.own(new Emitter<PointerEvent>());
	private readonly contextMenuEmitter = this.own(new Emitter<MouseEvent>());

	readonly onDidPointerDown: Event<PointerEvent> = this.pointerDownEmitter.event;
	readonly onDidContextMenu: Event<MouseEvent> = this.contextMenuEmitter.event;
	readonly targetWindow: Window;

	constructor(readonly element: HTMLElement) {
		super();
		this.targetWindow = getWindow(element);
		this.own(addDisposableListener<PointerEvent>(
			element,
			'pointerdown',
			event => this.pointerDownEmitter.fire(event),
		));
		this.own(addDisposableListener<MouseEvent>(
			element,
			'contextmenu',
			event => this.contextMenuEmitter.fire(event),
		));
	}

	startTracking(pointerId: number | undefined, handlers: PointerTrackingHandlers): IDisposable {
		return new PointerTrackingSession(this.targetWindow, handlers);
	}

	capturePointer(pointerId: number | undefined): void {
		if (
			pointerId !== undefined &&
			typeof this.element.setPointerCapture === 'function'
		) {
			this.element.setPointerCapture(pointerId);
		}
	}

	releasePointer(pointerId: number | undefined): void {
		if (
			pointerId !== undefined &&
			typeof this.element.hasPointerCapture === 'function' &&
			this.element.hasPointerCapture(pointerId)
		) {
			this.element.releasePointerCapture(pointerId);
		}
	}
}

class PointerTrackingSession extends DisposableOwner {
	constructor(
		targetWindow: Window,
		handlers: PointerTrackingHandlers,
	) {
		super();
		this.own(addDisposableListener<PointerEvent>(targetWindow, 'pointermove', handlers.onMove));
		this.own(addDisposableListener<PointerEvent>(targetWindow, 'pointerup', handlers.onUp));
		this.own(addDisposableListener<PointerEvent>(targetWindow, 'pointercancel', handlers.onCancel));
		this.own(addDisposableListener(targetWindow, 'blur', handlers.onBlur, { once: true }));
	}
}
