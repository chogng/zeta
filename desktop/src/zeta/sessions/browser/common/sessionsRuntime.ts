import { DisposableOwner } from "../../../base/common/lifecycle.js";
import { ServiceCollection } from "../../../platform/instantiation/common/instantiation.js";
import type { IRendererHost } from "../../../platform/renderer/common/rendererHost.js";
import { ChatService } from "../../../workbench/services/chat/browser/chatService.js";
import { IChatService } from "../../../workbench/services/chat/common/chatService.js";
import { WorkbenchSessionService } from "../../../workbench/services/sessions/browser/sessionService.js";
import { IWorkbenchSessionService } from "../../../workbench/services/sessions/common/sessionService.js";
import { SessionsViewService } from "../../services/view/browser/sessionsViewService.js";
import { ISessionsViewService } from "../../services/view/common/sessionsViewService.js";

/** Shared App Server-backed state used by one dedicated Sessions renderer. */
export class SessionsRuntime extends DisposableOwner {
  readonly services = new ServiceCollection();
  readonly sessions: WorkbenchSessionService;
  readonly view: SessionsViewService;
  readonly chat: ChatService;

  constructor(api: IRendererHost) {
    super();
    this.sessions = this.own(new WorkbenchSessionService({ session: api.session, events: api.events }));
    this.view = this.own(new SessionsViewService(this.sessions));
    this.chat = this.own(new ChatService({
      modelApi: api.model,
      threadApi: api.thread,
      turnApi: api.turn,
      skillApi: api.skills,
      appServerApi: api.appServer,
      eventApi: api.events,
    }));
    this.services.set(IWorkbenchSessionService, this.sessions);
    this.services.set(ISessionsViewService, this.view);
    this.services.set(IChatService, this.chat);
  }

  async initialize(): Promise<void> {
    await this.view.initialize();
    if (!this.view.activeSelection) this.view.openNewSession("New code session");
  }
}
