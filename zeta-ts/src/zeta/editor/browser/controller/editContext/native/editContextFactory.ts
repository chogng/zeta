import { type NativeEditContextObject, type NativeEditContextWindow } from './nativeEditContext.js';

export namespace EditContext {
	export function create(window: Window, options?: unknown): NativeEditContextObject {
		const Constructor = (window as NativeEditContextWindow).EditContext;
		if (typeof Constructor !== 'function') throw new Error('The EditContext API is unavailable');
		return new Constructor(options);
	}
}
