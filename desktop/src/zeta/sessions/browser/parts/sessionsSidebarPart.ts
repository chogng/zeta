import "./media/sessionsSidebarPart.css";
import type { IWorkbenchSessionService } from "../../../workbench/services/sessions/common/sessionService.js";
import type { ISessionsViewService } from "../../services/view/common/sessionsViewService.js";
import { WorkbenchPart } from "../../../workbench/browser/part.js";
import { SessionsList } from "../common/sessionsList.js";

/** Session navigation Part for the dedicated Sessions Workbench. */
export class SessionsSidebarPart extends WorkbenchPart {
  private readonly list: SessionsList;

  override get minimumWidth(): number { return 190; }
  override get maximumWidth(): number { return 460; }

  constructor(ownerDocument: Document, sessionService: IWorkbenchSessionService, viewService: ISessionsViewService) {
    super("sidebar", ownerDocument);
    this.list = this.own(new SessionsList(ownerDocument, sessionService, viewService, "Sessions", "New session"));
    this.contentElement.append(this.list.element);
  }

  focus(): void { this.list.focus(); }
}
