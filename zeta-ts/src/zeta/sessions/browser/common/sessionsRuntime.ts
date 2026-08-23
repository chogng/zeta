import { DisposableOwner } from "../../../base/common/lifecycle.js";
import { ServiceCollection } from "../../../platform/instantiation/common/instantiation.js";
import type { IRendererHost } from "../../../platform/renderer/common/rendererHost.js";
import type { IWorkspaceContextApi } from "../../../platform/workspace/common/workspaceIpc.js";
import type { IConfigurationService } from "../../../platform/configuration/common/configurationService.js";
import { isSingleFolderWorkspaceIdentifier, parseWorkspaceIdentifier } from "../../../platform/workspace/common/workspace.js";
import { getRemoteWorkspacePath, isRemoteResource } from "../../../platform/remote/common/remote.js";
import { ChatService } from "../../../workbench/services/chat/browser/chatService.js";
import { IChatService } from "../../../workbench/services/chat/common/chatService.js";
import { AppServerSessionsManagementService } from "../../services/sessions/browser/appServerSessionsManagementService.js";
import { ISessionsManagementService } from "../../services/sessions/common/sessionsManagementService.js";
import { SessionsViewService } from "../../services/view/browser/sessionsViewService.js";
import { ISessionsViewService } from "../../services/view/common/sessionsViewService.js";
import type { ISessionsWindowApi } from "../../common/sessionsWindow.js";

export interface SessionsRuntimeOptions {
  readonly sessionsWindowApi?: ISessionsWindowApi;
  readonly workspaceApi?: IWorkspaceContextApi;
  readonly configurationService?: IConfigurationService;
}

/** Shared App Server-backed state used by one dedicated Sessions renderer. */
export class SessionsRuntime extends DisposableOwner {
  readonly services = new ServiceCollection();
  readonly sessions: AppServerSessionsManagementService;
  readonly view: SessionsViewService;
  readonly chat: ChatService;

  private currentWorkspaceRoot: string | undefined;

  constructor(api: IRendererHost, options: SessionsRuntimeOptions = {}) {
    super();
    this.sessions = this.own(new AppServerSessionsManagementService({
      session: api.session,
      events: api.events,
      ...(options.sessionsWindowApi ? {
        workspaceRouter: {
          currentWorkspaceRoot: () => this.currentWorkspaceRoot,
          reopenWorkspace: (root: string) => options.sessionsWindowApi!.openWorkspace(root),
        },
      } : {}),
    }));
    this.view = this.own(new SessionsViewService(this.sessions));
    this.chat = this.own(new ChatService({
      modelApi: api.model,
      threadApi: api.thread,
      turnApi: api.turn,
      skillApi: api.skills,
      appServerApi: api.appServer,
      eventApi: api.events,
      ...(options.configurationService ? { configurationService: options.configurationService } : {}),
    }));
    this.services.set(ISessionsManagementService, this.sessions);
    this.services.set(ISessionsViewService, this.view);
    this.services.set(IChatService, this.chat);
    if (options.workspaceApi) {
      const subscription = options.workspaceApi.onDidChange(workspace => this.updateWorkspaceRoot(workspace));
      this.defer(() => subscription.dispose());
    }
    this.workspaceApi = options.workspaceApi;
  }

  private readonly workspaceApi: IWorkspaceContextApi | undefined;

  async initialize(): Promise<void> {
    if (this.workspaceApi) this.updateWorkspaceRoot(await this.workspaceApi.getWorkspace());
    await this.view.initialize();
    if (!this.view.activeSelection) this.view.openNewSession("New code session");
  }

  private updateWorkspaceRoot(value: unknown): void {
    const workspace = parseWorkspaceIdentifier(value);
    this.currentWorkspaceRoot = isSingleFolderWorkspaceIdentifier(workspace)
      ? isRemoteResource(workspace.uri) ? getRemoteWorkspacePath(workspace.uri) : workspace.uri.fsPath
      : undefined;
  }
}
