import { WorkbenchViewContainerId } from "../common/views.js";
import { CHAT_AGENT_SIDEBAR_VIEW_CONTAINER_ID, CHAT_VIEW_CONTAINER_ID } from "../contrib/chat/common/chat.js";
import { createDefaultWorkbenchLayoutState } from "./layout/workbenchLayoutState.js";
import { createWorkbenchProfile } from "./workbenchProfile.js";

/** Shared initial Workbench UI profile used by every build mode. */
export const defaultWorkbenchProfile = createWorkbenchProfile({
  id: "default",
  label: "Workbench",
  layout: createDefaultWorkbenchLayoutState(),
  composition: {
    sidebar: WorkbenchViewContainerId.Sidebar,
    auxiliarybar: CHAT_VIEW_CONTAINER_ID,
    agentSidebar: CHAT_AGENT_SIDEBAR_VIEW_CONTAINER_ID,
    panel: WorkbenchViewContainerId.Terminal,
  },
});
