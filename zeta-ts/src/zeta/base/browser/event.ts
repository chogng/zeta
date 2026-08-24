import { Emitter, type Event as BaseEvent } from '../common/event.js';
import { type IDisposable, markAsDisposed, setDisposableOwner, trackDisposable } from '../common/lifecycle.js';

type DomListenerOptions = boolean | AddEventListenerOptions;

/** Browser event names shared by HTML elements, documents, and windows. */
export interface DOMEventMap extends HTMLElementEventMap, DocumentEventMap, WindowEventMap {}

/** Exposes one native DOM event through the common disposable Event contract. */
export class DomEmitter<K extends keyof DOMEventMap> implements IDisposable {
	private readonly emitter: Emitter<DOMEventMap[K]>;
	private disposed = false;
	public readonly event: BaseEvent<DOMEventMap[K]>;

	public constructor(target: EventTarget, type: K, options?: DomListenerOptions) {
		const listener = (event: Event): void => this.emitter.fire(event as DOMEventMap[K]);
		const capture = typeof options === 'boolean' ? options : options?.capture ?? false;
		this.emitter = new Emitter<DOMEventMap[K]>({
			onWillAddFirstListener: () => target.addEventListener(type, listener, options),
			onDidRemoveLastListener: () => target.removeEventListener(type, listener, capture),
		});
		this.event = this.emitter.event;
		trackDisposable(this);
		setDisposableOwner(this.emitter, this);
	}

	public dispose(): void {
		if (this.disposed) return;
		this.disposed = true;
		try {
			this.emitter.dispose();
		} finally {
			markAsDisposed(this);
		}
	}

	public [Symbol.dispose](): void {
		this.dispose();
	}
}
