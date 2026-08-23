import { Emitter, type Event } from "../../../../../base/common/event.js";
import { DisposableOwner } from "../../../../../base/common/lifecycle.js";
import type { AgentResponse, IChatService, ModelCatalogEntry, SkillCommandDefinition, SlashCommandDefinition, Thread, ThreadItem, ThreadUpdateEnvelope, Turn, TurnInteraction } from "../../../../services/chat/common/chatService.js";
import type { SkillReference } from "../../../../../platform/skills/common/skillApi.js";
import type { IActiveSessionThread, IUntitledChatSession, ModelRef, SessionId, ThreadId } from "../../../../../sessions/services/sessions/common/session.js";
import type { ISessionsManagementService } from "../../../../../sessions/services/sessions/common/sessionsManagementService.js";
import { chatListItem, chatPlanListItem, chatTurnErrorListItem, type IChatListItem } from "../list/chatListItems.js";

export type ChatPaneState =
	| "loading"
	| "ready"
	| "submitting"
	| "error";

/** The local or durable identity currently projected by a Chat pane. */
export type ChatPaneSelection =
	| { readonly kind: "session"; readonly active: IActiveSessionThread }
	| { readonly kind: "untitled"; readonly session: IUntitledChatSession };

/**
 * Projection of one Chat tab, before or after it acquires a durable Thread.
 *
 * Canonical committed state is refreshed from `session/thread/read`. Transient item
 * updates are layered by Item ID and discarded once the committed snapshot
 * contains the same item.
 */
export class ChatPaneModel extends DisposableOwner {
	private readonly chatService: IChatService;
	private readonly sessionService: ISessionsManagementService;
	private readonly _onDidChange = this.own(new Emitter<void>());
	private readonly transientItems = new Map<string, ThreadItem>();
	private selection: ChatPaneSelection;
	private _thread: Thread | undefined;
	private _interaction: TurnInteraction | undefined;
	private _state: ChatPaneState = "loading";
	private _error: string | undefined;
	private generation = 0;
	private disposed = false;
	private readPending = false;
	private initializePromise: Promise<void> | undefined;
	private subscriptionThreadId: ThreadId | undefined;
	private subscriptionPromise: Promise<void> | undefined;
	private streamInstanceId: string | undefined;
	private streamSequence = 0;
	private readonly retiredStreamInstanceIds = new Set<string>();
	private _models: readonly ModelCatalogEntry[] = [];
	private _slashCommands: readonly SlashCommandDefinition[] = [];
	private _skillCommands: readonly SkillCommandDefinition[] = [];

	readonly onDidChange: Event<void> = this._onDidChange.event;

	constructor(chatService: IChatService, selection: ChatPaneSelection, sessionService: ISessionsManagementService) {
		super();
		this.chatService = chatService;
		this.sessionService = sessionService;
		this.selection = selection;
		this.own(chatService.onDidUpdateThread((update) => this.acceptUpdate(update)));
		this.own(chatService.onDidBecomeReady(() => void this.reconnect()));
		this.own(chatService.onDidChangeModels(() => void this.loadModels()));
		this.own(chatService.onDidChangeSkills(() => void this.loadSkillCommands()));
		this.defer(() => {
			this.disposed = true;
			this.generation++;
			const active = this.activeSession;
			if (active) void this.chatService.unsubscribeThread(active.session.sessionId, active.threadId);
			this.transientItems.clear();
			this.resetStreamCursor();
		});
		void this.initialize();
	}

	get state(): ChatPaneState {
		return this._state;
	}

	get error(): string | undefined {
		return this._error;
	}

	get thread(): Thread | undefined {
		return this._thread;
	}

	get sessionId(): SessionId | undefined {
		return this.activeSession?.session.sessionId;
	}

	get untitledSessionId(): string | undefined {
		return this.selection.kind === "untitled" ? this.selection.session.untitledSessionId : undefined;
	}

	get threadId(): ThreadId | undefined {
		return this.activeSession?.threadId;
	}

	get models(): readonly ModelCatalogEntry[] {
		return this._models;
	}

	get slashCommands(): readonly SlashCommandDefinition[] {
		return this._slashCommands;
	}

	get skillCommands(): readonly SkillCommandDefinition[] {
		return this._skillCommands;
	}

	get selectedModel(): ModelRef | undefined {
		return this.selection.kind === "untitled"
			? this.selection.session.model
			: this.selection.active.session.model ?? undefined;
	}

	get items(): readonly IChatListItem[] {
		const turns = this._thread?.turns ?? [];
		const latestTurnId = turns.at(-1)?.turnId;
		const committed = turns.flatMap((turn) => {
			const items = turn.items.map((item) => chatListItem(item));
			const plan = chatPlanListItem(turn);
			const failure = chatTurnErrorListItem(turn, { actionsEnabled: turn.turnId === latestTurnId });
			return [...items, ...(plan ? [plan] : []), ...(failure ? [failure] : [])];
		});
		const committedIds = new Set(committed.map((item) => item.id));
		const transient = [...this.transientItems.values()]
			.filter((item) => !committedIds.has(item.itemId))
			.map((item) => chatListItem(item, true));
		return [...committed, ...transient];
	}

	get interaction(): TurnInteraction | undefined {
		return this._interaction;
	}

	get canInterrupt(): boolean {
		return activeTurn(this._thread) !== undefined;
	}

	async initialize(): Promise<void> {
		if (!this.initializePromise) {
			this.initializePromise = this._initialize();
		}
		return this.initializePromise;
	}

	async selectThread(active: IActiveSessionThread): Promise<void> {
		const current = this.activeSession;
		if (!current || active.session.sessionId !== current.session.sessionId) {
			throw new Error(`ChatPaneModel cannot select a Thread from another Session: ${active.session.sessionId}`);
		}
		const previousThreadId = current.threadId;
		const previousModel = current.session.model;
		this.selection = { kind: "session", active };
		if (previousThreadId === active.threadId && this._thread?.threadId === active.threadId) {
			if (!sameModel(previousModel, active.session.model)) void this.loadModels();
			return;
		}
		if (previousThreadId !== active.threadId) {
			void this.chatService.unsubscribeThread(current.session.sessionId, previousThreadId);
		}
		await this.subscribe(active);
	}

	selectUntitledSession(session: IUntitledChatSession): void {
		if (this.selection.kind !== "untitled" || this.selection.session.untitledSessionId !== session.untitledSessionId) {
			throw new Error(`ChatPaneModel cannot select another Untitled Chat Session: ${session.untitledSessionId}`);
		}
		this.selection = { kind: "untitled", session };
		void this.loadModels();
	}

	async selectModel(model: ModelRef): Promise<void> {
		if (this.selection.kind === "untitled") {
			this.sessionService.setUntitledSessionModel(this.selection.session.untitledSessionId, model);
			return;
		}
		await this.sessionService.setModel(this.selection.active.session.sessionId, model);
	}

	async send(text: string, skills?: readonly SkillReference[]): Promise<void> {
		const input = text.trim();
		if (!input) return;
		try {
			this.setState("submitting");
			const active = await this.ensureActiveSession();
			if (this._thread?.threadId !== active.threadId) {
				await this.subscribe(active);
			}
			const thread = this._thread;
			if (!thread || thread.threadId !== active.threadId) {
				throw new Error("Chat Thread is not available");
			}
			const turn = activeTurn(thread);
			if (turn) {
				if (!isSteerableTurn(turn)) {
					throw new Error(`The active ${turn.status} Turn cannot accept steering`);
				}
				if (skills?.length) {
					throw new Error("Skills can only be selected when starting a new Turn");
				}
				await this.chatService.steerTurn({
					sessionId: active.session.sessionId,
					threadId: active.threadId,
					turnId: turn.turnId,
					expectedSequence: thread.sequence,
					text: input,
				});
			} else {
				await this.chatService.startTurn({
					sessionId: active.session.sessionId,
					threadId: active.threadId,
					expectedSequence: thread.sequence,
					text: input,
					skills,
				});
			}
			await this.refreshThread();
			this.setState("ready");
		} catch (error) {
			this.setError(error);
			throw error;
		}
	}

	async executeServerCommand(name: string, argumentsText: string): Promise<void> {
		if (name !== "compact") {
			await this.send(`/${name}${argumentsText ? ` ${argumentsText}` : ""}`);
			return;
		}
		try {
			this.setState("submitting");
			const active = await this.ensureActiveSession();
			if (this._thread?.threadId !== active.threadId) {
				await this.subscribe(active);
			}
			const thread = this._thread;
			if (!thread || thread.threadId !== active.threadId) {
				throw new Error("Chat Thread is not available");
			}
			if (activeTurn(thread)) {
				throw new Error("Context can be compacted only when the active Turn has finished");
			}
			await this.chatService.compactContext({
				sessionId: active.session.sessionId,
				threadId: active.threadId,
				expectedSequence: thread.sequence,
				...(argumentsText.trim() ? { retentionPrompt: argumentsText.trim() } : {}),
			});
			await this.refreshThread();
			this.setState("ready");
		} catch (error) {
			this.setError(error);
			throw error;
		}
	}

	async retryFailedTurn(turnId: string): Promise<void> {
		const turns = this._thread?.turns ?? [];
		const turn = turns.at(-1);
		if (turn?.turnId !== turnId || turn.status !== "failed" || turn.error?.retryable !== true) {
			throw new Error("Only the latest retryable failed Turn can be retried");
		}
		await this.send("Try again.");
	}

	async interrupt(): Promise<void> {
		const thread = this._thread;
		const turn = activeTurn(thread);
		if (!thread || !turn) return;
		try {
			this.setState("submitting");
			await this.chatService.interruptTurn({
				sessionId: thread.sessionId,
				threadId: thread.threadId,
				turnId: turn.turnId,
				expectedSequence: thread.sequence,
			});
			await this.refreshThread();
			this.setState("ready");
		} catch (error) {
			this.setError(error);
		}
	}

	async resolveInteraction(response: AgentResponse): Promise<void> {
		const thread = this._thread;
		const interaction = this._interaction;
		const turn = activeTurn(thread);
		if (!thread || !turn || !interaction) return;
		if (response.type !== interaction.request.type) {
			throw new Error("Interaction response kind does not match request");
		}
		try {
			this.setState("submitting");
			await this.chatService.resolveInteraction({
				sessionId: thread.sessionId,
				threadId: thread.threadId,
				turnId: turn.turnId,
				requestId: interaction.requestId,
				expectedSequence: thread.sequence,
				response,
			});
			this._interaction = undefined;
			await this.refreshThread();
			this.setState("ready");
		} catch (error) {
			this.setError(error);
			throw error;
		}
	}

	private async _initialize(): Promise<void> {
		this.setState("loading");
		if (this.selection.kind === "untitled") {
			await this.loadCatalogs();
			if (!this.disposed && this.selection.kind === "untitled") {
				this.setState("ready");
			}
			return;
		}
		await Promise.all([this.subscribe(this.selection.active), this.loadCatalogs()]);
	}

	private async loadCatalogs(): Promise<void> {
		const [models, slashCommands, skillCommands] = await Promise.allSettled([this.modelEntries(), this.chatService.listSlashCommands(), this.chatService.listSkillCommands()]);
		if (models.status === "fulfilled") this._models = models.value;
		if (slashCommands.status === "fulfilled") this._slashCommands = slashCommands.value;
		if (skillCommands.status === "fulfilled") this._skillCommands = skillCommands.value;
		this._onDidChange.fire();
	}

	private async loadModels(): Promise<void> {
		try {
			this._models = await this.modelEntries();
			this._onDidChange.fire();
		} catch {
			// Keep the last valid catalog when a transient refresh fails.
		}
	}

	private async modelEntries(): Promise<readonly ModelCatalogEntry[]> {
		const [visible, catalog] = await Promise.all([this.chatService.listModels(), this.chatService.listModelCatalog()]);
		const selected = this.selectedModel;
		if (!selected || visible.some(entry => sameModel(entry.model, selected))) return visible;
		const selectedEntry = catalog.find(entry => sameModel(entry.model, selected)) ?? {
			model: selected,
			displayName: selected.model,
			access: "unknown" as const,
			outputTransport: "unary" as const,
		};
		return [...visible, selectedEntry];
	}

	private async loadSkillCommands(): Promise<void> {
		try {
			this._skillCommands = await this.chatService.listSkillCommands();
			this._onDidChange.fire();
		} catch {
			// Keep the last valid catalog when a transient refresh fails.
		}
	}

	private async subscribe(active: IActiveSessionThread): Promise<void> {
		if (
			this.subscriptionThreadId === active.threadId &&
			this.subscriptionPromise
		) {
			return this.subscriptionPromise;
		}
		this.subscriptionThreadId = active.threadId;
		const promise = this.performSubscribe(active);
		this.subscriptionPromise = promise;
		try {
			await promise;
		} finally {
			if (this.subscriptionPromise === promise) {
				this.subscriptionThreadId = undefined;
				this.subscriptionPromise = undefined;
			}
		}
	}

	private async performSubscribe(active: IActiveSessionThread): Promise<void> {
		const generation = ++this.generation;
		const oldThreadId = this._thread?.threadId;
		this._thread = undefined;
		this._interaction = undefined;
		this.transientItems.clear();
		this.resetStreamCursor();
		this.setState("loading");
		if (oldThreadId && oldThreadId !== active.threadId) {
			void this.chatService.unsubscribeThread(active.session.sessionId, oldThreadId);
		}
		try {
			const result = await this.chatService.subscribeThread(active.session.sessionId, active.threadId, 0);
			if (this.disposed || generation !== this.generation) return;
			this._thread = result.thread;
			this.discardCommittedTransientItems();
			for (const update of result.updates) {
				if (update.durableSequence > result.thread.sequence) {
					this.acceptUpdate(update);
				}
			}
			this.setState("ready");
		} catch (error) {
			if (this.disposed || generation !== this.generation) return;
			this.setError(error);
		}
	}

	private acceptUpdate(update: ThreadUpdateEnvelope): void {
		const selectedThreadId = this._thread?.threadId ?? this.threadId;
		if (!selectedThreadId) return;
		if (update.threadId !== selectedThreadId) return;
		if (!this.acceptStreamCursor(update)) return;
		switch (update.update.type) {
			case "committed":
				this.acceptCommittedEvent(update);
				this.scheduleRefresh();
				break;
			case "itemStarted":
				this.transientItems.set(
					update.update.item.itemId,
					update.update.item,
				);
				this._onDidChange.fire();
				break;
			case "itemDelta":
				this.applyDelta(update);
				break;
		}
	}

	private async reconnect(): Promise<void> {
		const active = this.activeSession;
		await Promise.all([
			active ? this.subscribe(active) : Promise.resolve(),
			this.loadCatalogs(),
		]);
	}

	private acceptStreamCursor(update: ThreadUpdateEnvelope): boolean {
		const cursor = update.streamCursor;
		if (!cursor) return true;
		if (cursor.streamInstanceId !== this.streamInstanceId) {
			if (this.retiredStreamInstanceIds.has(cursor.streamInstanceId)) return false;
			if (this.streamInstanceId) this.retiredStreamInstanceIds.add(this.streamInstanceId);
			this.streamInstanceId = cursor.streamInstanceId;
			this.streamSequence = cursor.sequence;
			this.transientItems.clear();
			return true;
		}
		if (cursor.sequence <= this.streamSequence) return false;
		if (cursor.sequence !== this.streamSequence + 1) {
			this.streamSequence = cursor.sequence;
			this.transientItems.clear();
			this.scheduleRefresh();
			return false;
		}
		this.streamSequence = cursor.sequence;
		return true;
	}

	private resetStreamCursor(): void {
		this.streamInstanceId = undefined;
		this.streamSequence = 0;
		this.retiredStreamInstanceIds.clear();
	}

	private acceptCommittedEvent(update: ThreadUpdateEnvelope): void {
		if (update.update.type !== "committed") return;
		const event = update.update.event;
		switch (event.type) {
			case "interactionRequested":
				this._interaction = event.interaction;
				this._onDidChange.fire();
				break;
			case "interactionResolved":
			case "interactionCancelled":
			case "turnCompleted":
			case "turnFailed":
			case "turnInterrupted":
				this._interaction = undefined;
				this._onDidChange.fire();
				break;
			default:
				break;
		}
	}

	private applyDelta(update: ThreadUpdateEnvelope): void {
		if (update.update.type !== "itemDelta") return;
		const item = this.transientItems.get(update.update.itemId);
		if (!item || !("text" in item)) return;
		const delta = update.update.delta;
		if (delta.type !== item.type) return;
		this.transientItems.set(item.itemId, {
			...item,
			text: item.text + delta.text,
		});
		this._onDidChange.fire();
	}

	private scheduleRefresh(): void {
		if (this.readPending) return;
		this.readPending = true;
		queueMicrotask(() => {
			this.readPending = false;
			void this.refreshThread();
		});
	}

	private async refreshThread(): Promise<void> {
		const active = this.activeSession;
		if (!active) return;
		const threadId = active.threadId;
		const generation = this.generation;
		try {
			const thread = await this.chatService.readThread(active.session.sessionId, threadId);
			if (
				this.disposed ||
				generation !== this.generation ||
				thread.threadId !== this.threadId
			) return;
			this._thread = thread;
			this.discardCommittedTransientItems();
			this._error = undefined;
			this._state = "ready";
			this._onDidChange.fire();
		} catch (error) {
			if (!this.disposed && generation === this.generation) {
				this.setError(error);
			}
		}
	}

	private discardCommittedTransientItems(): void {
		const committedIds = new Set(
			this._thread?.turns.flatMap(
				(turn) => turn.items.map((item) => item.itemId),
			) ?? [],
		);
		for (const itemId of this.transientItems.keys()) {
			if (committedIds.has(itemId)) this.transientItems.delete(itemId);
		}
	}

	private setState(state: ChatPaneState, error?: string): void {
		this._state = state;
		this._error = error;
		this._onDidChange.fire();
	}

	private setError(error: unknown): void {
		this.setState(
			"error",
			error instanceof Error ? error.message : "Chat is unavailable.",
		);
	}

	private get activeSession(): IActiveSessionThread | undefined {
		return this.selection.kind === "session" ? this.selection.active : undefined;
	}

	private async ensureActiveSession(): Promise<IActiveSessionThread> {
		if (this.selection.kind === "session") return this.selection.active;
		const untitledSession = this.selection.session;
		const created = await this.sessionService.materializeUntitledSession(untitledSession.untitledSessionId);
		if (this.disposed) {
			this.sessionService.promoteUntitledSession(untitledSession.untitledSessionId, created);
			throw new Error("Untitled Chat Session was closed while its durable Session was being created");
		}
		this.selection = { kind: "session", active: created };
		this.sessionService.promoteUntitledSession(untitledSession.untitledSessionId, created);
		await this.subscribe(created);
		return created;
	}
}

function activeTurn(thread: Thread | undefined): Turn | undefined {
	return [...(thread?.turns ?? [])].reverse().find(
		(turn) =>
			turn.status === "created" ||
			turn.status === "running" ||
			turn.status === "waitingForApproval" ||
			turn.status === "waitingForUserInput" ||
			turn.status === "waitingForCapability" ||
			turn.status === "cancelling",
	);
}

function isSteerableTurn(turn: Turn): boolean {
	return turn.status === "running" || turn.status === "waitingForApproval" || turn.status === "waitingForUserInput";
}

function sameModel(left: ModelRef | null | undefined, right: ModelRef | null | undefined): boolean {
	return left?.provider === right?.provider && left?.model === right?.model;
}
