import { addDisposableListener, getWindow } from '../../../base/browser/dom.js';
import { Emitter, type Event } from '../../../base/common/event.js';
import { Disposable, type IDisposable } from '../../../base/common/lifecycle.js';

export interface PointerTrackingHandlers {
	readonly onMove: (event: PointerEvent) => void;
	readonly onUp: (event: PointerEvent) => void;
	readonly onCancel: (event: PointerEvent) => void;
	readonly onBlur: () => void;
}

/** Owns browser pointer dispatch, capture, and one-window drag sessions. */
export class PointerHandler extends Disposable {
	private readonly pointerDownEmitter = this._register(new Emitter<PointerEvent>());
	private readonly contextMenuEmitter = this._register(new Emitter<MouseEvent>());

	readonly onDidPointerDown: Event<PointerEvent> = this.pointerDownEmitter.event;
	readonly onDidContextMenu: Event<MouseEvent> = this.contextMenuEmitter.event;
	readonly targetWindow: Window;

	constructor(readonly element: HTMLElement) {
		super();
		this.targetWindow = getWindow(element);
		this._register(addDisposableListener<PointerEvent>(
			element,
			'pointerdown',
			event => this.pointerDownEmitter.fire(event),
		));
		this._register(addDisposableListener<MouseEvent>(
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

class PointerTrackingSession extends Disposable {
	constructor(
		targetWindow: Window,
		handlers: PointerTrackingHandlers,
	) {
		super();
		this._register(addDisposableListener<PointerEvent>(targetWindow, 'pointermove', handlers.onMove));
		this._register(addDisposableListener<PointerEvent>(targetWindow, 'pointerup', handlers.onUp));
		this._register(addDisposableListener<PointerEvent>(targetWindow, 'pointercancel', handlers.onCancel));
		this._register(addDisposableListener(targetWindow, 'blur', handlers.onBlur, { once: true }));
	}
}
