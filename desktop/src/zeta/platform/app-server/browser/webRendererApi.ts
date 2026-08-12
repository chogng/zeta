import { createViteDevAppServerApi, createViteDevResourceApi, createViteDevServerEventApi } from "./appServerApi.js";
import { ViteDevAppServerConnection, type ViteDevAppServerConnectionOptions, type ViteDevAppServerMetadata, type ViteDevHotContext } from "./viteDevConnection.js";
import { createViteDevFileApi } from "../../files/browser/fileApi.js";
import { createViteDevExtensionApi } from "../../extensions/browser/extensionApi.js";
import { createViteDevDiffApi } from "../../diff/browser/diffApi.js";
import { createViteDevSyntaxApi } from "../../syntax/browser/syntaxApi.js";
import { createViteDevGitApi } from "../../git/browser/gitApi.js";
import type { IRendererHost } from "../../renderer/common/rendererHost.js";
import { createViteDevWorkspaceSearchApi } from "../../search/browser/searchApi.js";
import { createViteDevModelApi, createViteDevSessionApi, createViteDevThreadApi, createViteDevTurnApi } from "../../sessions/browser/sessionApi.js";
import { createViteDevSkillApi } from "../../skills/browser/skillApi.js";
import { ViteDevTerminalProcessService } from "../../terminal/browser/viteDevTerminalProcessService.js";
import { createViteDevTypstApi } from "../../typst/browser/typstApi.js";
import { createViteDevDocumentCollaborationApi } from "../../collaboration/browser/documentCollaborationApi.js";
import { createViteDevCodeIndexApi } from "../../codeIndex/browser/codeIndexApi.js";
import { createViteDevToolSearchApi } from "../../toolSearch/browser/toolSearchApi.js";

export interface ConnectedWebRendererApi {
  readonly api: IRendererHost;
  readonly metadata: ViteDevAppServerMetadata;
  dispose(): void;
}

/** Connects a browser Renderer host to the loopback Vite development bridge. */
export async function connectViteDevRendererApi(hot: ViteDevHotContext, options: ViteDevAppServerConnectionOptions = {}): Promise<ConnectedWebRendererApi> {
  const connection = new ViteDevAppServerConnection(hot, options);
  try {
    const metadata = await connection.connect();
    return {
      api: createRendererHost(connection),
      metadata,
      dispose: () => connection.dispose(),
    };
  } catch (error) {
    connection.dispose();
    throw error;
  }
}

function createRendererHost(connection: ViteDevAppServerConnection): IRendererHost {
  const appServer = createViteDevAppServerApi(connection);
  const resource = createViteDevResourceApi(connection);
  return {
    appServer,
    session: createViteDevSessionApi(connection),
    model: createViteDevModelApi(connection),
    thread: createViteDevThreadApi(connection),
    turn: createViteDevTurnApi(connection),
    skills: createViteDevSkillApi(connection),
    typst: createViteDevTypstApi(connection),
    documentCollaboration: createViteDevDocumentCollaborationApi(connection),
    resource,
    extensions: createViteDevExtensionApi(connection, resource),
    fs: createViteDevFileApi(connection),
    diff: createViteDevDiffApi(connection),
    syntax: createViteDevSyntaxApi(connection),
    git: createViteDevGitApi(connection),
    workspaceSearch: createViteDevWorkspaceSearchApi(connection),
    terminal: new ViteDevTerminalProcessService(connection, appServer),
    events: createViteDevServerEventApi(connection),
    codeIndex: createViteDevCodeIndexApi(connection),
    toolSearch: createViteDevToolSearchApi(connection),
  };
}
