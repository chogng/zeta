import { type EditContextOptions } from "../editContext.js";
import { NativeEditContext, type NativeEditContextWindow } from "./nativeEditContext.js";

/** Returns the native constructor exposed by the editor's owner window. */
export function getNativeEditContextConstructor(container: HTMLElement): NativeEditContextWindow["EditContext"] {
	return (container.ownerDocument.defaultView as NativeEditContextWindow | null)?.EditContext;
}

export function supportsNativeEditContext(container: HTMLElement): boolean {
	return typeof getNativeEditContextConstructor(container) === "function";
}

/** Creates the native implementation; fallback selection belongs to the parent factory. */
export function createNativeEditContext(container: HTMLElement, options: EditContextOptions = {}): NativeEditContext {
	return new NativeEditContext(container, options);
}
