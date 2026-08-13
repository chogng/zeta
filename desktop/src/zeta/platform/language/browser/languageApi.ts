import type { ViteDevAppServerConnection } from "../../app-server/browser/viteDevConnection.js";
import { viteDevRequest, voidResult } from "../../app-server/browser/viteDevRequest.js";
import type { UnavailableOperation } from "../../renderer/browser/disconnectedHost.js";
import type { ILanguageApi } from "../common/languageApi.js";

export function createDisconnectedLanguageApi(unavailable: UnavailableOperation): ILanguageApi {
  return { synchronize: () => unavailable("language.synchronize"), close: () => unavailable("language.close"), hover: () => unavailable("language.hover"), completions: () => unavailable("language.completions"), formatDocument: () => unavailable("language.formatDocument"), formatRange: () => unavailable("language.formatRange"), signatureHelp: () => unavailable("language.signatureHelp"), inlayHints: () => unavailable("language.inlayHints"), linkedEditingRanges: () => unavailable("language.linkedEditingRanges"), locations: () => unavailable("language.locations"), hierarchy: () => unavailable("language.hierarchy"), workspaceSymbols: () => unavailable("language.workspaceSymbols"), prepareRename: () => unavailable("language.prepareRename"), rename: () => unavailable("language.rename"), codeActions: () => unavailable("language.codeActions"), resolveCodeAction: () => unavailable("language.resolveCodeAction") };
}

export function createViteDevLanguageApi(connection: ViteDevAppServerConnection): ILanguageApi {
  return {
    synchronize: params => voidResult(viteDevRequest(connection, "language/synchronize", params)),
    close: params => voidResult(viteDevRequest(connection, "language/close", params)),
    hover: params => viteDevRequest(connection, "language/hover", params),
    completions: params => viteDevRequest(connection, "language/completions", params),
    formatDocument: params => viteDevRequest(connection, "language/formatDocument", params),
    formatRange: params => viteDevRequest(connection, "language/formatRange", params),
    signatureHelp: params => viteDevRequest(connection, "language/signatureHelp", params),
    inlayHints: params => viteDevRequest(connection, "language/inlayHints", params),
    linkedEditingRanges: params => viteDevRequest(connection, "language/linkedEditingRanges", params),
    locations: params => viteDevRequest(connection, "language/locations", params),
    hierarchy: params => viteDevRequest(connection, "language/hierarchy", params),
    workspaceSymbols: params => viteDevRequest(connection, "language/workspaceSymbols", params),
    prepareRename: params => viteDevRequest(connection, "language/prepareRename", params),
    rename: params => viteDevRequest(connection, "language/rename", params),
    codeActions: params => viteDevRequest(connection, "language/codeActions", params),
    resolveCodeAction: params => viteDevRequest(connection, "language/resolveCodeAction", params),
  };
}
