import { addDisposableListener } from "../../../base/browser/dom.js";
import { DisposableOwner } from "../../../base/common/lifecycle.js";
import { navigateToSessionsPage } from "../common/sessionNavigation.js";
import { SessionChatSurface } from "../common/sessionChatSurface.js";
import { SessionsList } from "../common/sessionsList.js";
import type { SessionsRuntime } from "../common/sessionsRuntime.js";
import type { SessionsProfile } from "../../common/sessionsProfile.js";

/** VS Code-inspired fixed agent-session workbench for the Code product. */
export class CodeSessionsWorkbench extends DisposableOwner {
  readonly element: HTMLElement;

  constructor(ownerDocument: Document, profile: SessionsProfile, runtime: SessionsRuntime) {
    super();
    this.element = ownerDocument.createElement("main");
    this.element.className = "zeta-sessions-window zeta-code-sessions-window";
    const header = ownerDocument.createElement("header");
    header.className = "zeta-sessions-titlebar";
    const returnToWorkbench = ownerDocument.createElement("button");
    returnToWorkbench.type = "button";
    returnToWorkbench.className = "zeta-sessions-titlebar-button";
    returnToWorkbench.textContent = "Workbench";
    const title = ownerDocument.createElement("div");
    title.className = "zeta-sessions-titlebar-title";
    title.textContent = profile.label;
    const newSession = ownerDocument.createElement("button");
    newSession.type = "button";
    newSession.className = "zeta-sessions-button zeta-sessions-primary-button";
    newSession.textContent = "New session";
    header.append(returnToWorkbench, title, newSession);
    const body = ownerDocument.createElement("div");
    body.className = "zeta-code-sessions-layout";
    const navigation = this.own(new SessionsList(ownerDocument, runtime.sessions, "Sessions", "New session"));
    const primary = ownerDocument.createElement("section");
    primary.className = "zeta-code-sessions-primary";
    const primaryHeading = ownerDocument.createElement("div");
    primaryHeading.className = "zeta-sessions-surface-header";
    primaryHeading.innerHTML = "<h1>Agent session</h1><p>Plan, implement, and review work without changing the regular Workbench.</p>";
    const chat = this.own(new SessionChatSurface(ownerDocument, runtime.sessions, runtime.chat, "Ask the coding agent to investigate, implement, or review…", "New code session"));
    primary.append(primaryHeading, chat.element);
    const context = ownerDocument.createElement("aside");
    context.className = "zeta-code-sessions-context";
    context.innerHTML = "<h2>Context</h2><p>Keep the workspace, changes, and terminal in the regular Workbench. This window is focused on the active agent session.</p><ul><li>Choose or create a session on the left.</li><li>Use the center pane for the agent thread.</li><li>Return to Workbench to inspect files and run tools.</li></ul>";
    body.append(navigation.element, primary, context);
    this.element.append(header, body);
    this.own(addDisposableListener(returnToWorkbench, "click", () => navigateToSessionsPage(profile.workbenchRelativePath)));
    this.own(addDisposableListener(newSession, "click", () => {
      runtime.sessions.createUntitledSession("New code session");
      chat.focus();
    }));
    void runtime.initialize();
  }
}
