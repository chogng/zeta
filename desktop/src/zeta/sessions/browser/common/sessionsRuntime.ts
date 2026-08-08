import { DisposableOwner } from "../../../base/common/lifecycle.js";
import type { IRendererHost } from "../../../platform/renderer/common/rendererHost.js";
import { ChatService } from "../../../workbench/services/chat/browser/chatService.js";
import { WorkbenchSessionService } from "../../../workbench/services/sessions/browser/sessionService.js";

/** Shared App Server-backed state used by one dedicated Sessions renderer. */
export class SessionsRuntime extends DisposableOwner {
  readonly sessions: WorkbenchSessionService;
  readonly chat: ChatService;

  constructor(api: IRendererHost) {
    super();
    this.sessions = this.own(new WorkbenchSessionService(api.session));
    this.chat = this.own(new ChatService({
      modelApi: api.model,
      threadApi: api.thread,
      turnApi: api.turn,
      appServerApi: api.appServer,
      eventApi: api.events,
    }));
  }

  initialize(): Promise<void> {
    return this.sessions.initialize();
  }
}
