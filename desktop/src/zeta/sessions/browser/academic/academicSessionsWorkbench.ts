import { addDisposableListener } from "../../../base/browser/dom.js";
import { DisposableOwner } from "../../../base/common/lifecycle.js";
import type { IBrowserViewApi } from "../../../platform/browser/common/browserView.js";
import type { SessionsProfile } from "../../common/sessionsProfile.js";
import { AcademicLibraryPane } from "./academicLibraryPane.js";
import { AcademicResearchWorkspace } from "./academicResearchWorkspace.js";
import { navigateToSessionsPage } from "../common/sessionNavigation.js";
import { SessionChatSurface } from "../common/sessionChatSurface.js";
import { SessionsList } from "../common/sessionsList.js";
import type { SessionsRuntime } from "../common/sessionsRuntime.js";

/** Fixed research workbench for literature, reading, browsing, and writing-agent sessions. */
export class AcademicSessionsWorkbench extends DisposableOwner {
  readonly element: HTMLElement;

  constructor(ownerDocument: Document, profile: SessionsProfile, runtime: SessionsRuntime, browserViewApi: IBrowserViewApi | undefined) {
    super();
    this.element = ownerDocument.createElement("main");
    this.element.className = "zeta-sessions-window zeta-academic-sessions-window";
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
    newSession.textContent = "New research session";
    header.append(returnToWorkbench, title, newSession);
    const body = ownerDocument.createElement("div");
    body.className = "zeta-academic-sessions-layout";
    const left = ownerDocument.createElement("aside");
    left.className = "zeta-academic-sessions-left";
    const sessionList = this.own(new SessionsList(ownerDocument, runtime.sessions, "Research sessions", "New research session"));
    const library = this.own(new AcademicLibraryPane(ownerDocument));
    left.append(sessionList.element, library.element);
    const research = this.own(new AcademicResearchWorkspace(ownerDocument, browserViewApi));
    const agent = ownerDocument.createElement("aside");
    agent.className = "zeta-academic-writing-agent";
    const agentHeader = ownerDocument.createElement("div");
    agentHeader.className = "zeta-sessions-surface-header";
    agentHeader.innerHTML = "<h1>Writing agent</h1><p>Ask for synthesis, structure, citations to verify, or a careful revision.</p>";
    const chat = this.own(new SessionChatSurface(ownerDocument, runtime.sessions, runtime.chat, "Ask the writing agent to synthesize sources or improve your draft…", "New research session"));
    agent.append(agentHeader, chat.element);
    body.append(left, research.element, agent);
    this.element.append(header, body);
    this.own(library.onDidSelectItem((item) => research.showSource(item)));
    this.own(research.onDidRequestWritingHelp((prompt) => void chat.sendPrompt(prompt)));
    this.own(addDisposableListener(returnToWorkbench, "click", () => navigateToSessionsPage(profile.workbenchRelativePath)));
    this.own(addDisposableListener(newSession, "click", () => {
      runtime.sessions.createUntitledSession("New research session");
      chat.focus();
    }));
    void runtime.initialize();
  }
}
