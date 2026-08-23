import assert from "node:assert/strict";
import test from "node:test";
import type { Session } from "../../../../../sessions/services/sessions/common/session.js";
import { projectAgentTree } from "../../browser/view/chatAgentTree.js";

test("projects Agent spawn lineage as a stable nested tree", () => {
	const session: Session = {
		sessionId: "session-1",
		title: "Review",
		status: "active",
		sequence: 8,
		threads: [
			{ threadId: "root", origin: { type: "root" }, status: "active", executionStatus: "running" },
			{
				threadId: "reviewer",
				origin: { type: "agentSpawn", parentThreadId: "root", parentSequence: 4, delegationId: "review" },
				status: "active",
				executionStatus: "waiting",
			},
			{
				threadId: "nested",
				origin: { type: "agentSpawn", parentThreadId: "reviewer", parentSequence: 6, delegationId: "nested" },
				status: "active",
				executionStatus: "completed",
			},
		],
	};

	const tree = projectAgentTree(session);

	assert.equal(tree.length, 1);
	assert.equal(tree[0]?.thread.threadId, "root");
	assert.equal(tree[0]?.children[0]?.thread.threadId, "reviewer");
	assert.equal(tree[0]?.children[0]?.children[0]?.thread.threadId, "nested");
});

test("keeps an orphaned lineage node visible as a root instead of dropping it", () => {
	const session: Session = {
		sessionId: "session-1",
		title: "Recovered",
		status: "active",
		sequence: 2,
		threads: [{
			threadId: "orphan",
			origin: { type: "agentSpawn", parentThreadId: "missing", parentSequence: 1, delegationId: "orphan" },
			status: "active",
		}],
	};

	assert.equal(projectAgentTree(session)[0]?.thread.threadId, "orphan");
});
