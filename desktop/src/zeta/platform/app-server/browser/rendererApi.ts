import { createDisconnectedAppServerApi, createDisconnectedResourceApi, createDisconnectedServerEventApi } from "./appServerApi.js";
import { createDisconnectedFileApi } from "../../files/browser/fileApi.js";
import { createDisconnectedExtensionApi } from "../../extensions/browser/extensionApi.js";
import { createDisconnectedDiffApi } from "../../diff/browser/diffApi.js";
import { createDisconnectedSyntaxApi } from "../../syntax/browser/syntaxApi.js";
import { createDisconnectedGitApi } from "../../git/browser/gitApi.js";
import type { IRendererHost } from "../../renderer/common/rendererHost.js";
import { unavailableOperation, WebAppServerUnavailableError } from "../../renderer/browser/disconnectedHost.js";
import { createDisconnectedWorkspaceSearchApi } from "../../search/browser/searchApi.js";
import { createDisconnectedModelApi, createDisconnectedSessionApi, createDisconnectedThreadApi, createDisconnectedTurnApi } from "../../sessions/browser/sessionApi.js";
import { createDisconnectedSkillApi } from "../../skills/browser/skillApi.js";
import { DisconnectedTerminalProcessService } from "../../terminal/browser/disconnectedTerminalProcessService.js";
import { createDisconnectedTypstApi } from "../../typst/browser/typstApi.js";
import { createDisconnectedDocumentCollaborationApi } from "../../collaboration/browser/documentCollaborationApi.js";

export { WebAppServerUnavailableError };

/** Composes the explicit disconnected capability set for standalone Web pages. */
export function createDisconnectedRendererApi(): IRendererHost {
  const appServer = createDisconnectedAppServerApi(unavailableOperation);
  return {
    appServer,
    session: createDisconnectedSessionApi(unavailableOperation),
    model: createDisconnectedModelApi(unavailableOperation),
    thread: createDisconnectedThreadApi(unavailableOperation),
    turn: createDisconnectedTurnApi(unavailableOperation),
    skills: createDisconnectedSkillApi(unavailableOperation),
    typst: createDisconnectedTypstApi(unavailableOperation),
    documentCollaboration: createDisconnectedDocumentCollaborationApi(unavailableOperation),
    resource: createDisconnectedResourceApi(unavailableOperation),
    extensions: createDisconnectedExtensionApi(unavailableOperation),
    fs: createDisconnectedFileApi(unavailableOperation),
    diff: createDisconnectedDiffApi(unavailableOperation),
    syntax: createDisconnectedSyntaxApi(unavailableOperation),
    git: createDisconnectedGitApi(unavailableOperation),
    workspaceSearch: createDisconnectedWorkspaceSearchApi(unavailableOperation),
    terminal: new DisconnectedTerminalProcessService(unavailableOperation, appServer),
    events: createDisconnectedServerEventApi(),
  };
}
