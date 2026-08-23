import type { AgentTreeNode } from "../../../../../sessions/services/sessions/common/session.js";

/** Presentation-only detail derived from one server-owned canonical Agent-tree node. */
export function agentNodeDetail(node: AgentTreeNode): string {
	const parts = [node.role?.name ?? agentNodeKind(node), node.executionStatus];
	if (node.waitingReason) parts.push(waitingReasonLabel(node.waitingReason));
	const reportedTokens = node.usage.inputTokens + node.usage.outputTokens;
	if (node.resourceBudget?.maxTotalTokens !== undefined) {
		parts.push(`${formatNumber(reportedTokens)}/${formatNumber(node.resourceBudget.maxTotalTokens)} tokens`);
	} else if (reportedTokens > 0) {
		parts.push(`${formatNumber(reportedTokens)} tokens`);
	}
	if (node.resourceBudget?.maxCostUsdMicros !== undefined) {
		parts.push(`$${(node.resourceBudget.maxCostUsdMicros / 1_000_000).toFixed(2)} cap`);
	}
	const waitingJoins = node.joins.filter(join => join.status === "waiting").length;
	if (waitingJoins > 0) parts.push(`${waitingJoins} join${waitingJoins === 1 ? "" : "s"} waiting`);
	if (node.result) parts.push(`result ${node.result.status}`);
	return parts.join(" · ");
}

export function agentNodeKind(node: AgentTreeNode): string {
	switch (node.origin.type) {
		case "root": return "Root";
		case "agentSpawn": return "Agent";
		case "fork": return "Fork";
		case "rewind": return "Rewind";
	}
}

export function canInterruptAgentNode(node: AgentTreeNode): boolean {
	return node.membershipStatus === "active"
		&& node.currentTurnId !== undefined
		&& (node.executionStatus === "queued" || node.executionStatus === "running" || node.executionStatus === "waiting");
}

function waitingReasonLabel(reason: NonNullable<AgentTreeNode["waitingReason"]>): string {
	switch (reason) {
		case "approval": return "waiting for approval";
		case "userInput": return "waiting for input";
		case "capability": return "waiting for capability";
	}
}

function formatNumber(value: number): string { return new Intl.NumberFormat("en-US").format(value); }
