import { operatingSystemFromNodePlatform, operatingSystemFromUserAgent, } from "./environment.js";
/** Host operating systems that affect keybinding resolution and labels. */
export var OperatingSystem;
(function (OperatingSystem) {
    OperatingSystem["Windows"] = "windows";
    OperatingSystem["Macintosh"] = "mac";
    OperatingSystem["Linux"] = "linux";
})(OperatingSystem || (OperatingSystem = {}));
const runtimeGlobal = globalThis;
function detectEnvironment() {
    const bridgedEnvironment = runtimeGlobal.zeta?.environment;
    if (bridgedEnvironment) {
        return {
            runtime: bridgedEnvironment.runtime,
            os: bridgedEnvironment.os,
            arch: bridgedEnvironment.arch,
        };
    }
    const nodeProcess = runtimeGlobal.process;
    if (typeof nodeProcess?.versions?.node === "string") {
        return {
            runtime: typeof nodeProcess.versions.electron === "string"
                ? "electron"
                : "node",
            os: operatingSystemFromNodePlatform(nodeProcess.platform),
            arch: nodeProcess.arch,
        };
    }
    if (runtimeGlobal.navigator) {
        return {
            runtime: "web",
            os: operatingSystemFromUserAgent(runtimeGlobal.navigator.userAgent),
        };
    }
    return {
        runtime: "unknown",
        os: "unknown",
    };
}
/** Runtime and host-OS information detected once for the current environment. */
export const environment = Object.freeze(detectEnvironment());
export const isWindows = environment.os === "windows";
export const isMacintosh = environment.os === "mac";
export const isLinux = environment.os === "linux";
export const isNative = environment.runtime === "electron" || environment.runtime === "node";
export const isWeb = environment.runtime === "web";
/**
 * The host OS used for keyboard shortcut resolution and labels.
 *
 * Unknown environments use Linux semantics, matching VS Code's fallback.
 */
export const operatingSystem = isMacintosh
    ? OperatingSystem.Macintosh
    : isWindows
        ? OperatingSystem.Windows
        : OperatingSystem.Linux;
