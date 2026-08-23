import { ViewPane, type IViewPaneOptions } from "../../../../browser/parts/views/viewPane.js";
import type { Session, SessionThread } from "../../../../../sessions/services/sessions/common/session.js";
import type { ISessionsManagementService } from "../../../../../sessions/services/sessions/common/sessionsManagementService.js";
import { type AgentTreeNode, projectAgentTree } from "./chatAgentTree.js";
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
		const nodes = projectAgentTree(session);
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
		const button = h(this.contentElement.ownerDocument, "button");
		button.className = "zeta-chat-agent-thread";
		button.type = "button";
		button.dataset.threadId = node.thread.threadId;
		button.dataset.origin = node.thread.origin.type;
		button.dataset.executionStatus = node.thread.executionStatus ?? "idle";
		button.style.paddingInlineStart = `${8 + (depth - 1) * 14}px`;
		button.setAttribute("role", "treeitem");
		button.setAttribute("aria-level", String(depth));
		if (node.children.length > 0) button.setAttribute("aria-expanded", "true");
		const selected = session.sessionId === activeSessionId && node.thread.threadId === activeThreadId;
		button.classList.toggle("checked", selected);
		button.setAttribute("aria-selected", String(selected));
		const marker = h(this.contentElement.ownerDocument, "span");
		marker.className = "zeta-chat-agent-thread-marker";
		marker.setAttribute("aria-hidden", "true");
		const text = h(this.contentElement.ownerDocument, "span");
		text.className = "zeta-chat-agent-thread-text";
		const title = h(this.contentElement.ownerDocument, "span");
		title.className = "zeta-chat-agent-thread-title";
		title.textContent = node.thread.title ?? defaultThreadTitle(session, node.thread);
		const detail = h(this.contentElement.ownerDocument, "span");
		detail.className = "zeta-chat-agent-session-detail";
		detail.textContent = threadDetail(node.thread);
		text.append(title, detail);
		button.append(marker, text);
		button.disabled = node.thread.status !== "active" || session.status !== "active";
		button.addEventListener("click", () => this.sessionService.selectThread(session.sessionId, node.thread.threadId));
		container.append(button);
		for (const child of node.children) this.appendNode(container, session, child, depth + 1, activeSessionId, activeThreadId);
	}
}

function defaultThreadTitle(session: Session, thread: SessionThread): string {
	if (thread.origin.type === "root") return session.title;
	if (thread.origin.type === "agentSpawn") return `Agent ${thread.origin.delegationId}`;
	return "Agent branch";
}

function threadDetail(thread: SessionThread): string {
	const kind = thread.origin.type === "root"
		? "Root"
		: thread.origin.type === "agentSpawn"
			? "Agent"
			: thread.origin.type === "fork"
				? "Fork"
				: "Rewind";
	return `${kind} · ${thread.executionStatus ?? thread.status}`;
}
