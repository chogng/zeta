import "./media/sessionsPart.css";
import { Dimension } from "../../../base/browser/geometry.js";
import type { ICommandService } from "../../../platform/commands/common/commands.js";
import type { IContextMenuService } from "../../../platform/contextview/browser/contextMenu.js";
import type { IContextViewService } from "../../../platform/contextview/browser/contextView.js";
import type { IChatService } from "../../../workbench/services/chat/common/chatService.js";
import type { IWorkbenchSessionService } from "../../../workbench/services/sessions/common/sessionService.js";
import { WorkbenchPart } from "../../../workbench/browser/part.js";
import type { SessionsViewSelection } from "../../services/view/common/sessionsViewService.js";
import { SessionsChatView } from "../common/sessionsChatView.js";

export interface SessionsPartOptions {
  readonly ownerDocument: Document;
  readonly sessionService: IWorkbenchSessionService;
  readonly chatService: IChatService;
  readonly contextMenuService: IContextMenuService;
  readonly contextViewService: IContextViewService;
  readonly commandService: ICommandService;
  readonly activateSelection: (selection: SessionsViewSelection) => void;
  readonly closeSelection: (selection: SessionsViewSelection) => void;
}

/** Passive primary Part that renders the visible Sessions supplied by its owner. */
export class SessionsPart extends WorkbenchPart {
  private readonly chat: SessionsChatView;
  private readonly heading: HTMLHeadingElement;
  private readonly description: HTMLParagraphElement;

  override get minimumWidth(): number { return 420; }

  constructor(options: SessionsPartOptions) {
    super("sessions", options.ownerDocument);
    const ownerDocument = options.ownerDocument;
    const header = ownerDocument.createElement("div");
    header.className = "zeta-sessions-surface-header";
    this.heading = ownerDocument.createElement("h1");
    this.description = ownerDocument.createElement("p");
    header.append(this.heading, this.description);
    this.chat = this.own(new SessionsChatView({
      ownerDocument,
      chatService: options.chatService,
      sessionService: options.sessionService,
      contextMenuService: options.contextMenuService,
      contextViewService: options.contextViewService,
      commandService: options.commandService,
      activateSelection: options.activateSelection,
      closeSelection: options.closeSelection,
    }));
    this.contentElement.append(header, this.chat.element);
    this.updateVisibleSelections([], undefined);
  }

  focus(): void { this.chat.focus(); }

  updateVisibleSelections(selections: readonly SessionsViewSelection[], active: SessionsViewSelection | undefined): void {
    if (active?.kind === "session") {
      this.heading.textContent = active.active.session.title.trim() || "Agent session";
      this.description.textContent = `Active thread ${active.active.threadId}`;
    } else if (active?.kind === "untitled") {
      this.heading.textContent = active.session.title.trim() || "New code session";
      this.description.textContent = "This draft becomes a durable Session when the first message is sent.";
    } else {
      this.heading.textContent = "Agent sessions";
      this.description.textContent = "Plan, implement, and review work in a focused agent workspace.";
    }
    this.chat.updateVisibleSelections(selections, active);
  }

  override layout(dimension: Dimension): void {
    const bounds = this.chat.element.getBoundingClientRect();
    this.chat.layout(new Dimension(bounds.width || dimension.width, bounds.height || dimension.height));
  }
}
