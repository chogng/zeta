import type { AgentResponse, Thread, ThreadId, ThreadItem, ThreadUpdateEnvelope, Turn, TurnInteraction } from "../../../../../../../generated/app-server/types.js";
import { Emitter, type Event } from "../../../../../base/common/event.js";
import { DisposableOwner } from "../../../../../base/common/lifecycle.js";
import { createUuid } from "../../../../../base/common/uuid.js";
import type { ZetaRendererApi } from "../../../../../platform/app-server/common/renderer-api.js";
import type { IActiveSessionThread } from "../../../../services/sessions/common/sessionService.js";
import { chatListItem, type IChatListItem } from "../list/chatListItems.js";

export type ChatPaneState =
  | "loading"
  | "ready"
  | "submitting"
  | "error";

/**
 * Projection of the selected Thread inside one Session-owned Chat pane.
 *
 * Canonical committed state is refreshed from `thread/read`. Transient item
 * updates are layered by Item ID and discarded once the committed snapshot
 * contains the same item.
 */
export class ChatPaneModel extends DisposableOwner {
  readonly #api: ZetaRendererApi;
  readonly #onDidChange = this.own(new Emitter<void>());
  readonly #transientItems = new Map<string, ThreadItem>();
  #selection: IActiveSessionThread;
  #thread: Thread | undefined;
  #interaction: TurnInteraction | undefined;
  #state: ChatPaneState = "loading";
  #error: string | undefined;
  #generation = 0;
  #disposed = false;
  #readPending = false;
  #initializePromise: Promise<void> | undefined;
  #subscriptionThreadId: ThreadId | undefined;
  #subscriptionPromise: Promise<void> | undefined;
  #streamInstanceId: string | undefined;
  #streamSequence = 0;

  readonly onDidChange: Event<void> = this.#onDidChange.event;

  constructor(api: ZetaRendererApi, active: IActiveSessionThread) {
    super();
    this.#api = api;
    this.#selection = active;
    const events = api.events.subscribe((notification) => {
      if (notification.method === "thread/update") {
        this.#acceptUpdate(notification.params);
      }
    });
    this.defer(() => events.dispose());
    const connection = api.appServer.onConnectionState((state) => {
      if (state === "ready") void this.#reconnect();
    });
    this.defer(() => connection.dispose());
    this.defer(() => {
      this.#disposed = true;
      this.#generation++;
      void this.#api.thread.unsubscribe({ threadId: this.#selection.threadId });
      this.#transientItems.clear();
      this.#resetStreamCursor();
    });
    void this.initialize();
  }

  get state(): ChatPaneState {
    return this.#state;
  }

  get error(): string | undefined {
    return this.#error;
  }

  get thread(): Thread | undefined {
    return this.#thread;
  }

  get threadId(): ThreadId {
    return this.#selection.threadId;
  }

  get items(): readonly IChatListItem[] {
    const committed = this.#thread?.turns.flatMap(
      (turn) => turn.items.map((item) => chatListItem(item)),
    ) ?? [];
    const committedIds = new Set(committed.map((item) => item.id));
    const transient = [...this.#transientItems.values()]
      .filter((item) => !committedIds.has(item.itemId))
      .map((item) => chatListItem(item, true));
    return [...committed, ...transient];
  }

  get interaction(): TurnInteraction | undefined {
    return this.#interaction;
  }

  get canInterrupt(): boolean {
    return activeTurn(this.#thread) !== undefined;
  }

  async initialize(): Promise<void> {
    if (!this.#initializePromise) {
      this.#initializePromise = this.#initialize();
    }
    return this.#initializePromise;
  }

  async selectThread(active: IActiveSessionThread): Promise<void> {
    if (active.session.sessionId !== this.#selection.session.sessionId) {
      throw new Error(`ChatPaneModel cannot select a Thread from another Session: ${active.session.sessionId}`);
    }
    const previousThreadId = this.#selection.threadId;
    this.#selection = active;
    if (previousThreadId === active.threadId && this.#thread?.threadId === active.threadId) return;
    if (previousThreadId !== active.threadId) {
      void this.#api.thread.unsubscribe({ threadId: previousThreadId });
    }
    await this.#subscribe(active);
  }

  async send(text: string): Promise<void> {
    const input = text.trim();
    if (!input) return;
    try {
      this.#setState("submitting");
      const active = this.#selection;
      if (this.#thread?.threadId !== active.threadId) {
        await this.#subscribe(active);
      }
      const thread = this.#thread;
      if (!thread || thread.threadId !== active.threadId) {
        throw new Error("Chat Thread is not available");
      }
      await this.#api.turn.start({
        commandId: commandId("turn"),
        sessionId: active.session.sessionId,
        threadId: active.threadId,
        expectedSequence: thread.sequence,
        input: [{ type: "text", text: input }],
      });
      await this.#refreshThread();
      this.#setState("ready");
    } catch (error) {
      this.#setError(error);
      throw error;
    }
  }

  async interrupt(): Promise<void> {
    const thread = this.#thread;
    const turn = activeTurn(thread);
    if (!thread || !turn) return;
    try {
      this.#setState("submitting");
      await this.#api.turn.interrupt({
        commandId: commandId("interrupt"),
        sessionId: thread.sessionId,
        threadId: thread.threadId,
        turnId: turn.turnId,
        expectedSequence: thread.sequence,
      });
      await this.#refreshThread();
      this.#setState("ready");
    } catch (error) {
      this.#setError(error);
    }
  }

  async resolveInteraction(response: AgentResponse): Promise<void> {
    const thread = this.#thread;
    const interaction = this.#interaction;
    const turn = activeTurn(thread);
    if (!thread || !turn || !interaction) return;
    if (response.type !== interaction.request.type) {
      throw new Error("Interaction response kind does not match request");
    }
    try {
      this.#setState("submitting");
      await this.#api.turn.resolveInteraction({
        commandId: commandId("interaction"),
        sessionId: thread.sessionId,
        threadId: thread.threadId,
        turnId: turn.turnId,
        requestId: interaction.requestId,
        expectedSequence: thread.sequence,
        response,
      });
      this.#interaction = undefined;
      await this.#refreshThread();
      this.#setState("ready");
    } catch (error) {
      this.#setError(error);
      throw error;
    }
  }

  async #initialize(): Promise<void> {
    this.#setState("loading");
    await this.#subscribe(this.#selection);
  }

  async #subscribe(active: IActiveSessionThread): Promise<void> {
    if (
      this.#subscriptionThreadId === active.threadId &&
      this.#subscriptionPromise
    ) {
      return this.#subscriptionPromise;
    }
    this.#subscriptionThreadId = active.threadId;
    const promise = this.#performSubscribe(active);
    this.#subscriptionPromise = promise;
    try {
      await promise;
    } finally {
      if (this.#subscriptionPromise === promise) {
        this.#subscriptionThreadId = undefined;
        this.#subscriptionPromise = undefined;
      }
    }
  }

  async #performSubscribe(active: IActiveSessionThread): Promise<void> {
    const generation = ++this.#generation;
    const oldThreadId = this.#thread?.threadId;
    this.#thread = undefined;
    this.#interaction = undefined;
    this.#transientItems.clear();
    this.#resetStreamCursor();
    this.#setState("loading");
    if (oldThreadId && oldThreadId !== active.threadId) {
      void this.#api.thread.unsubscribe({ threadId: oldThreadId });
    }
    try {
      const result = await this.#api.thread.subscribe({
        threadId: active.threadId,
        afterSequence: 0,
      });
      if (this.#disposed || generation !== this.#generation) return;
      this.#thread = result.thread;
      this.#discardCommittedTransientItems();
      for (const update of result.updates) {
        if (update.durableSequence > result.thread.sequence) {
          this.#acceptUpdate(update);
        }
      }
      this.#setState("ready");
    } catch (error) {
      if (this.#disposed || generation !== this.#generation) return;
      this.#setError(error);
    }
  }

  #acceptUpdate(update: ThreadUpdateEnvelope): void {
    const selectedThreadId = this.#thread?.threadId ?? this.#selection.threadId;
    if (update.threadId !== selectedThreadId) return;
    if (!this.#acceptStreamCursor(update)) return;
    switch (update.update.type) {
      case "committed":
        this.#acceptCommittedEvent(update);
        this.#scheduleRefresh();
        break;
      case "itemStarted":
        this.#transientItems.set(
          update.update.item.itemId,
          update.update.item,
        );
        this.#onDidChange.fire();
        break;
      case "itemDelta":
        this.#applyDelta(update);
        break;
      case "planUpdated":
        break;
    }
  }

  async #reconnect(): Promise<void> {
    await this.#subscribe(this.#selection);
  }

  #acceptStreamCursor(update: ThreadUpdateEnvelope): boolean {
    const cursor = update.streamCursor;
    if (!cursor) return true;
    if (cursor.streamInstanceId !== this.#streamInstanceId) {
      this.#streamInstanceId = cursor.streamInstanceId;
      this.#streamSequence = cursor.sequence;
      this.#transientItems.clear();
      return true;
    }
    if (cursor.sequence <= this.#streamSequence) return false;
    if (cursor.sequence !== this.#streamSequence + 1) {
      this.#streamSequence = cursor.sequence;
      this.#transientItems.clear();
      this.#scheduleRefresh();
      return false;
    }
    this.#streamSequence = cursor.sequence;
    return true;
  }

  #resetStreamCursor(): void {
    this.#streamInstanceId = undefined;
    this.#streamSequence = 0;
  }

  #acceptCommittedEvent(update: ThreadUpdateEnvelope): void {
    if (update.update.type !== "committed") return;
    const event = update.update.event;
    switch (event.type) {
      case "interactionRequested":
        this.#interaction = event.interaction;
        this.#onDidChange.fire();
        break;
      case "interactionResolved":
      case "interactionCancelled":
      case "turnCompleted":
      case "turnFailed":
      case "turnInterrupted":
        this.#interaction = undefined;
        this.#onDidChange.fire();
        break;
      default:
        break;
    }
  }

  #applyDelta(update: ThreadUpdateEnvelope): void {
    if (update.update.type !== "itemDelta") return;
    const item = this.#transientItems.get(update.update.itemId);
    if (!item || !("text" in item)) return;
    const delta = update.update.delta;
    if (delta.type !== item.type) return;
    this.#transientItems.set(item.itemId, {
      ...item,
      text: item.text + delta.text,
    });
    this.#onDidChange.fire();
  }

  #scheduleRefresh(): void {
    if (this.#readPending) return;
    this.#readPending = true;
    queueMicrotask(() => {
      this.#readPending = false;
      void this.#refreshThread();
    });
  }

  async #refreshThread(): Promise<void> {
    const threadId = this.#selection.threadId;
    const generation = this.#generation;
    try {
      const result = await this.#api.thread.read({ threadId });
      if (
        this.#disposed ||
        generation !== this.#generation ||
        result.thread.threadId !== this.#selection.threadId
      ) return;
      this.#thread = result.thread;
      this.#discardCommittedTransientItems();
      this.#error = undefined;
      this.#state = "ready";
      this.#onDidChange.fire();
    } catch (error) {
      if (!this.#disposed && generation === this.#generation) {
        this.#setError(error);
      }
    }
  }

  #discardCommittedTransientItems(): void {
    const committedIds = new Set(
      this.#thread?.turns.flatMap(
        (turn) => turn.items.map((item) => item.itemId),
      ) ?? [],
    );
    for (const itemId of this.#transientItems.keys()) {
      if (committedIds.has(itemId)) this.#transientItems.delete(itemId);
    }
  }

  #setState(state: ChatPaneState, error?: string): void {
    this.#state = state;
    this.#error = error;
    this.#onDidChange.fire();
  }

  #setError(error: unknown): void {
    this.#setState(
      "error",
      error instanceof Error ? error.message : "Chat is unavailable.",
    );
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
