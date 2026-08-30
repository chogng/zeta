import { type EditContextOptions } from "../editContext.js";
import { BrowserEditContext, type NativeEditContextWindow } from "./nativeEditContext.js";

/** Returns the native constructor exposed by the editor's owner window. */
export function getNativeEditContextConstructor(container: HTMLElement): NativeEditContextWindow["EditContext"] {
	return (container.ownerDocument.defaultView as NativeEditContextWindow | null)?.EditContext;
}

export function supportsNativeEditContext(container: HTMLElement): boolean {
	return typeof getNativeEditContextConstructor(container) === "function";
}

/** Creates the native implementation after the parent selects this browser capability. */
export function createNativeEditContext(container: HTMLElement, options: EditContextOptions): BrowserEditContext {
	return new BrowserEditContext(container, options);
}
