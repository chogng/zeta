/**
 * Registers finite IPC routes with one shared sender, main-frame, URL, and params gate.
 */
export function registerTrustedIpcRoutes(ipcMain, target, routes) {
    const channels = new Set();
    for (const route of routes) {
        if (channels.has(route.channel)) {
            throw new Error(`Duplicate IPC route: ${route.channel}`);
        }
        channels.add(route.channel);
    }
    const registered = [];
    try {
        for (const route of routes) {
            ipcMain.handle(route.channel, (event, rawParams) => {
                requireTrustedSender(event, target);
                return route.invoke(route.validate(rawParams));
            });
            registered.push(route.channel);
        }
    }
    catch (error) {
        for (const channel of registered)
            ipcMain.removeHandler(channel);
        throw error;
    }
    return toDisposable(() => {
        for (const channel of channels)
            ipcMain.removeHandler(channel);
    });
}
export function requireTrustedSender(event, target) {
    if (event.sender !== target.webContents) {
        throw new Error("Untrusted renderer IPC sender");
    }
    if (!event.senderFrame || event.senderFrame !== event.sender.mainFrame) {
        throw new Error("Renderer IPC must originate from the main frame");
    }
    if (!target.allowedEntryUrls.has(normalizeEntryUrl(event.senderFrame.url))) {
        throw new Error("Renderer IPC URL is not allowed");
    }
}
export function normalizeEntryUrl(value) {
    return new URL(value).href;
}
import { toDisposable, } from "../../../base/common/lifecycle.js";
