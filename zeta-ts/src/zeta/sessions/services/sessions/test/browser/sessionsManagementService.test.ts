import assert from "node:assert/strict";
import test from "node:test";
import type { AgentTreeNodeProjection, ServerNotification, Session as SessionDto } from "../../../../../../../generated/app-server/types.js";
import type { IServerEventApi } from "../../../../../platform/app-server/common/appServerApi.js";
import type { ISessionApi, ITurnApi } from "../../../../../platform/sessions/common/sessionApi.js";
import { AppServerSessionsManagementService } from "../../browser/appServerSessionsManagementService.js";

test("AppServerSessionsManagementService refreshes subscribed Sessions from canonical update snapshots", async () => {
	let current = session(1);
	const listeners = new Set<(event: ServerNotification) => void>();
	const subscribed: string[] = [];
	const unsubscribed: string[] = [];
	const api: ISessionApi = {
		async create() { return { session: current }; },
		async read() { return { session: current }; },
		async list() { return { sessions: [current] }; },
		async subscribe(params) {
			subscribed.push(params.sessionId);
			return { session: current, updates: [], threadProjections: [], agentTree: { roots: [] } };
		},
		async unsubscribe(params) { unsubscribed.push(params.sessionId); },
		async createThread() { throw new Error("Not used"); },
		async forkThread() { throw new Error("Not used"); },
		async archiveThread() { throw new Error("Not used"); },
		async complete() { throw new Error("Not used"); },
		async archive() { throw new Error("Not used"); },
		async stop() { throw new Error("Not used"); },
		async setModel() { throw new Error("Not used"); },
		async setNextApprovalMode() { throw new Error("Not used"); },
	};
	const events: IServerEventApi = {
		subscribe(listener) {
			listeners.add(listener);
			return { dispose: () => { listeners.delete(listener); } };
		},
	};
	using service = new AppServerSessionsManagementService({ session: api, events });
	await service.initialize();

	assert.deepEqual(subscribed, ["session-1"]);
	assert.equal(service.active?.session.sequence, 1);

	current = { ...current, sequence: 2, model: { provider: "openai", model: "gpt-live" } };
	emit(listeners, { method: "session/update", params: { sessionId: "session-1", durableSequence: 2, update: { type: "committed", event: { type: "sessionModelChanged", sessionId: "session-1", model: current.model! } } } });
	await waitFor(() => service.sessions[0]?.sequence === 2);

	assert.deepEqual(service.sessions[0]?.model, { provider: "openai", model: "gpt-live" });
	assert.deepEqual(service.active?.session.model, { provider: "openai", model: "gpt-live" });

	current = { ...current, sequence: 3, status: "archived" };
	emit(listeners, { method: "session/update", params: { sessionId: "session-1", durableSequence: 3, update: { type: "committed", event: { type: "sessionArchived", sessionId: "session-1" } } } });
	await waitFor(() => service.sessions[0]?.status === "archived");

	assert.deepEqual(unsubscribed, ["session-1"]);
	assert.equal(service.active, undefined);
});

test("AppServerSessionsManagementService stops refreshing when the canonical snapshot cannot reach an announced sequence", async () => {
	const current = session(1);
	const listeners = new Set<(event: ServerNotification) => void>();
	let subscriptions = 0;
	const api: ISessionApi = {
		async create() { return { session: current }; },
		async read() { return { session: current }; },
		async list() { return { sessions: [current] }; },
		async subscribe() { subscriptions += 1; return { session: current, updates: [], threadProjections: [], agentTree: { roots: [] } }; },
		async unsubscribe() {},
		async createThread() { throw new Error("Not used"); },
		async forkThread() { throw new Error("Not used"); },
		async archiveThread() { throw new Error("Not used"); },
		async complete() { throw new Error("Not used"); },
		async archive() { throw new Error("Not used"); },
		async stop() { throw new Error("Not used"); },
		async setModel() { throw new Error("Not used"); },
		async setNextApprovalMode() { throw new Error("Not used"); },
	};
	const events: IServerEventApi = {
		subscribe(listener) {
			listeners.add(listener);
			return { dispose: () => { listeners.delete(listener); } };
		},
	};
	using service = new AppServerSessionsManagementService({ session: api, events });
	await service.initialize();

	emit(listeners, { method: "session/update", params: { sessionId: "session-1", durableSequence: 2, update: { type: "committed", event: { type: "sessionModelChanged", sessionId: "session-1", model: { provider: "openai", model: "gpt-live" } } } } });
	await waitFor(() => service.state === "error");

	assert.equal(subscriptions, 2);
	assert.match(service.error ?? "", /did not advance/);
});

test("AppServerSessionsManagementService refreshes the canonical Agent tree after Thread updates", async () => {
	const current = session(1);
	const listeners = new Set<(event: ServerNotification) => void>();
	let tree: AgentTreeNodeProjection = agentTreeNode();
	let subscriptions = 0;
	const api: ISessionApi = {
		async create() { return { session: current }; },
		async read() { return { session: current }; },
		async list() { return { sessions: [current] }; },
		async subscribe() {
			subscriptions += 1;
			return { session: current, updates: [], threadProjections: [], agentTree: { roots: [tree] } };
		},
		async unsubscribe() {},
		async createThread() { throw new Error("Not used"); },
		async forkThread() { throw new Error("Not used"); },
		async archiveThread() { throw new Error("Not used"); },
		async complete() { throw new Error("Not used"); },
		async archive() { throw new Error("Not used"); },
		async stop() { throw new Error("Not used"); },
		async setModel() { throw new Error("Not used"); },
		async setNextApprovalMode() { throw new Error("Not used"); },
	};
	const events: IServerEventApi = {
		subscribe(listener) {
			listeners.add(listener);
			return { dispose: () => { listeners.delete(listener); } };
		},
	};
	using service = new AppServerSessionsManagementService({ session: api, events });
	await service.initialize();

	tree = { ...tree, threadSequence: 12, executionStatus: "completed" };
	emit(listeners, {
		method: "session/thread/update",
		params: {
			sessionId: "session-1",
			threadId: "thread-1",
			durableSequence: 12,
			update: { type: "committed", event: { type: "turnCompleted", threadId: "thread-1", turnId: "turn-7" } },
		},
	});
	await waitFor(() => service.sessions[0]?.agentTree?.[0]?.threadSequence === 12);

	assert.equal(service.sessions[0]?.agentTree?.[0]?.executionStatus, "completed");
	assert.equal(service.sessions[0]?.sequence, 1);
	assert.equal(subscriptions, 2);

	// A replayed or delayed Thread notification must not trigger another projection fetch.
	emit(listeners, {
		method: "session/thread/update",
		params: {
			sessionId: "session-1",
			threadId: "thread-1",
			durableSequence: 12,
			update: { type: "committed", event: { type: "turnCompleted", threadId: "thread-1", turnId: "turn-7" } },
		},
	});
	await new Promise(resolve => setTimeout(resolve, 0));
	assert.equal(subscriptions, 2);
});

test("AppServerSessionsManagementService reopens a foreign Session Workspace before selecting its Thread", async () => {
	const current = { ...session(1), workspace: { authorityId: "current", root: "/workspaces/current" } };
	const foreign: SessionDto = {
		...session(1),
		sessionId: "session-foreign",
		title: "Foreign",
		workspace: { authorityId: "foreign", root: "/workspaces/foreign" },
		threads: [{ threadId: "thread-foreign", origin: { type: "root" }, status: "active" }],
	};
	let currentWorkspaceRoot = current.workspace.root;
	const reopened: string[] = [];
	const api: ISessionApi = {
		async create() { return { session: current }; },
		async read(params) { return { session: params.sessionId === foreign.sessionId ? foreign : current }; },
		async list() { return { sessions: [foreign, current] }; },
		async subscribe(params) { const value = params.sessionId === foreign.sessionId ? foreign : current; return { session: value, updates: [], threadProjections: [], agentTree: { roots: [] } }; },
		async unsubscribe() {},
		async createThread() { throw new Error("Not used"); },
		async forkThread() { throw new Error("Not used"); },
		async archiveThread() { throw new Error("Not used"); },
		async complete() { throw new Error("Not used"); },
		async archive() { throw new Error("Not used"); },
		async stop() { throw new Error("Not used"); },
		async setModel() { throw new Error("Not used"); },
		async setNextApprovalMode() { throw new Error("Not used"); },
	};
	using service = new AppServerSessionsManagementService({
		session: api,
		workspaceRouter: {
			currentWorkspaceRoot: () => currentWorkspaceRoot,
			async reopenWorkspace(root) {
				reopened.push(root);
				currentWorkspaceRoot = root;
			},
		},
	});
	await service.initialize();

	assert.equal(service.active?.session.sessionId, current.sessionId);
	service.selectThread(foreign.sessionId, "thread-foreign");
	await waitFor(() => service.active?.session.sessionId === foreign.sessionId);

	assert.deepEqual(reopened, ["/workspaces/foreign"]);
	assert.equal(service.active?.threadId, "thread-foreign");
});

test("AppServerSessionsManagementService interrupts the exact canonical Agent Turn and sequence", async () => {
	const current = session(3);
	const interrupted: Parameters<ITurnApi["interrupt"]>[0][] = [];
	const api: ISessionApi = {
		async create() { return { session: current }; },
		async read() { return { session: current }; },
		async list() { return { sessions: [current] }; },
		async subscribe() {
			return {
				session: current,
				updates: [],
				threadProjections: [],
				agentTree: { roots: [agentTreeNode()] },
			};
		},
		async unsubscribe() {},
		async createThread() { throw new Error("Not used"); },
		async forkThread() { throw new Error("Not used"); },
		async archiveThread() { throw new Error("Not used"); },
		async complete() { throw new Error("Not used"); },
		async archive() { throw new Error("Not used"); },
		async stop() { throw new Error("Not used"); },
		async setModel() { throw new Error("Not used"); },
		async setNextApprovalMode() { throw new Error("Not used"); },
	};
	const turn: ITurnApi = {
		async start() { throw new Error("Not used"); },
		async compact() { throw new Error("Not used"); },
		async steer() { throw new Error("Not used"); },
		async interrupt(params) { interrupted.push(params); return { sequence: 12 }; },
		async resolveInteraction() { throw new Error("Not used"); },
	};
	using service = new AppServerSessionsManagementService({ session: api, turn });
	await service.initialize();

	await service.interruptThread("session-1", "thread-1");

	assert.equal(interrupted.length, 1);
	assert.deepEqual(interrupted[0] && {
		sessionId: interrupted[0].sessionId,
		threadId: interrupted[0].threadId,
		turnId: interrupted[0].turnId,
		expectedSequence: interrupted[0].expectedSequence,
	}, { sessionId: "session-1", threadId: "thread-1", turnId: "turn-7", expectedSequence: 11 });
});

function session(sequence: number): SessionDto {
	return {
		sessionId: "session-1",
		title: "Session 1",
		status: "active",
		nextApprovalMode: "askPermissions",
		sequence,
		threads: [{ threadId: "thread-1", origin: { type: "root" }, status: "active" }],
	};
}

function agentTreeNode() {
	const usageTotal = { reported: 0, complete: true };
	return {
		threadId: "thread-1",
		threadSequence: 11,
		title: "Reviewer",
		origin: { type: "root" as const },
		membershipStatus: "active" as const,
		executionStatus: "running" as const,
		currentTurnId: "turn-7",
		usage: {
			modelInvocations: 0,
			inputTokens: usageTotal,
			outputTokens: usageTotal,
			cachedInputTokens: usageTotal,
			reasoningTokens: usageTotal,
		},
		joins: [],
		children: [],
	};
}

function emit(listeners: ReadonlySet<(event: ServerNotification) => void>, event: ServerNotification): void {
	for (const listener of listeners) listener(event);
}

async function waitFor(predicate: () => boolean): Promise<void> {
	for (let attempt = 0; attempt < 30; attempt += 1) {
		if (predicate()) return;
		await new Promise(resolve => setTimeout(resolve, 0));
	}
	assert.fail("Timed out waiting for Session refresh");
}
