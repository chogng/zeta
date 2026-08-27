import assert from "node:assert/strict";
import test from "node:test";
import type { AgentTreeNode } from "../../../../../sessions/services/sessions/common/session.js";
import { agentNodeDetail, canInterruptAgentNode } from "../../browser/view/chatAgentTree.js";

test("renders status, waiting reason, Goal budget, joins, and result from a canonical node", () => {
	const node = agentNode({
		executionStatus: "waiting",
		waitingReason: "approval",
		goal: { goalId: "goal-1", objective: "finish", status: "active", tokenBudget: 10_000, tokensUsed: 2_500 },
		usage: { inputTokens: 2_000, outputTokens: 500 },
		joins: [{ status: "waiting" }],
		result: { status: "completed", summary: "Done" },
	});

	assert.equal(agentNodeDetail(node), "Agent · waiting · waiting for approval · 2,500/10,000 goal tokens · goal active · 1 join waiting · result completed");
});

test("only exposes exact active non-terminal Turns as interruptible", () => {
	assert.equal(canInterruptAgentNode(agentNode({ executionStatus: "running", currentTurnId: "turn-1" })), true);
	assert.equal(canInterruptAgentNode(agentNode({ executionStatus: "completed", currentTurnId: "turn-1" })), false);
	assert.equal(canInterruptAgentNode(agentNode({ executionStatus: "running" })), false);
});

function agentNode(overrides: Partial<AgentTreeNode>): AgentTreeNode {
	return {
		threadId: "agent-1",
		threadSequence: 4,
		title: "Reviewer",
		origin: { type: "agentSpawn", parentThreadId: "root", parentSequence: 2, delegationId: "review" },
		membershipStatus: "active",
		executionStatus: "idle",
		usage: { inputTokens: 0, outputTokens: 0 },
		joins: [],
		children: [],
		...overrides,
	};
}
