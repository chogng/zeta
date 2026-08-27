import { toDisposable, type IDisposable } from '../../../base/common/lifecycle.js';

/** Observes the physical canvas size without rounding through CSS pixels. */
export function observeDevicePixelDimensions(element: HTMLCanvasElement, ownerWindow: Window, callback: (width: number, height: number) => void): IDisposable {
	const ResizeObserverConstructor = (ownerWindow as Window & { readonly ResizeObserver?: typeof ResizeObserver }).ResizeObserver;
	if (!ResizeObserverConstructor) throw new Error('WebGPU text rendering requires ResizeObserver');
	const observer = new ResizeObserverConstructor((entries: ResizeObserverEntry[]) => {
		const entry = entries.find(candidate => candidate.target === element);
		const size = entry?.devicePixelContentBoxSize?.[0];
		if (!size || size.inlineSize <= 0 || size.blockSize <= 0) return;
		callback(size.inlineSize, size.blockSize);
	});
	observer.observe(element, { box: 'device-pixel-content-box' });
	return toDisposable(() => observer.disconnect());
}

export function validatedDevicePixelRatio(ownerWindow: Window): number {
	const value = ownerWindow.devicePixelRatio;
	if (!Number.isFinite(value) || value <= 0) throw new RangeError('WebGPU device pixel ratio must be finite and positive');
	return value;
}
