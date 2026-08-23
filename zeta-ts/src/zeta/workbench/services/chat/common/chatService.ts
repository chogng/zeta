import type { Event } from "../../../../base/common/event.js";
import { createServiceIdentifier } from "../../../../platform/instantiation/common/instantiation.js";
import type { ModelRef, SessionId, ThreadId } from "../../../../sessions/services/sessions/common/session.js";
import type { SkillReference } from "../../../../platform/skills/common/skillApi.js";
import type { ModelCatalogEntry } from "./modelCatalog.js";

export type { ModelCatalogEntry } from "./modelCatalog.js";

export interface ChatImageAttachment {
	readonly contentDigest: string;
	readonly mediaType: "png" | "jpeg" | "gif" | "webP";
	readonly encodedBytes: number;
	readonly width: number;
	readonly height: number;
}

export interface SlashCommandDefinition {
	readonly name: string;
	readonly description: string;
	readonly argumentMode: "none" | "optional";
}

export interface SkillCommandDefinition {
	readonly name: string;
	readonly description: string;
	readonly source: string;
	readonly skill: SkillReference;
}

export type ThreadItem =
	| { readonly type: "userMessage"; readonly itemId: string; readonly turnId: string; readonly text: string }
	| { readonly type: "userImage"; readonly itemId: string; readonly turnId: string; readonly url: string }
	| { readonly type: "userImageAttachment"; readonly itemId: string; readonly turnId: string; readonly attachment: ChatImageAttachment }
	| { readonly type: "agentMessage"; readonly itemId: string; readonly turnId: string; readonly text: string }
	| { readonly type: "reasoning"; readonly itemId: string; readonly turnId: string; readonly text: string }
	| { readonly type: "plan"; readonly itemId: string; readonly turnId: string; readonly text: string }
	| { readonly type: "toolCall"; readonly itemId: string; readonly turnId: string; readonly toolCallId: string; readonly name: string; readonly argumentsJson: string }
	| { readonly type: "toolResult"; readonly itemId: string; readonly turnId: string; readonly toolCallId: string; readonly text: string; readonly isError: boolean };

export type TurnStatus = "created" | "running" | "waitingForApproval" | "waitingForUserInput" | "waitingForCapability" | "cancelling" | "completed" | "failed" | "interrupted";

export interface TurnError {
	readonly code: "modelInvocationFailed" | "contextOverflow" | "providerAuth" | "invalidRequest" | "invalidResponse" | "completionPersistenceFailed" | "interactionDeadlineElapsed" | "toolRepetition" | "turnBudgetExhausted";
	readonly message: string;
	readonly retryable: boolean;
}

export type PlanStepStatus = "pending" | "inProgress" | "completed";

export interface PlanStep {
	readonly step: string;
	readonly status: PlanStepStatus;
}

export interface PlanUpdate {
	readonly explanation?: string | null;
	readonly steps: readonly PlanStep[];
}

export interface Turn {
	readonly turnId: string;
	readonly status: TurnStatus;
	readonly model?: ModelRef | null;
	readonly resourceBudget?: TurnResourceBudget | null;
	readonly plan?: PlanUpdate | null;
	readonly usage: ModelUsageSummary;
	readonly items: readonly ThreadItem[];
	readonly error?: TurnError | null;
}

export interface ModelUsageTotal {
	readonly reported: number;
	readonly complete: boolean;
}

export interface ModelUsageSummary {
	readonly modelInvocations: number;
	readonly inputTokens: ModelUsageTotal;
	readonly outputTokens: ModelUsageTotal;
	readonly cachedInputTokens: ModelUsageTotal;
	readonly reasoningTokens: ModelUsageTotal;
}

export interface ModelPriceSnapshot {
	readonly model: ModelRef;
	readonly revision: string;
	readonly inputUsdMicrosPerMillionTokens: number;
	readonly cachedInputUsdMicrosPerMillionTokens: number;
	readonly outputUsdMicrosPerMillionTokens: number;
}

export interface TurnResourceBudget {
	readonly maxTotalTokens?: number | null;
	readonly maxCostUsdMicros?: number | null;
	readonly priceSnapshot?: ModelPriceSnapshot | null;
}

export interface Thread {
	readonly sessionId: SessionId;
	readonly threadId: ThreadId;
	readonly title: string;
	readonly status: "active" | "archived";
	readonly sequence: number;
	readonly usage: ModelUsageSummary;
	readonly turns: readonly Turn[];
}

export interface UserInputOption { readonly label: string; readonly description: string }
export interface UserInputQuestion { readonly id: string; readonly header: string; readonly question: string; readonly options?: readonly UserInputOption[]; readonly allowFreeForm: boolean }
export interface RequestUserInput { readonly questions: readonly UserInputQuestion[] }
export interface ActionApprovalRequest { readonly reason: string }
export interface DynamicToolCall { readonly callId: string; readonly name: string; readonly definitionDigest: string; readonly arguments: unknown }

export type AgentRequest =
	| { readonly type: "approval"; readonly request: ActionApprovalRequest }
	| { readonly type: "userInput"; readonly request: RequestUserInput }
	| { readonly type: "dynamicTool"; readonly call: DynamicToolCall };

export type AgentResponse =
	| { readonly type: "approval"; readonly response: { readonly decision: "approveOnce" | "decline" } }
	| { readonly type: "userInput"; readonly response: { readonly answers: Readonly<Record<string, { readonly value: string }>> } }
	| { readonly type: "dynamicTool"; readonly response: { readonly callId: string; readonly content: readonly ({ readonly type: "text"; readonly text: string } | { readonly type: "image"; readonly dataUrl: string })[]; readonly success: boolean } };

export interface TurnInteraction {
	readonly requestId: string;
	readonly itemId?: string | null;
	readonly request: AgentRequest;
	readonly deadline?: { readonly expiresAtUnixMs: number } | null;
}

export type ThreadCommittedEvent =
	| { readonly type: "interactionRequested"; readonly interaction: TurnInteraction }
	| { readonly type: "interactionResolved" }
	| { readonly type: "interactionCancelled" }
	| { readonly type: "turnCompleted" }
	| { readonly type: "turnFailed" }
	| { readonly type: "turnInterrupted" }
	| { readonly type: "threadCreated" | "turnAccepted" | "turnStarted" | "turnSteered" | "planUpdated" | "modelUsageRecorded" | "itemCompleted" | "toolExecutionStarted" | "toolExecutionEscalated" | "turnCancelling" };

export type ThreadUpdate =
	| { readonly type: "committed"; readonly event: ThreadCommittedEvent }
	| { readonly type: "itemStarted"; readonly item: ThreadItem }
	| { readonly type: "itemDelta"; readonly itemId: string; readonly delta: { readonly type: "agentMessage" | "reasoning" | "plan"; readonly text: string } }
	| { readonly type: "toolOutputDelta" };

export interface ThreadUpdateEnvelope {
	readonly sessionId: SessionId;
	readonly threadId: ThreadId;
	readonly durableSequence: number;
	readonly streamCursor?: { readonly streamInstanceId: string; readonly sequence: number } | null;
	readonly update: ThreadUpdate;
}

export interface ThreadSubscription {
	readonly thread: Thread;
	readonly updates: readonly ThreadUpdateEnvelope[];
}

export interface StartTurnOptions { readonly sessionId: SessionId; readonly threadId: ThreadId; readonly expectedSequence: number; readonly text: string; readonly skills?: readonly SkillReference[]; readonly resourceBudget?: TurnResourceBudget }
export interface CompactContextOptions { readonly sessionId: SessionId; readonly threadId: ThreadId; readonly expectedSequence: number; readonly retentionPrompt?: string }
export interface SteerTurnOptions { readonly sessionId: SessionId; readonly threadId: ThreadId; readonly turnId: string; readonly expectedSequence: number; readonly text: string }
export interface InterruptTurnOptions { readonly sessionId: SessionId; readonly threadId: ThreadId; readonly turnId: string; readonly expectedSequence: number }
export interface ResolveInteractionOptions extends InterruptTurnOptions { readonly requestId: string; readonly response: AgentResponse }

/** Frontend Chat operations, catalogs, and Thread update lifecycle. */
export interface IChatService {
	readonly onDidUpdateThread: Event<ThreadUpdateEnvelope>;
	readonly onDidBecomeReady: Event<void>;
	readonly onDidChangeModels: Event<void>;
	readonly onDidChangeSkills: Event<void>;
	listModels(): Promise<readonly ModelCatalogEntry[]>;
	listModelCatalog(): Promise<readonly ModelCatalogEntry[]>;
	refreshModels(): Promise<readonly ModelCatalogEntry[]>;
	isModelVisible(model: ModelRef): boolean;
	setModelVisible(model: ModelRef, visible: boolean): Promise<void>;
	listSlashCommands(): Promise<readonly SlashCommandDefinition[]>;
	listSkillCommands(): Promise<readonly SkillCommandDefinition[]>;
	readThread(sessionId: SessionId, threadId: ThreadId): Promise<Thread>;
	subscribeThread(sessionId: SessionId, threadId: ThreadId, afterSequence: number): Promise<ThreadSubscription>;
	unsubscribeThread(sessionId: SessionId, threadId: ThreadId): Promise<void>;
	startTurn(options: StartTurnOptions): Promise<void>;
	compactContext(options: CompactContextOptions): Promise<void>;
	steerTurn(options: SteerTurnOptions): Promise<void>;
	interruptTurn(options: InterruptTurnOptions): Promise<void>;
	resolveInteraction(options: ResolveInteractionOptions): Promise<void>;
}

export const IChatService = createServiceIdentifier<IChatService>("chatService");
