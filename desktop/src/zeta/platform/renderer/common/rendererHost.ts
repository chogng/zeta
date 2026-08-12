import type { IAppServerApi, IResourceApi, IServerEventApi } from "../../app-server/common/appServerApi.js";
import type { IExtensionApi } from "../../extensions/common/extensionApi.js";
import type { IFileApi } from "../../files/common/fileApi.js";
import type { IDiffApi } from "../../diff/common/diffApi.js";
import type { ISyntaxApi } from "../../syntax/common/syntaxApi.js";
import type { IGitApi } from "../../git/common/gitApi.js";
import type { IWorkspaceSearchApi } from "../../search/common/searchApi.js";
import type { IModelApi, ISessionApi, IThreadApi, ITurnApi } from "../../sessions/common/sessionApi.js";
import type { ISkillApi } from "../../skills/common/skillApi.js";
import type { ITerminalProcessService } from "../../terminal/common/terminalProcessService.js";
import type { ITypstApi } from "../../typst/common/typstApi.js";
import type { IDocumentCollaborationApi } from "../../collaboration/common/documentCollaborationApi.js";
import type { ICodeIndexApi } from "../../codeIndex/common/codeIndexApi.js";
import type { IConnectorApi } from "../../connectors/common/connectorApi.js";
import type { IToolSearchApi } from "../../toolSearch/common/toolSearchApi.js";
import type { ILanguageApi } from "../../language/common/languageApi.js";

/** Transport-neutral capability set supplied by a renderer host at startup. */
export interface IRendererHost {
  readonly appServer: IAppServerApi;
  readonly session: ISessionApi;
  readonly model: IModelApi;
  readonly thread: IThreadApi;
  readonly turn: ITurnApi;
  readonly skills: ISkillApi;
  readonly typst: ITypstApi;
  readonly documentCollaboration: IDocumentCollaborationApi;
  readonly resource: IResourceApi;
  readonly extensions: IExtensionApi;
  readonly fs: IFileApi;
  readonly diff: IDiffApi;
  readonly syntax: ISyntaxApi;
  readonly language: ILanguageApi;
  readonly git: IGitApi;
  readonly workspaceSearch: IWorkspaceSearchApi;
  readonly terminal: ITerminalProcessService;
  readonly events: IServerEventApi;
  readonly codeIndex: ICodeIndexApi;
  readonly connectors: IConnectorApi;
  readonly toolSearch: IToolSearchApi;
}
