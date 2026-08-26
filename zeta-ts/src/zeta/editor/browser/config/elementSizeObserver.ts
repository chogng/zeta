import { getClientArea, type IDimension, Dimension } from '../../../base/browser/geometry.js';
import { observeElementSize } from '../../../base/browser/observer.js';
import { Emitter, type Event } from '../../../base/common/event.js';
import { DisposableOwner, DisposableSlot, type IDisposable } from '../../../base/common/lifecycle.js';
import { isFiniteNumber } from '../../../base/common/numbers.js';

/**
 * Caches one editor host size and publishes only actual size changes.
 *
 * The browser observer is an implementation detail. Consumers receive the
 * same dimension shape for ResizeObserver and explicit initial observation,
 * which keeps layout ownership in the viewport rather than in the DOM API.
 */
export class ElementSizeObserver extends DisposableOwner {
	private readonly changeEmitter = this.own(new Emitter<IDimension>());
	private readonly observation = this.own(new DisposableSlot<IDisposable>());
	private currentSize: Dimension | undefined;

	readonly onDidChange: Event<IDimension> = this.changeEmitter.event;

	constructor(private readonly element: HTMLElement) {
		super();
	}

	get size(): IDimension | undefined {
		return this.currentSize;
	}

	startObserving(): void {
		this.observation.replace(observeElementSize(
			this.element,
			size => this.observe(size),
			{ box: 'content-box' },
		));
	}

	stopObserving(): void {
		this.observation.clear();
	}

	/** Publishes a caller-supplied size, normally for the initial layout. */
	observe(size: IDimension): void {
		if (!isFiniteNumber(size.width) || size.width < 0 || !isFiniteNumber(size.height) || size.height < 0) {
			throw new RangeError('Editor element size must be finite and non-negative');
		}
		if (this.currentSize?.width === size.width && this.currentSize.height === size.height) return;
		this.currentSize = new Dimension(size.width, size.height);
		this.changeEmitter.fire(this.currentSize);
	}

	/** Reads the current client area when no ResizeObserver event has arrived. */
	observeNow(): void {
		this.observe(getClientArea(this.element));
	}
}
