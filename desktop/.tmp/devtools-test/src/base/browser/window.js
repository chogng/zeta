import { Emitter } from "../common/event.js";
import { DisposableOwner, DisposableStore, } from "../common/lifecycle.js";
const registrations = new Map();
const windowIds = new WeakMap();
const onDidRegisterEmitter = new Emitter();
const onWillUnregisterEmitter = new Emitter();
const onDidUnregisterEmitter = new Emitter();
let nextWindowId = 1;
export const mainWindow = window;
const mainRegistration = {
    id: nextWindowId++,
    window: mainWindow,
    disposables: new DisposableStore(),
};
registrations.set(mainRegistration.id, mainRegistration);
windowIds.set(mainWindow, mainRegistration.id);
export const onDidRegisterWindow = onDidRegisterEmitter.event;
export const onWillUnregisterWindow = onWillUnregisterEmitter.event;
export const onDidUnregisterWindow = onDidUnregisterEmitter.event;
/**
 * Registers an auxiliary browser window and owns resources scoped to its
 * lifetime.
 */
export function registerWindow(targetWindow) {
    if (windowIds.has(targetWindow)) {
        throw new Error("Browser window is already registered");
    }
    const id = nextWindowId++;
    const lifecycle = new WindowRegistrationLifecycle();
    const registration = {
        id,
        window: targetWindow,
        disposables: lifecycle.disposables,
    };
    lifecycle.initialize(registration);
    registrations.set(id, registration);
    windowIds.set(targetWindow, id);
    onDidRegisterEmitter.fire(registration);
    return lifecycle;
}
export function getWindows() {
    return [...registrations.values()];
}
export function getWindowById(id) {
    return registrations.get(id);
}
export function getWindowId(targetWindow) {
    return windowIds.get(targetWindow);
}
export function isRegisteredWindow(targetWindow) {
    return windowIds.has(targetWindow);
}
/** Opens a new browsing context without exposing the opener capability. */
export function openWindowNoOpener(targetWindow, url) {
    targetWindow.open(url.toString(), "_blank", "noopener");
}
/** Opens a popup without exposing the opener capability. */
export function openPopupWindow(targetWindow, url, options = {}) {
    const features = [
        "popup=yes",
        "noopener",
        options.width === undefined ? undefined : `width=${options.width}`,
        options.height === undefined ? undefined : `height=${options.height}`,
        options.left === undefined ? undefined : `left=${options.left}`,
        options.top === undefined ? undefined : `top=${options.top}`,
    ].filter((feature) => feature !== undefined);
    targetWindow.open(url.toString(), "_blank", features.join(","));
}
/** Resolves the owning window for a node, document, event, or window. */
export function getWindow(source) {
    if (!source)
        return mainWindow;
    if (isWindow(source))
        return source;
    if (isDocument(source)) {
        return (source.defaultView ?? mainWindow);
    }
    if ("ownerDocument" in source) {
        return (source.ownerDocument?.defaultView ?? mainWindow);
    }
    return (source.view ?? mainWindow);
}
export function getDocument(source) {
    return getWindow(source).document;
}
export function isWindow(value) {
    return typeof value === "object" &&
        value !== null &&
        "window" in value &&
        value.window === value;
}
function isDocument(value) {
    return typeof value === "object" &&
        value !== null &&
        "nodeType" in value &&
        value.nodeType === Node.DOCUMENT_NODE;
}
class WindowRegistrationLifecycle extends DisposableOwner {
    disposables;
    #registration;
    constructor() {
        super();
        this.defer(() => {
            const registration = this.#registration;
            if (!registration)
                return;
            registrations.delete(registration.id);
            windowIds.delete(registration.window);
            onDidUnregisterEmitter.fire(registration.window);
            this.#registration = undefined;
        });
        this.disposables = this.own(new DisposableStore());
        this.defer(() => {
            if (this.#registration) {
                onWillUnregisterEmitter.fire(this.#registration);
            }
        });
    }
    initialize(registration) {
        this.#registration = registration;
    }
}
