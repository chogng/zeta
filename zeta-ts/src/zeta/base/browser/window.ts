import { Emitter, type Event } from "../common/event.js";
import {
	DisposableOwner,
	DisposableStore,
	type IDisposable,
} from "../common/lifecycle.js";

export type BrowserWindow = Window & typeof globalThis;

export interface IRegisteredWindow {
	readonly id: number;
	readonly window: BrowserWindow;
	readonly disposables: DisposableStore;
}

const registrations = new Map<number, IRegisteredWindow>();
const windowIds = new WeakMap<Window, number>();
const onDidRegisterEmitter = new Emitter<IRegisteredWindow>();
const onWillUnregisterEmitter = new Emitter<IRegisteredWindow>();
const onDidUnregisterEmitter = new Emitter<BrowserWindow>();
let nextWindowId = 1;

export const mainWindow = window as BrowserWindow;

const mainRegistration: IRegisteredWindow = {
	id: nextWindowId++,
	window: mainWindow,
	disposables: new DisposableStore(),
};
registrations.set(mainRegistration.id, mainRegistration);
windowIds.set(mainWindow, mainRegistration.id);

export const onDidRegisterWindow: Event<IRegisteredWindow> =
	onDidRegisterEmitter.event;
export const onWillUnregisterWindow: Event<IRegisteredWindow> =
	onWillUnregisterEmitter.event;
export const onDidUnregisterWindow: Event<BrowserWindow> =
	onDidUnregisterEmitter.event;

/**
 * Registers an auxiliary browser window and owns resources scoped to its
 * lifetime.
 */
export function registerWindow(targetWindow: Window): IDisposable {
	if (windowIds.has(targetWindow)) {
		throw new Error("Browser window is already registered");
	}

	const id = nextWindowId++;
	const lifecycle = new WindowRegistrationLifecycle();
	const registration: IRegisteredWindow = {
		id,
		window: targetWindow as BrowserWindow,
		disposables: lifecycle.disposables,
	};
	lifecycle.initialize(registration);
	registrations.set(id, registration);
	windowIds.set(targetWindow, id);
	onDidRegisterEmitter.fire(registration);
	return lifecycle;
}

export function getWindows(): readonly IRegisteredWindow[] {
	return [...registrations.values()];
}

export function getWindowById(id: number): IRegisteredWindow | undefined {
	return registrations.get(id);
}

export function getWindowId(targetWindow: Window): number | undefined {
	return windowIds.get(targetWindow);
}

export function isRegisteredWindow(targetWindow: Window): boolean {
	return windowIds.has(targetWindow);
}

/** Opens a new browsing context without exposing the opener capability. */
export function openWindowNoOpener(
	targetWindow: Window,
	url: URL,
): void {
	targetWindow.open(url.toString(), "_blank", "noopener");
}

export interface PopupWindowOptions {
	readonly width?: number;
	readonly height?: number;
	readonly left?: number;
	readonly top?: number;
}

/** Opens a popup without exposing the opener capability. */
export function openPopupWindow(
	targetWindow: Window,
	url: URL,
	options: PopupWindowOptions = {},
): void {
	const features = [
		"popup=yes",
		"noopener",
		options.width === undefined ? undefined : `width=${options.width}`,
		options.height === undefined ? undefined : `height=${options.height}`,
		options.left === undefined ? undefined : `left=${options.left}`,
		options.top === undefined ? undefined : `top=${options.top}`,
	].filter((feature): feature is string => feature !== undefined);
	targetWindow.open(url.toString(), "_blank", features.join(","));
}

/** Resolves the owning window for a node, document, event, or window. */
export function getWindow(
	source?: Node | Document | UIEvent | Window | null,
): BrowserWindow {
	if (!source) return mainWindow;
	if (isWindow(source)) return source as BrowserWindow;
	if (isDocument(source)) {
		return (source.defaultView ?? mainWindow) as BrowserWindow;
	}
	if ("ownerDocument" in source) {
		return (source.ownerDocument?.defaultView ?? mainWindow) as BrowserWindow;
	}
	return (source.view ?? mainWindow) as BrowserWindow;
}

export function getDocument(
	source?: Node | Document | UIEvent | Window | null,
): Document {
	return getWindow(source).document;
}

export function isWindow(value: unknown): value is Window {
	return typeof value === "object" &&
		value !== null &&
		"window" in value &&
		(value as Window).window === value;
}

function isDocument(value: unknown): value is Document {
	return typeof value === "object" &&
		value !== null &&
		"nodeType" in value &&
		(value as Node).nodeType === 9;
}

class WindowRegistrationLifecycle extends DisposableOwner {
	readonly disposables: DisposableStore;
	private registration: IRegisteredWindow | undefined;

	constructor() {
		super();
		this.defer(() => {
			const registration = this.registration;
			if (!registration) return;
			registrations.delete(registration.id);
			windowIds.delete(registration.window);
			onDidUnregisterEmitter.fire(registration.window);
			this.registration = undefined;
		});
		this.disposables = this.own(new DisposableStore());
		this.defer(() => {
			if (this.registration) {
				onWillUnregisterEmitter.fire(this.registration);
			}
		});
	}

	initialize(registration: IRegisteredWindow): void {
		this.registration = registration;
	}
}
