import type {
	ISandboxGlobals,
} from "../common/sandboxTypes.js";

interface ISandboxGlobal {
	readonly zeta?: ISandboxGlobals;
}

const globals = (globalThis as unknown as ISandboxGlobal).zeta;
if (!globals) {
	throw new Error("Zeta sandbox preload bridge is unavailable");
}

/** IPC capability installed by the sandbox preload. */
export const ipcRenderer = globals.ipcRenderer;

/** Read-only process metadata installed by the sandbox preload. */
export const sandboxProcess = globals.process;

/** Browser-object helpers installed by the sandbox preload. */
export const sandboxWebUtils = globals.webUtils;
