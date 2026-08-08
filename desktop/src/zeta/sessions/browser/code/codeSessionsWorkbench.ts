import { addDisposableListener } from "../../../base/browser/dom.js";
import { DisposableOwner } from "../../../base/common/lifecycle.js";
import type { ISessionsWindowApi } from "../../common/sessionsWindow.js";
import { returnToWorkbench } from "../common/sessionNavigation.js";
import { SessionChatSurface } from "../common/sessionChatSurface.js";
import { SessionsList } from "../common/sessionsList.js";
import type { SessionsRuntime } from "../common/sessionsRuntime.js";
import type { SessionsProfile } from "../../common/sessionsProfile.js";

/** VS Code-inspired fixed agent-session workbench for the Code product. */
export class CodeSessionsWorkbench extends DisposableOwner {
  readonly element: HTMLElement;

  constructor(ownerDocument: Document, profile: SessionsProfile, runtime: SessionsRuntime, sessionsWindowApi: ISessionsWindowApi | undefined) {
    super();
    this.element = ownerDocument.createElement("main");
    this.element.className = "zeta-sessions-window zeta-code-sessions-window";
    const header = ownerDocument.createElement("header");
    header.className = "zeta-sessions-titlebar";
    const returnButton = ownerDocument.createElement("button");
    returnButton.type = "button";
    returnButton.className = "zeta-sessions-titlebar-button";
    returnButton.textContent = "Workbench";
    const title = ownerDocument.createElement("div");
    title.className = "zeta-sessions-titlebar-title";
    title.textContent = profile.label;
    const newSession = ownerDocument.createElement("button");
    newSession.type = "button";
    newSession.className = "zeta-sessions-button";
    newSession.textContent = "New session";
    header.append(returnButton, title, newSession);
    const body = ownerDocument.createElement("div");
    body.className = "zeta-code-sessions-layout";
    const navigation = this.own(new SessionsList(ownerDocument, runtime.sessions, "Sessions", "New session"));
    const primary = ownerDocument.createElement("section");
    primary.className = "zeta-code-sessions-primary";
    const primaryHeading = ownerDocument.createElement("div");
    primaryHeading.className = "zeta-sessions-surface-header";
    primaryHeading.innerHTML = "<h1>Agent session</h1><p>Plan, implement, and review work without changing the regular Workbench.</p>";
    const chat = this.own(new SessionChatSurface(ownerDocument, runtime.sessions, runtime.chat, "Ask the coding agent to investigate, implement, or review…", "New code session", "Turn a task into an executable plan", "Describe the outcome, constraints, or a file to inspect. The agent thread stays focused here while your tools remain in Workbench."));
    primary.append(primaryHeading, chat.element);
    const context = ownerDocument.createElement("aside");
    context.className = "zeta-code-sessions-context";
    context.innerHTML = "<h2>Context</h2><p>Keep the workspace, changes, and terminal in the regular Workbench. This window is focused on the active agent session.</p><ul><li>Choose or create a session on the left.</li><li>Use the center pane for the agent thread.</li><li>Return to Workbench to inspect files and run tools.</li></ul>";
    body.append(navigation.element, primary, context);
    this.element.append(header, body);
    this.own(addDisposableListener(returnButton, "click", () => returnToWorkbench(profile.workbenchRelativePath, sessionsWindowApi)));
    this.own(addDisposableListener(newSession, "click", () => {
      runtime.sessions.createUntitledSession("New code session");
      chat.focus();
    }));
    void runtime.initialize();
  }
}
