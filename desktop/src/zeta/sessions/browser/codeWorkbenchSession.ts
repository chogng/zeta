import { createWorkbenchSession } from "../../workbench/browser/workbenchSession.js";
import { WorkbenchViewContainerId } from "../../workbench/common/views.js";
import { CHAT_AGENT_SIDEBAR_VIEW_CONTAINER_ID, CHAT_VIEW_CONTAINER_ID } from "../../workbench/contrib/chat/common/chat.js";

/** Code product session: Explorer, Chat, and Terminal are the initial surfaces. */
export const codeWorkbenchSession = createWorkbenchSession({
  id: "code",
  productId: "code",
  label: "Code Workbench",
  layout: {
    version: 3,
    sidebar: { width: 240, visible: true },
    auxiliarybar: { width: 380, visible: true },
    agentSidebar: { width: 300, visible: false },
    panel: { height: 240, visible: true },
  },
  composition: {
    sidebar: WorkbenchViewContainerId.Sidebar,
    auxiliarybar: CHAT_VIEW_CONTAINER_ID,
    agentSidebar: CHAT_AGENT_SIDEBAR_VIEW_CONTAINER_ID,
    panel: WorkbenchViewContainerId.Terminal,
  },
});
