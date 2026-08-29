import { Disposable, toDisposable } from "../../../base/common/lifecycle.js";
import { ServiceContainer } from "../../../platform/instantiation/common/instantiation.js";
import type { IRendererHost } from "../../../platform/renderer/common/rendererHost.js";
import type { IWorkspaceContextApi } from "../../../platform/workspace/common/workspaceIpc.js";
import type { IConfigurationService } from "../../../platform/configuration/common/configurationService.js";
import { parseWorkspace, workspaceOpenTarget } from "../../../platform/workspace/common/workspace.js";
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
export class SessionsRuntime extends Disposable {
	readonly container: ServiceContainer;
	readonly sessions: AppServerSessionsManagementService;
	readonly view: SessionsViewService;
	readonly chat: ChatService;

	private currentWorkspaceRoot: string | undefined;

	constructor(api: IRendererHost, options: SessionsRuntimeOptions = {}) {
		super();
		this.container = this._register(new ServiceContainer());
		this.sessions = this._register(new AppServerSessionsManagementService({
			session: api.session,
			turn: api.turn,
			events: api.events,
			...(options.sessionsWindowApi ? {
				workspaceRouter: {
					currentWorkspaceRoot: () => this.currentWorkspaceRoot,
					reopenWorkspace: (root: string) => options.sessionsWindowApi!.openWorkspace(root),
				},
			} : {}),
		}));
		this.view = this._register(new SessionsViewService(this.sessions));
		this.chat = this._register(new ChatService({
			modelApi: api.model,
			threadApi: api.thread,
			turnApi: api.turn,
			turnChangesApi: api.turnChanges,
			skillApi: api.skills,
			appServerApi: api.appServer,
			eventApi: api.events,
			...(options.configurationService ? { configurationService: options.configurationService } : {}),
		}));
		this.container.registerInstance(ISessionsManagementService, this.sessions);
		this.container.registerInstance(ISessionsViewService, this.view);
		this.container.registerInstance(IChatService, this.chat);
		if (options.workspaceApi) {
			const subscription = options.workspaceApi.onDidChange(workspace => this.updateWorkspaceRoot(workspace));
			this._register(toDisposable(() => subscription.dispose()));
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
		const workspace = parseWorkspace(value);
		this.currentWorkspaceRoot = workspaceOpenTarget(workspace);
	}
}
