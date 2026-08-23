import { chromium, type Browser } from "@playwright/test";
import type { AppServerTestMode } from "./testTarget.js";
import { PlaywrightDriver } from "./playwrightDriver.js";

export interface BrowserLaunchOptions {
	readonly appServerMode: AppServerTestMode;
	readonly baseURL: string;
}

export interface BrowserLaunchResult {
	readonly application: Browser;
	readonly driver: PlaywrightDriver;
}

/** Launches the browser-hosted Zeta Workbench through Playwright. */
export async function launchBrowser(options: BrowserLaunchOptions): Promise<BrowserLaunchResult> {
	const browser = await chromium.launch();
	const context = await browser.newContext();
	const page = await context.newPage();
	const consoleErrors: string[] = [];
	page.on("console", message => {
		if (message.type() === "error") consoleErrors.push(message.text());
	});
	await page.goto(options.baseURL, { waitUntil: "domcontentloaded" });
	if (options.appServerMode === "required") {
		await page.waitForFunction(
			() => (globalThis as { zetaWebWorkbenchHost?: unknown }).zetaWebWorkbenchHost !== undefined,
			undefined,
			{ timeout: 30_000 },
		);
	}

	const driver = new PlaywrightDriver(browser, page, consoleErrors);
	await driver.workbench.waitForReady();
	return { application: browser, driver };
}
