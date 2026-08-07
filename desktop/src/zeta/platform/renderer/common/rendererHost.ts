import type { IAppServerApi, IResourceApi, IServerEventApi } from "../../app-server/common/appServerApi.js";
import type { IExtensionApi } from "../../extensions/common/extensionApi.js";
import type { IFileApi } from "../../files/common/fileApi.js";
import type { IDiffApi } from "../../diff/common/diffApi.js";
import type { ISyntaxApi } from "../../syntax/common/syntaxApi.js";
import type { IGitApi } from "../../git/common/gitApi.js";
import type { IWorkspaceSearchApi } from "../../search/common/searchApi.js";
import type { IModelApi, ISessionApi, IThreadApi, ITurnApi } from "../../sessions/common/sessionApi.js";
import type { ITerminalProcessService } from "../../terminal/common/terminalProcessService.js";
import type { ITypstApi } from "../../typst/common/typstApi.js";

/** Transport-neutral capability set supplied by a renderer host at startup. */
export interface IRendererHost {
  readonly appServer: IAppServerApi;
  readonly session: ISessionApi;
  readonly model: IModelApi;
  readonly thread: IThreadApi;
  readonly turn: ITurnApi;
  readonly typst: ITypstApi;
  readonly resource: IResourceApi;
  readonly extensions: IExtensionApi;
  readonly fs: IFileApi;
  readonly diff: IDiffApi;
  readonly syntax: ISyntaxApi;
  readonly git: IGitApi;
  readonly workspaceSearch: IWorkspaceSearchApi;
  readonly terminal: ITerminalProcessService;
  readonly events: IServerEventApi;
}
