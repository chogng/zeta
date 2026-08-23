import { bootstrapElectronMain } from "./bootstrap.js";

bootstrapElectronMain();
await import(workbenchMainEntry());

function workbenchMainEntry(): string {
	const entry = process.env.ZETA_ELECTRON_MAIN;
	if (entry === undefined || entry.length === 0) {
		return "./zeta/code/electron-main/main.js";
	}
	if (entry === "code" || entry === "academic") {
		const configuredMode = process.env.ZETA_WORKBENCH_MODE;
		if (configuredMode !== undefined && configuredMode !== entry) {
			throw new Error(`Electron Main entry '${entry}' conflicts with ZETA_WORKBENCH_MODE '${configuredMode}'`);
		}
		return entry === "code"
			? "./zeta/code/electron-main/codeMain.js"
			: "./zeta/code/electron-main/acaMain.js";
	}
	throw new TypeError(`Unknown Electron Main entry '${entry}'. Expected code or academic`);
}
