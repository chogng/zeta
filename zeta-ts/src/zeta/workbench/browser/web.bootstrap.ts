import { URI } from "../../base/common/uri.js";
import { WorkbenchModeRegistry, type WorkbenchModeId } from "../../product/common/workbenchMode.js";
import { connectViteDevRendererApi, type ViteDevRendererCapabilityContribution } from "../../platform/app-server/browser/webRendererApi.js";
import { BrowserClipboardService } from "../../platform/clipboard/browser/browserClipboardService.js";
import { BrowserOpenerService } from "../../platform/opener/browser/browserOpenerService.js";
import { startWebWorkbench } from "./web.factory.js";

declare const __ZETA_WEB_APP_SERVER__: boolean;

/** Starts a Workbench mode after resolving its optional development host. */
export function startBrowserWorkbench(modeId: WorkbenchModeId, rendererCapabilities: readonly ViteDevRendererCapabilityContribution[] = []): void {
	document.title = WorkbenchModeRegistry.get(modeId).title;
	void startBrowserWorkbenchAsync(modeId, rendererCapabilities);
}

async function startBrowserWorkbenchAsync(modeId: WorkbenchModeId, rendererCapabilities: readonly ViteDevRendererCapabilityContribution[]): Promise<void> {
	if (globalThis.zetaWebWorkbenchHost !== undefined || !__ZETA_WEB_APP_SERVER__) {
		startWebWorkbench(modeId);
		return;
	}
	const hot = import.meta.hot;
	if (!hot) {
		console.error("Zeta Web App Server development mode requires the Vite hot channel");
		startWebWorkbench(modeId);
		return;
	}
	let disposeConnectedHost: (() => void) | undefined;
	try {
		const connected = await connectViteDevRendererApi(hot, {
			openerService: new BrowserOpenerService(window),
			clipboardService: new BrowserClipboardService(window.navigator.clipboard),
		}, {}, rendererCapabilities);
		globalThis.zetaWebWorkbenchHost = {
			api: connected.api,
			workspace: Object.freeze({
				id: connected.metadata.workspaceId,
				uri: URI.file(connected.metadata.workspaceRoot),
			}),
		};
		disposeConnectedHost = () => connected.dispose();
	} catch (error) {
		console.error("Failed to connect the Zeta Web development host", error);
	}
	try {
		startWebWorkbench(modeId);
	} catch (error) {
		disposeConnectedHost?.();
		throw error;
	}
	if (disposeConnectedHost) window.addEventListener("pagehide", disposeConnectedHost, { once: true });
}
