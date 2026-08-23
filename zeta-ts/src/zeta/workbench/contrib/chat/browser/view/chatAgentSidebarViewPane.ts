import { ViewPane, type IViewPaneOptions } from "../../../../browser/parts/views/viewPane.js";
import type { AgentTreeNode, Session } from "../../../../../sessions/services/sessions/common/session.js";
import type { ISessionsManagementService } from "../../../../../sessions/services/sessions/common/sessionsManagementService.js";
import { agentNodeDetail, canInterruptAgentNode } from "./chatAgentTree.js";
import { h } from "../../../../../base/browser/dom.js";

/** Projects durable Session Thread lineage as the Workbench Agent tree. */
export class ChatAgentSidebarViewPane extends ViewPane {
	private readonly sessionService: ISessionsManagementService;

	constructor(container: HTMLElement, options: IViewPaneOptions, sessionService: ISessionsManagementService) {
		super(container, options);
		this.sessionService = sessionService;
		this.contentElement.classList.add("zeta-chat-agent-sidebar-view");
		this.own(sessionService.onDidChange(() => this.render()));
		this.render();
	}

	override focus(): void {
		this.contentElement.querySelector<HTMLButtonElement>("[role='treeitem']")?.focus();
	}

	private render(): void {
		const sessions = this.sessionService.sessions.filter((session) => session.status !== "archived");
		const active = this.sessionService.active;
		const content = this.contentElement;
		content.replaceChildren();
		if (sessions.length === 0) {
			const empty = h(content.ownerDocument, "p");
			empty.className = "zeta-chat-agent-sidebar-empty";
			empty.textContent = "No agent sessions yet.";
			content.append(empty);
			return;
		}
		for (const session of sessions) {
			content.append(this.createSessionTree(session, active?.session.sessionId, active?.threadId));
		}
	}

	private createSessionTree(session: Session, activeSessionId: string | undefined, activeThreadId: string | undefined): HTMLElement {
		const group = h(this.contentElement.ownerDocument, "section");
		group.className = "zeta-chat-agent-session";
		group.dataset.sessionId = session.sessionId;
		const heading = h(this.contentElement.ownerDocument, "h3");
		heading.className = "zeta-chat-agent-session-title";
		heading.textContent = session.title;
		const tree = h(this.contentElement.ownerDocument, "div");
		tree.className = "zeta-chat-agent-tree";
		tree.setAttribute("role", "tree");
		tree.setAttribute("aria-label", `${session.title} agents`);
		const nodes = session.agentTree ?? [];
		if (nodes.length === 0) {
			const empty = h(this.contentElement.ownerDocument, "span");
			empty.className = "zeta-chat-agent-session-detail";
			empty.textContent = session.status;
			tree.append(empty);
		} else {
			for (const node of nodes) this.appendNode(tree, session, node, 1, activeSessionId, activeThreadId);
		}
		group.append(heading, tree);
		return group;
	}

	private appendNode(container: HTMLElement, session: Session, node: AgentTreeNode, depth: number, activeSessionId: string | undefined, activeThreadId: string | undefined): void {
		const row = h(this.contentElement.ownerDocument, "div");
		row.className = "zeta-chat-agent-thread-row";
		row.setAttribute("role", "presentation");
		row.style.paddingInlineStart = `${(depth - 1) * 14}px`;
		const button = h(this.contentElement.ownerDocument, "button");
		button.className = `zeta-chat-agent-thread origin-${originClass(node)} status-${node.executionStatus}`;
		button.type = "button";
		button.dataset.threadId = node.threadId;
		button.setAttribute("role", "treeitem");
		button.setAttribute("aria-level", String(depth));
		if (node.children.length > 0) button.setAttribute("aria-expanded", "true");
		const selected = session.sessionId === activeSessionId && node.threadId === activeThreadId;
		button.classList.toggle("checked", selected);
		button.setAttribute("aria-selected", String(selected));
		const marker = h(this.contentElement.ownerDocument, "span");
		marker.className = "zeta-chat-agent-thread-marker";
		marker.setAttribute("aria-hidden", "true");
		const text = h(this.contentElement.ownerDocument, "span");
		text.className = "zeta-chat-agent-thread-text";
		const title = h(this.contentElement.ownerDocument, "span");
		title.className = "zeta-chat-agent-thread-title";
		title.textContent = node.title || defaultThreadTitle(session, node);
		const detail = h(this.contentElement.ownerDocument, "span");
		detail.className = "zeta-chat-agent-session-detail";
		detail.textContent = agentNodeDetail(node);
		if (node.result?.summary) detail.title = node.result.summary;
		text.append(title, detail);
		button.append(marker, text);
		button.disabled = node.membershipStatus !== "active" || session.status !== "active";
		button.addEventListener("click", () => this.sessionService.selectThread(session.sessionId, node.threadId));
		row.append(button);
		if (session.status === "active" && canInterruptAgentNode(node)) {
			const interrupt = h(this.contentElement.ownerDocument, "button");
			interrupt.className = "zeta-chat-agent-thread-stop";
			interrupt.type = "button";
			interrupt.textContent = "Stop";
			interrupt.setAttribute("aria-label", `Interrupt ${node.title || defaultThreadTitle(session, node)}`);
			interrupt.addEventListener("click", () => {
				void this.sessionService.interruptThread(session.sessionId, node.threadId).catch(error => {
					console.error(`Failed to interrupt Agent Thread '${node.threadId}'`, error);
				});
			});
			row.append(interrupt);
		}
		container.append(row);
		for (const child of node.children) this.appendNode(container, session, child, depth + 1, activeSessionId, activeThreadId);
	}
}

function defaultThreadTitle(session: Session, node: AgentTreeNode): string {
	if (node.origin.type === "root") return session.title;
	if (node.origin.type === "agentSpawn") return `Agent ${node.origin.delegationId}`;
	return "Agent branch";
}

function originClass(node: AgentTreeNode): string {
	switch (node.origin.type) {
		case "root": return "root";
		case "agentSpawn": return "agent-spawn";
		case "fork": return "fork";
		case "rewind": return "rewind";
	}
}
