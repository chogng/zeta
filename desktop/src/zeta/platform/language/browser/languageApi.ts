import type { ViteDevAppServerConnection } from "../../app-server/browser/viteDevConnection.js";
import { viteDevRequest } from "../../app-server/browser/viteDevRequest.js";
import type { UnavailableOperation } from "../../renderer/browser/disconnectedHost.js";
import type { ILanguageApi } from "../common/languageApi.js";

export function createDisconnectedLanguageApi(unavailable: UnavailableOperation): ILanguageApi {
  return { locations: () => unavailable("language.locations"), hierarchy: () => unavailable("language.hierarchy"), workspaceSymbols: () => unavailable("language.workspaceSymbols"), prepareRename: () => unavailable("language.prepareRename"), rename: () => unavailable("language.rename"), codeActions: () => unavailable("language.codeActions"), resolveCodeAction: () => unavailable("language.resolveCodeAction") };
}

export function createViteDevLanguageApi(connection: ViteDevAppServerConnection): ILanguageApi {
  return {
    locations: params => viteDevRequest(connection, "language/locations", params),
    hierarchy: params => viteDevRequest(connection, "language/hierarchy", params),
    workspaceSymbols: params => viteDevRequest(connection, "language/workspaceSymbols", params),
    prepareRename: params => viteDevRequest(connection, "language/prepareRename", params),
    rename: params => viteDevRequest(connection, "language/rename", params),
    codeActions: params => viteDevRequest(connection, "language/codeActions", params),
    resolveCodeAction: params => viteDevRequest(connection, "language/resolveCodeAction", params),
  };
}
