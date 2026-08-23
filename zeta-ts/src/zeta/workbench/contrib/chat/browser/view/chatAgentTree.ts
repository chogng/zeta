import type { Session, SessionThread, ThreadId } from "../../../../../sessions/services/sessions/common/session.js";

export interface AgentTreeNode {
	readonly thread: SessionThread;
	readonly children: readonly AgentTreeNode[];
}

/** Derives a stable, lossless tree from canonical Session Thread lineage. */
export function projectAgentTree(session: Session): readonly AgentTreeNode[] {
	const children = new Map<ThreadId, SessionThread[]>();
	const roots: SessionThread[] = [];
	const known = new Set(session.threads.map(thread => thread.threadId));
	for (const thread of session.threads) {
		const parentId = parentThreadId(thread);
		if (!parentId || !known.has(parentId) || parentId === thread.threadId) {
			roots.push(thread);
			continue;
		}
		const siblings = children.get(parentId) ?? [];
		siblings.push(thread);
		children.set(parentId, siblings);
	}
	const projected = new Set<ThreadId>();
	const build = (thread: SessionThread): AgentTreeNode => {
		projected.add(thread.threadId);
		return {
			thread,
			children: (children.get(thread.threadId) ?? [])
				.filter(child => !projected.has(child.threadId))
				.map(build),
		};
	};
	const tree = roots.map(build);
	for (const thread of session.threads) {
		if (!projected.has(thread.threadId)) tree.push(build(thread));
	}
	return tree;
}

function parentThreadId(thread: SessionThread): ThreadId | undefined {
	return thread.origin.type === "root" ? undefined : thread.origin.parentThreadId;
}
