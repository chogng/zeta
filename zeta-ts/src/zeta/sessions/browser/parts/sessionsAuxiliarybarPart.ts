import "./media/sessionsAuxiliarybarPart.css";
import type { IUntitledChatSession } from "../../services/sessions/common/session.js";
import type { ISessionsManagementService } from "../../services/sessions/common/sessionsManagementService.js";
import { WorkbenchPart } from "../../../workbench/browser/part.js";
import type { ISessionsViewService } from "../../services/view/common/sessionsViewService.js";
import { h } from "../../../base/browser/dom.js";

/** Typed Session and Thread context for the active Sessions Workbench slot. */
export class SessionsAuxiliarybarPart extends WorkbenchPart {
	private readonly sessionService: ISessionsManagementService;
	private readonly viewService: ISessionsViewService;

	override get minimumWidth(): number { return 220; }
	override get maximumWidth(): number { return 460; }

	constructor(container: HTMLElement, sessionService: ISessionsManagementService, viewService: ISessionsViewService) {
		super(container, "auxiliarybar");
		this.sessionService = sessionService;
		this.viewService = viewService;
		this.own(viewService.onDidChange(() => this.render()));
		this.render();
	}

	private render(): void {
		const content = this.contentElement;
		const heading = h(content.ownerDocument, "h2");
		heading.textContent = "Session context";
		const selection = this.viewService.activeSelection;
		if (selection?.kind === "session") {
			const active = selection.active;
			content.replaceChildren(heading, contextList(content.ownerDocument, [
				["Status", active.session.status],
				["Model", active.session.model ? `${active.session.model.provider}/${active.session.model.model}` : "Default"],
				["Threads", String(active.session.threads.length)],
				["Active thread", active.threadId],
				["Session", active.session.sessionId],
			]));
			return;
		}
		if (selection?.kind === "untitled") {
			content.replaceChildren(heading, contextList(content.ownerDocument, untitledContext(selection.session)));
			return;
		}
		const state = h(content.ownerDocument, "p");
		state.className = "zeta-sessions-context-empty";
		state.textContent = this.sessionService.state === "loading" ? "Loading sessions…" : this.sessionService.error ?? "No active session.";
		content.replaceChildren(heading, state);
	}
}

function untitledContext(session: IUntitledChatSession): ReadonlyArray<readonly [string, string]> {
	return [
		["Status", "Draft"],
		["Model", session.model ? `${session.model.provider}/${session.model.model}` : "Default"],
		["Session", "Created on first send"],
	];
}

function contextList(ownerDocument: Document, rows: ReadonlyArray<readonly [string, string]>): HTMLDListElement {
	const list = h(ownerDocument, "dl");
	list.className = "zeta-sessions-context-list";
	for (const [label, value] of rows) {
		const term = h(ownerDocument, "dt");
		term.textContent = label;
		const detail = h(ownerDocument, "dd");
		detail.textContent = value;
		detail.title = value;
		list.append(term, detail);
	}
	return list;
}
