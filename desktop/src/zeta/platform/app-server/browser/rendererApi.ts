import { createDisconnectedAppServerApi, createDisconnectedResourceApi, createDisconnectedServerEventApi } from "./appServerApi.js";
import { createDisconnectedFileApi } from "../../files/browser/fileApi.js";
import { createDisconnectedGitApi } from "../../git/browser/gitApi.js";
import type { IRendererHost } from "../../renderer/common/rendererHost.js";
import { unavailableOperation, WebAppServerUnavailableError } from "../../renderer/browser/disconnectedHost.js";
import { createDisconnectedWorkspaceSearchApi } from "../../search/browser/searchApi.js";
import { createDisconnectedModelApi, createDisconnectedSessionApi, createDisconnectedThreadApi, createDisconnectedTurnApi } from "../../sessions/browser/sessionApi.js";
import { createDisconnectedTerminalProcessApi } from "../../terminal/browser/terminalProcessApi.js";
import { createDisconnectedTypstApi } from "../../typst/browser/typstApi.js";

export { WebAppServerUnavailableError };

/** Composes the explicit disconnected capability set for standalone Web pages. */
export function createDisconnectedRendererApi(): IRendererHost {
  return {
    appServer: createDisconnectedAppServerApi(unavailableOperation),
    session: createDisconnectedSessionApi(unavailableOperation),
    model: createDisconnectedModelApi(unavailableOperation),
    thread: createDisconnectedThreadApi(unavailableOperation),
    turn: createDisconnectedTurnApi(unavailableOperation),
    typst: createDisconnectedTypstApi(unavailableOperation),
    resource: createDisconnectedResourceApi(unavailableOperation),
    fs: createDisconnectedFileApi(unavailableOperation),
    git: createDisconnectedGitApi(unavailableOperation),
    workspaceSearch: createDisconnectedWorkspaceSearchApi(unavailableOperation),
    terminal: createDisconnectedTerminalProcessApi(unavailableOperation),
    events: createDisconnectedServerEventApi(),
  };
}
