import { createWorkbenchSession } from "../../workbench/browser/workbenchSession.js";
import { WorkbenchViewContainerId } from "../../workbench/common/views.js";
import { CHAT_AGENT_SIDEBAR_VIEW_CONTAINER_ID, CHAT_VIEW_CONTAINER_ID } from "../../workbench/contrib/chat/common/chat.js";

/** Academic product session: the document surface gets the full right edge. */
export const academicWorkbenchSession = createWorkbenchSession({
  id: "academic",
  productId: "academic",
  label: "Academic Workbench",
  layout: {
    version: 3,
    sidebar: { width: 280, visible: true },
    auxiliarybar: { width: 420, visible: false },
    agentSidebar: { width: 300, visible: false },
    panel: { height: 280, visible: true },
  },
  composition: {
    sidebar: WorkbenchViewContainerId.Sidebar,
    auxiliarybar: CHAT_VIEW_CONTAINER_ID,
    agentSidebar: CHAT_AGENT_SIDEBAR_VIEW_CONTAINER_ID,
    panel: WorkbenchViewContainerId.Problems,
  },
});
