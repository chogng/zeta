import type { ViteDevAppServerConnection } from "../../app-server/browser/viteDevConnection.js";
import { viteDevRequest, voidResult } from "../../app-server/browser/viteDevRequest.js";
import type { UnavailableOperation } from "../../renderer/browser/disconnectedHost.js";
import type { ITerminalProcessApi } from "../common/terminalProcessApi.js";

export function createDisconnectedTerminalProcessApi(unavailable: UnavailableOperation): ITerminalProcessApi {
  return {
    listProfiles: () => unavailable("terminal.listProfiles"),
    create: () => unavailable("terminal.create"),
    write: () => unavailable("terminal.write"),
    resize: () => unavailable("terminal.resize"),
    read: () => unavailable("terminal.read"),
    close: () => unavailable("terminal.close"),
  };
}

export function createViteDevTerminalProcessApi(connection: ViteDevAppServerConnection): ITerminalProcessApi {
  return {
    listProfiles: () => viteDevRequest(connection, "terminal/profile/list", {}),
    create: (params) => viteDevRequest(connection, "terminal/create", params),
    write: (params) => voidResult(viteDevRequest(connection, "terminal/write", params)),
    resize: (params) => voidResult(viteDevRequest(connection, "terminal/resize", params)),
    read: (params) => viteDevRequest(connection, "terminal/read", params),
    close: (params) => voidResult(viteDevRequest(connection, "terminal/close", params)),
  };
}
