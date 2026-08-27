import type { AgentResponse as AgentResponseDto, InputItem, SkillRef as SkillRefDto, Thread as ThreadDto, ThreadUpdateEnvelope as ThreadUpdateEnvelopeDto } from "../../../../../../generated/app-server/types.js";
import { Emitter } from "../../../../base/common/event.js";
import { DisposableOwner } from "../../../../base/common/lifecycle.js";
import { createUuid } from "../../../../base/common/uuid.js";
import type { IConfigurationService } from "../../../../platform/configuration/common/configurationService.js";
import type { IAppServerApi, IServerEventApi } from "../../../../platform/app-server/common/appServerApi.js";
import type { IModelApi, IThreadApi, ITurnApi } from "../../../../platform/sessions/common/sessionApi.js";
import type { ISkillApi } from "../../../../platform/skills/common/skillApi.js";
import type { ModelRef, SessionId, ThreadId } from "../../../../sessions/services/sessions/common/session.js";
import type { CompactContextOptions, IChatService, InterruptTurnOptions, ModelCatalogEntry, ResolveInteractionOptions, SkillCommandDefinition, SlashCommandDefinition, StartTurnOptions, SteerTurnOptions, Thread, ThreadGoalUpdate, ThreadSubscription, ThreadUpdateEnvelope } from "../common/chatService.js";
import { ModelCatalogConfiguration, modelRefIdentity } from "../common/modelCatalog.js";

export interface ChatServiceOptions {
	readonly modelApi: IModelApi;
	readonly threadApi: IThreadApi;
	readonly turnApi: ITurnApi;
	readonly skillApi: ISkillApi;
	readonly appServerApi: IAppServerApi;
	readonly eventApi: IServerEventApi;
	readonly configurationService?: IConfigurationService;
}

/** App Server-backed implementation of the frontend Chat service. */
export class ChatService extends DisposableOwner implements IChatService {
	private readonly _onDidUpdateThread = this.own(new Emitter<ThreadUpdateEnvelope>());
	private readonly _onDidUpdateGoal = this.own(new Emitter<ThreadGoalUpdate>());
	private readonly _onDidBecomeReady = this.own(new Emitter<void>());
	private readonly _onDidChangeModels = this.own(new Emitter<void>());
	private readonly _onDidChangeSkills = this.own(new Emitter<void>());
	private readonly hiddenModels = new Map<string, ModelRef>();
	private modelCatalog: readonly ModelCatalogEntry[] = [];
	private modelCatalogLoad: Promise<readonly ModelCatalogEntry[]> | undefined;
	private hasLoadedModelCatalog = false;

	readonly onDidUpdateThread = this._onDidUpdateThread.event;
	readonly onDidUpdateGoal = this._onDidUpdateGoal.event;
	readonly onDidBecomeReady = this._onDidBecomeReady.event;
	readonly onDidChangeModels = this._onDidChangeModels.event;
	readonly onDidChangeSkills = this._onDidChangeSkills.event;

	constructor(private readonly options: ChatServiceOptions) {
		super();
		const events = options.eventApi.subscribe((event) => {
			if (event.method === "session/thread/update") this._onDidUpdateThread.fire(toThreadUpdate(event.params));
			if (event.method === "thread/goal/updated") this._onDidUpdateGoal.fire({ threadId: event.params.threadId, goal: { ...event.params.goal } });
			if (event.method === "thread/goal/cleared") this._onDidUpdateGoal.fire({ threadId: event.params.threadId });
			if (event.method === "skills/changed") this._onDidChangeSkills.fire();
		});
		this.defer(() => events.dispose());
		const connection = options.appServerApi.onConnectionState((state) => {
			if (state !== "ready") return;
			const refresh = this.refreshModels();
			this._onDidBecomeReady.fire();
			void refresh.catch(() => undefined);
		});
		this.defer(() => connection.dispose());
		this.acceptHiddenModels(options.configurationService?.getValue(ModelCatalogConfiguration.hiddenModels) ?? []);
		if (options.configurationService) {
			this.own(options.configurationService.onDidChangeConfiguration((event) => {
				if (!event.affectsConfiguration(ModelCatalogConfiguration.hiddenModels)) return;
				this.acceptHiddenModels(options.configurationService!.getValue(ModelCatalogConfiguration.hiddenModels));
			}));
		}
	}

	async listModels(): Promise<readonly ModelCatalogEntry[]> {
		const catalog = await this.listModelCatalog();
		return catalog.filter(entry => this.isModelVisible(entry.model));
	}

	async listModelCatalog(): Promise<readonly ModelCatalogEntry[]> {
		if (this.hasLoadedModelCatalog) return this.modelCatalog;
		return this.refreshModels();
	}

	async refreshModels(): Promise<readonly ModelCatalogEntry[]> {
		if (this.modelCatalogLoad) return this.modelCatalogLoad;
		const load = this.options.modelApi.list().then(result => this.acceptModelCatalog(result.models));
		this.modelCatalogLoad = load;
		try {
			return await load;
		} finally {
			if (this.modelCatalogLoad === load) this.modelCatalogLoad = undefined;
		}
	}

	isModelVisible(model: ModelRef): boolean {
		return !this.hiddenModels.has(modelRefIdentity(model));
	}

	async setModelVisible(model: ModelRef, visible: boolean): Promise<void> {
		const identity = modelRefIdentity(model);
		if (visible === !this.hiddenModels.has(identity)) return;
		const models = [...this.hiddenModels.values()].filter(candidate => modelRefIdentity(candidate) !== identity);
		if (!visible) models.push({ ...model });
		if (this.options.configurationService) {
			await this.options.configurationService.updateValue(ModelCatalogConfiguration.hiddenModels, models);
			return;
		}
		this.acceptHiddenModels(models);
	}

	async listSlashCommands(): Promise<readonly SlashCommandDefinition[]> {
		const commands = await this.options.appServerApi.getSlashCommands();
		return commands.map((command) => ({ ...command }));
	}

	async listSkillCommands(): Promise<readonly SkillCommandDefinition[]> {
		const catalog = await this.options.skillApi.list("cached");
		const counts = new Map<string, number>();
		for (const skill of catalog.skills.filter(skill => skill.enabled && skill.compatible)) counts.set(skill.id.name, (counts.get(skill.id.name) ?? 0) + 1);
		return catalog.skills
			.filter(skill => skill.enabled && skill.compatible && counts.get(skill.id.name) === 1)
			.map(skill => ({
				name: skill.id.name,
				description: skill.description,
				source: skill.id.source,
				skill: { id: { ...skill.id }, version: { type: "pinnedDigest", digest: skill.contentDigest } },
			}));
	}

	async readThread(sessionId: SessionId, threadId: ThreadId): Promise<Thread> {
		return toThread((await this.options.threadApi.read({ sessionId, threadId })).thread);
	}

	async subscribeThread(sessionId: SessionId, threadId: ThreadId, afterSequence: number): Promise<ThreadSubscription> {
		const result = await this.options.threadApi.subscribe({ sessionId, threadId, afterSequence });
		return { thread: toThread(result.thread), updates: result.updates.map(toThreadUpdate) };
	}

	unsubscribeThread(sessionId: SessionId, threadId: ThreadId): Promise<void> {
		return this.options.threadApi.unsubscribe({ sessionId, threadId });
	}

	async startTurn(options: StartTurnOptions): Promise<void> {
		const input: InputItem[] = [
			...(options.skills ?? []).map(skill => ({ type: "skill" as const, skill: skill as SkillRefDto })),
			...(options.contexts ?? []).map(context => ({ type: "context" as const, name: context.name, content: context.content })),
			{ type: "text", text: options.text },
		];
		await this.options.turnApi.start({ commandId: commandId("turn"), sessionId: options.sessionId, threadId: options.threadId, expectedSequence: options.expectedSequence, approvalMode: "askPermissions", input });
	}

	async compactContext(options: CompactContextOptions): Promise<void> {
		await this.options.turnApi.compact({
			commandId: commandId("compact"),
			sessionId: options.sessionId,
			threadId: options.threadId,
			expectedSequence: options.expectedSequence,
			retentionPrompt: options.retentionPrompt,
		});
	}

	async steerTurn(options: SteerTurnOptions): Promise<void> {
		await this.options.turnApi.steer({
			commandId: commandId("steer"),
			sessionId: options.sessionId,
			threadId: options.threadId,
			turnId: options.turnId,
			expectedSequence: options.expectedSequence,
			input: [
				...(options.contexts ?? []).map(context => ({ type: "context" as const, name: context.name, content: context.content })),
				{ type: "text", text: options.text },
			],
		});
	}

	async interruptTurn(options: InterruptTurnOptions): Promise<void> {
		await this.options.turnApi.interrupt({ commandId: commandId("interrupt"), ...options });
	}

	async resolveInteraction(options: ResolveInteractionOptions): Promise<void> {
		await this.options.turnApi.resolveInteraction({ commandId: commandId("interaction"), ...options, response: toAgentResponse(options.response) });
	}

	private acceptModelCatalog(entries: readonly {
		readonly model: ModelRef;
		readonly displayName: string;
		readonly access: ModelCatalogEntry["access"];
		readonly outputTransport: ModelCatalogEntry["outputTransport"];
	}[]): readonly ModelCatalogEntry[] {
		const identities = new Set<string>();
		const catalog = entries.map(entry => {
			const identity = modelRefIdentity(entry.model);
			if (identities.has(identity)) throw new Error(`Model catalog contains duplicate entry '${entry.model.provider}/${entry.model.model}'`);
			identities.add(identity);
			return Object.freeze({
				model: Object.freeze({ ...entry.model }),
				displayName: entry.displayName,
				access: entry.access,
				outputTransport: entry.outputTransport,
			});
		});
		const changed = !sameModelCatalog(this.modelCatalog, catalog);
		this.modelCatalog = Object.freeze(catalog);
		this.hasLoadedModelCatalog = true;
		if (changed) this._onDidChangeModels.fire();
		return this.modelCatalog;
	}

	private acceptHiddenModels(models: readonly ModelRef[]): void {
		const next = new Map(models.map(model => [modelRefIdentity(model), Object.freeze({ ...model })]));
		if (sameKeys(this.hiddenModels, next)) return;
		this.hiddenModels.clear();
		for (const [identity, model] of next) this.hiddenModels.set(identity, model);
		this._onDidChangeModels.fire();
	}
}

function sameModelCatalog(left: readonly ModelCatalogEntry[], right: readonly ModelCatalogEntry[]): boolean {
	return left.length === right.length && left.every((entry, index) => {
		const candidate = right[index];
		return candidate !== undefined
			&& entry.displayName === candidate.displayName
			&& entry.access === candidate.access
			&& entry.outputTransport === candidate.outputTransport
			&& modelRefIdentity(entry.model) === modelRefIdentity(candidate.model);
	});
}

function sameKeys(left: ReadonlyMap<string, unknown>, right: ReadonlyMap<string, unknown>): boolean {
	return left.size === right.size && [...left.keys()].every(key => right.has(key));
}

function toThread(thread: ThreadDto): Thread {
	return {
		sessionId: thread.sessionId,
		threadId: thread.threadId,
		title: thread.title,
		status: thread.status,
		sequence: thread.sequence,
		goal: thread.goal ? { ...thread.goal } : thread.goal,
		usage: {
			modelInvocations: thread.usage.modelInvocations,
			inputTokens: { ...thread.usage.inputTokens },
			outputTokens: { ...thread.usage.outputTokens },
			cachedInputTokens: { ...thread.usage.cachedInputTokens },
			reasoningTokens: { ...thread.usage.reasoningTokens },
		},
		turns: thread.turns.map((turn) => ({
			turnId: turn.turnId,
			status: turn.status,
			model: turn.model ? { ...turn.model } : turn.model,
			plan: turn.plan ? {
				explanation: turn.plan.explanation,
				steps: turn.plan.steps.map((step) => ({ ...step })),
			} : turn.plan,
			usage: {
				modelInvocations: turn.usage.modelInvocations,
				inputTokens: { ...turn.usage.inputTokens },
				outputTokens: { ...turn.usage.outputTokens },
				cachedInputTokens: { ...turn.usage.cachedInputTokens },
				reasoningTokens: { ...turn.usage.reasoningTokens },
			},
			items: turn.items.map((item) => ({ ...item })),
			error: turn.error ? { ...turn.error } : turn.error,
		})),
	};
}

function toThreadUpdate(update: ThreadUpdateEnvelopeDto): ThreadUpdateEnvelope {
	return update as unknown as ThreadUpdateEnvelope;
}

function toAgentResponse(response: ResolveInteractionOptions["response"]): AgentResponseDto {
	switch (response.type) {
		case "approval": return { type: "approval", response: { ...response.response } };
		case "userInput": return { type: "userInput", response: { answers: { ...response.response.answers } } };
		case "dynamicTool": return { type: "dynamicTool", response: { ...response.response, content: response.response.content.map((output) => ({ ...output })) } };
	}
}

function commandId(kind: string): string { return `desktop-${kind}-${createUuid()}`; }
