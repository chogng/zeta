import type { AgentResponse, ModelCatalogEntry, ModelRef, SessionId, SlashCommandDefinition, Thread, ThreadId, ThreadItem, ThreadUpdateEnvelope, Turn, TurnInteraction } from "../../../../../../../generated/app-server/types.js";
import { Emitter, type Event } from "../../../../../base/common/event.js";
import { DisposableOwner } from "../../../../../base/common/lifecycle.js";
import { createUuid } from "../../../../../base/common/uuid.js";
import type { ZetaRendererApi } from "../../../../../platform/app-server/common/renderer-api.js";
import type { IActiveSessionThread, IChatDraft, IWorkbenchSessionService } from "../../../../services/sessions/common/sessionService.js";
import { chatListItem, type IChatListItem } from "../list/chatListItems.js";

export type ChatPaneState =
  | "loading"
  | "ready"
  | "submitting"
  | "error";

/** The local or durable identity currently projected by a Chat pane. */
export type ChatPaneSelection =
  | { readonly kind: "session"; readonly active: IActiveSessionThread }
  | { readonly kind: "draft"; readonly draft: IChatDraft };

/**
 * Projection of one Chat tab, before or after it acquires a durable Thread.
 *
 * Canonical committed state is refreshed from `thread/read`. Transient item
 * updates are layered by Item ID and discarded once the committed snapshot
 * contains the same item.
 */
export class ChatPaneModel extends DisposableOwner {
  private readonly api: ZetaRendererApi;
  private readonly sessionService: IWorkbenchSessionService;
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
  private _models: readonly ModelCatalogEntry[] = [];
  private _slashCommands: readonly SlashCommandDefinition[] = [];

  readonly onDidChange: Event<void> = this._onDidChange.event;

  constructor(api: ZetaRendererApi, selection: ChatPaneSelection, sessionService: IWorkbenchSessionService) {
    super();
    this.api = api;
    this.sessionService = sessionService;
    this.selection = selection;
    const events = api.events.subscribe((notification) => {
      if (notification.method === "thread/update") {
        this.acceptUpdate(notification.params);
      }
    });
    this.defer(() => events.dispose());
    const connection = api.appServer.onConnectionState((state) => {
      if (state === "ready") void this.reconnect();
    });
    this.defer(() => connection.dispose());
    this.defer(() => {
      this.disposed = true;
      this.generation++;
      const threadId = this.threadId;
      if (threadId) void this.api.thread.unsubscribe({ threadId });
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

  get draftId(): string | undefined {
    return this.selection.kind === "draft" ? this.selection.draft.draftId : undefined;
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

  get selectedModel(): ModelRef | undefined {
    return this.selection.kind === "draft"
      ? this.selection.draft.model
      : this.selection.active.session.model ?? undefined;
  }

  get items(): readonly IChatListItem[] {
    const committed = this._thread?.turns.flatMap(
      (turn) => turn.items.map((item) => chatListItem(item)),
    ) ?? [];
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
      if (!sameModel(previousModel, active.session.model)) this._onDidChange.fire();
      return;
    }
    if (previousThreadId !== active.threadId) {
      void this.api.thread.unsubscribe({ threadId: previousThreadId });
    }
    await this.subscribe(active);
  }

  selectDraft(draft: IChatDraft): void {
    if (this.selection.kind !== "draft" || this.selection.draft.draftId !== draft.draftId) {
      throw new Error(`ChatPaneModel cannot select another Chat Draft: ${draft.draftId}`);
    }
    this.selection = { kind: "draft", draft };
    this._onDidChange.fire();
  }

  async selectModel(model: ModelRef): Promise<void> {
    if (this.selection.kind === "draft") {
      this.sessionService.setDraftModel(this.selection.draft.draftId, model);
      return;
    }
    await this.sessionService.setModel(this.selection.active.session.sessionId, model);
  }

  async send(text: string): Promise<void> {
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
      await this.api.turn.start({
        commandId: commandId("turn"),
        sessionId: active.session.sessionId,
        threadId: active.threadId,
        expectedSequence: thread.sequence,
        input: [{ type: "text", text: input }],
      });
      await this.refreshThread();
      this.setState("ready");
    } catch (error) {
      this.setError(error);
      throw error;
    }
  }

  async interrupt(): Promise<void> {
    const thread = this._thread;
    const turn = activeTurn(thread);
    if (!thread || !turn) return;
    try {
      this.setState("submitting");
      await this.api.turn.interrupt({
        commandId: commandId("interrupt"),
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
      await this.api.turn.resolveInteraction({
        commandId: commandId("interaction"),
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
    if (this.selection.kind === "draft") {
      await this.loadCatalogs();
      if (!this.disposed && this.selection.kind === "draft") {
        this.setState("ready");
      }
      return;
    }
    await Promise.all([this.subscribe(this.selection.active), this.loadCatalogs()]);
  }

  private async loadCatalogs(): Promise<void> {
    const [models, slashCommands] = await Promise.allSettled([this.api.model.list(), this.api.appServer.getSlashCommands()]);
    this._models = models.status === "fulfilled" ? models.value.models : [];
    this._slashCommands = slashCommands.status === "fulfilled" ? slashCommands.value : [];
    this._onDidChange.fire();
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
      void this.api.thread.unsubscribe({ threadId: oldThreadId });
    }
    try {
      const result = await this.api.thread.subscribe({
        threadId: active.threadId,
        afterSequence: 0,
      });
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
      case "planUpdated":
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
      const result = await this.api.thread.read({ threadId });
      if (
        this.disposed ||
        generation !== this.generation ||
        result.thread.threadId !== this.threadId
      ) return;
      this._thread = result.thread;
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
    const draft = this.selection.draft;
    const created = await this.sessionService.materializeDraft(draft.draftId);
    if (this.disposed) {
      this.sessionService.promoteDraft(draft.draftId, created);
      throw new Error("Chat Draft was closed while its Session was being created");
    }
    this.selection = { kind: "session", active: created };
    this.sessionService.promoteDraft(draft.draftId, created);
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

function commandId(kind: string): string {
  return `desktop-${kind}-${createUuid()}`;
}

function sameModel(left: ModelRef | null | undefined, right: ModelRef | null | undefined): boolean {
  return left?.provider === right?.provider && left?.model === right?.model;
}
