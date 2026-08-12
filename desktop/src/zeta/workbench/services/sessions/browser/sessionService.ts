import type { ModelRef as ModelRefDto, Session as SessionDto } from "../../../../../../generated/app-server/types.js";
import { Emitter } from "../../../../base/common/event.js";
import { DisposableOwner } from "../../../../base/common/lifecycle.js";
import { createUuid } from "../../../../base/common/uuid.js";
import type { IServerEventApi } from "../../../../platform/app-server/common/appServerApi.js";
import type { ISessionApi } from "../../../../platform/sessions/common/sessionApi.js";
import type { IActiveSessionThread, IUntitledChatSession, IWorkbenchSessionService, ModelRef, Session, SessionId, ThreadId, WorkbenchSessionState } from "../common/sessionService.js";

/** App Server-backed active Session selector for one workbench window. */
export class WorkbenchSessionService extends DisposableOwner implements IWorkbenchSessionService {
  private readonly api: ISessionApi;
  private readonly _onDidChange = this.own(new Emitter<void>());
  private _sessions: readonly Session[] = [];
  private _active: IActiveSessionThread | undefined;
  private _untitledSessions: readonly IUntitledChatSession[] = [];
  private _activeUntitledSessionId: string | undefined;
  private _state: WorkbenchSessionState = "loading";
  private _error: string | undefined;
  private initializePromise: Promise<void> | undefined;
  private readonly subscribedSessionIds = new Set<SessionId>();
  private readonly pendingSessionSequences = new Map<SessionId, number>();
  private readonly refreshes = new Map<SessionId, Promise<void>>();

  readonly onDidChange = this._onDidChange.event;

  constructor(api: ISessionApi | { readonly session: ISessionApi; readonly events?: IServerEventApi }) {
    super();
    this.api = "session" in api ? api.session : api;
    const events = "session" in api ? api.events : undefined;
    if (events) {
      const subscription = events.subscribe(event => {
        if (event.method !== "session/update") return;
        this.acceptSessionUpdate(event.params.sessionId, event.params.durableSequence);
      });
      this.defer(() => subscription.dispose());
    }
    this.defer(() => {
      for (const sessionId of this.subscribedSessionIds) {
        void this.api.unsubscribe({ sessionId }).catch(error => console.error(`Failed to unsubscribe Session '${sessionId}'`, error));
      }
      this.subscribedSessionIds.clear();
      this.pendingSessionSequences.clear();
      this.refreshes.clear();
    });
  }

  get sessions(): readonly Session[] { return this._sessions; }
  get active(): IActiveSessionThread | undefined { return this._active; }
  get untitledSessions(): readonly IUntitledChatSession[] { return this._untitledSessions; }
  get activeUntitledSession(): IUntitledChatSession | undefined { return this._untitledSessions.find((session) => session.untitledSessionId === this._activeUntitledSessionId); }
  get state(): WorkbenchSessionState { return this._state; }
  get error(): string | undefined { return this._error; }

  initialize(): Promise<void> {
    if (!this.initializePromise || this._state === "error") this.initializePromise = this.loadSessions();
    return this.initializePromise;
  }

  selectThread(sessionId: SessionId, threadId: ThreadId): void {
    const session = this._sessions.find((candidate) => candidate.sessionId === sessionId);
    const thread = session?.threads.find((candidate) => candidate.threadId === threadId && candidate.status === "active");
    if (!session || !thread || session.status !== "active") throw new Error(`Active Session Thread is not available: ${threadId}`);
    if (this._active?.session.sessionId === sessionId && this._active.threadId === threadId && this._activeUntitledSessionId === undefined) return;
    this._active = { session, threadId };
    this._activeUntitledSessionId = undefined;
    this._error = undefined;
    this._onDidChange.fire();
  }

  createUntitledSession(title = "New Chat"): IUntitledChatSession {
    const session = this.addUntitledSession(title);
    this._activeUntitledSessionId = session.untitledSessionId;
    this._error = undefined;
    this._onDidChange.fire();
    return session;
  }

  selectUntitledSession(untitledSessionId: string): void {
    if (!this._untitledSessions.some((session) => session.untitledSessionId === untitledSessionId)) throw new Error(`Untitled Chat Session is not available: ${untitledSessionId}`);
    if (this._activeUntitledSessionId === untitledSessionId) return;
    this._activeUntitledSessionId = untitledSessionId;
    this._error = undefined;
    this._onDidChange.fire();
  }

  discardUntitledSession(untitledSessionId: string): void {
    const sessions = this._untitledSessions.filter((session) => session.untitledSessionId !== untitledSessionId);
    if (sessions.length === this._untitledSessions.length) return;
    this._untitledSessions = sessions;
    if (this._activeUntitledSessionId === untitledSessionId) this._activeUntitledSessionId = sessions[0]?.untitledSessionId;
    this.restoreAvailableSelection();
    this._onDidChange.fire();
  }

  setUntitledSessionModel(untitledSessionId: string, model: ModelRef): void {
    const current = this._untitledSessions.find((session) => session.untitledSessionId === untitledSessionId);
    if (!current) throw new Error(`Untitled Chat Session is not available: ${untitledSessionId}`);
    if (sameModel(current.model, model)) return;
    this._untitledSessions = this._untitledSessions.map((session) => session.untitledSessionId === untitledSessionId ? { ...session, model } : session);
    this._onDidChange.fire();
  }

  async materializeUntitledSession(untitledSessionId: string): Promise<IActiveSessionThread> {
    const session = this._untitledSessions.find((candidate) => candidate.untitledSessionId === untitledSessionId);
    if (!session) throw new Error(`Untitled Chat Session is not available: ${untitledSessionId}`);
    return this.createSession(session.title, session.model);
  }

  promoteUntitledSession(untitledSessionId: string, active: IActiveSessionThread): void {
    const wasActive = this._activeUntitledSessionId === untitledSessionId;
    this._untitledSessions = this._untitledSessions.filter((session) => session.untitledSessionId !== untitledSessionId);
    if (wasActive) this._activeUntitledSessionId = undefined;
    this._sessions = [active.session, ...this._sessions.filter((session) => session.sessionId !== active.session.sessionId)];
    if (wasActive || !this._active) this._active = active;
    this.setState("ready");
  }

  async ensureActiveThread(): Promise<IActiveSessionThread> {
    await this.initialize();
    return this._active ?? this.startNewSession();
  }

  async startNewSession(title = "New Chat"): Promise<IActiveSessionThread> {
    const active = await this.createSession(title);
    this.activateSession(active);
    return active;
  }

  async archiveSession(sessionId: SessionId): Promise<void> {
    await this.initialize();
    const session = this._sessions.find((candidate) => candidate.sessionId === sessionId && candidate.status === "active");
    if (!session) throw new Error(`Active Session is not available: ${sessionId}`);
    this.setState("archiving");
    try {
      const result = await this.api.archive({ commandId: commandId("archive-session"), sessionId, expectedSequence: session.sequence });
      this.replaceSession(toSession(result.session));
      await this.unsubscribeSession(sessionId);
      if (this._active?.session.sessionId === sessionId) this._active = firstActiveThread(this._sessions);
      this.restoreAvailableSelection();
      this.setState("ready");
    } catch (error) {
      this.setError(error);
      throw error;
    }
  }

  async stopSession(sessionId: SessionId): Promise<void> {
    await this.initialize();
    const session = this._sessions.find((candidate) => candidate.sessionId === sessionId && candidate.status === "active");
    if (!session) throw new Error(`Active Session is not available: ${sessionId}`);
    this.setState("stopping");
    try {
      const result = await this.api.stop({ commandId: commandId("stop-session"), sessionId, expectedSequence: session.sequence });
      this.replaceSession(toSession(result.session));
      await this.unsubscribeSession(sessionId);
      if (this._active?.session.sessionId === sessionId) this._active = firstActiveThread(this._sessions);
      this.restoreAvailableSelection();
      this.setState("ready");
    } catch (error) {
      this.setError(error);
      throw error;
    }
  }

  async setModel(sessionId: SessionId, model: ModelRef): Promise<void> {
    await this.initialize();
    const session = this._sessions.find((candidate) => candidate.sessionId === sessionId && candidate.status === "active");
    if (!session) throw new Error(`Active Session is not available: ${sessionId}`);
    try {
      const result = await this.api.setModel({ commandId: commandId("session-model"), sessionId, expectedSequence: session.sequence, model });
      const updated = toSession(result.session);
      this.replaceSession(updated);
      if (this._active?.session.sessionId === sessionId) this._active = { session: updated, threadId: this._active.threadId };
      this._error = undefined;
      this._onDidChange.fire();
    } catch (error) {
      this.setError(error);
      throw error;
    }
  }

  private async createSession(title: string, model: ModelRef | undefined = undefined): Promise<IActiveSessionThread> {
    this.setState("creating");
    try {
      const created = await this.api.create({ commandId: commandId("session"), title });
      const thread = await this.api.createThread({ commandId: commandId("thread"), sessionId: created.session.sessionId, expectedSequence: created.session.sequence, title: "Main" });
      const result = model && !sameModel(thread.session.model, model)
        ? await this.api.setModel({ commandId: commandId("session-model"), sessionId: thread.session.sessionId, expectedSequence: thread.session.sequence, model })
        : thread;
      const session = toSession(result.session);
      const subscribed = await this.subscribeSession(session);
      if (!subscribed.threads.some(candidate => candidate.threadId === thread.threadId && candidate.status === "active")) {
        throw new Error(`Created Thread is missing from subscribed Session snapshot: ${thread.threadId}`);
      }
      return { session: subscribed, threadId: thread.threadId };
    } catch (error) {
      this.setError(error);
      throw error;
    }
  }

  private async loadSessions(): Promise<void> {
    this.setState("loading");
    try {
      const result = await this.api.list();
      const sessions = result.sessions.map(toSession);
      this._sessions = await Promise.all(sessions.map(session => this.subscribeSession(session)));
      this._active = firstActiveThread(this._sessions);
      this.restoreAvailableSelection();
      this.setState("ready");
    } catch (error) {
      this.setError(error);
    }
  }

  private setState(state: WorkbenchSessionState): void {
    this._state = state;
    this._error = undefined;
    this._onDidChange.fire();
  }

  private setError(error: unknown): void {
    this.restoreAvailableSelection();
    this._state = "error";
    this._error = error instanceof Error ? error.message : "Unable to load sessions.";
    this._onDidChange.fire();
  }

  private addUntitledSession(title: string): IUntitledChatSession {
    const session = { untitledSessionId: createUuid(), title, model: undefined };
    this._untitledSessions = [session, ...this._untitledSessions];
    return session;
  }

  private restoreAvailableSelection(): void {
    if (this.activeUntitledSession || this._active) return;
    this._activeUntitledSessionId = this._untitledSessions[0]?.untitledSessionId;
  }

  private activateSession(active: IActiveSessionThread): void {
    this._sessions = [active.session, ...this._sessions.filter((session) => session.sessionId !== active.session.sessionId)];
    this._active = active;
    this._activeUntitledSessionId = undefined;
    this.setState("ready");
  }

  private async subscribeSession(session: Session): Promise<Session> {
    if (session.status !== "active") return session;
    const result = await this.api.subscribe({ sessionId: session.sessionId, afterSequence: session.sequence });
    this.subscribedSessionIds.add(session.sessionId);
    const subscribed = toSession(result.session);
    if (subscribed.sessionId !== session.sessionId) {
      await this.unsubscribeSession(session.sessionId);
      throw new Error(`Session subscription returned '${subscribed.sessionId}' for '${session.sessionId}'`);
    }
    if (subscribed.status !== "active") await this.unsubscribeSession(session.sessionId);
    return subscribed;
  }

  private async unsubscribeSession(sessionId: SessionId): Promise<void> {
    if (!this.subscribedSessionIds.delete(sessionId)) return;
    this.pendingSessionSequences.delete(sessionId);
    await this.api.unsubscribe({ sessionId });
  }

  private acceptSessionUpdate(sessionId: SessionId, durableSequence: number): void {
    const session = this._sessions.find(candidate => candidate.sessionId === sessionId);
    if (!session || durableSequence <= session.sequence) return;
    this.pendingSessionSequences.set(sessionId, Math.max(durableSequence, this.pendingSessionSequences.get(sessionId) ?? 0));
    if (this.refreshes.has(sessionId)) return;
    const refresh = this.refreshSessionUntilCurrent(sessionId).finally(() => this.refreshes.delete(sessionId));
    this.refreshes.set(sessionId, refresh);
  }

  private async refreshSessionUntilCurrent(sessionId: SessionId): Promise<void> {
    try {
      while (true) {
        const expectedSequence = this.pendingSessionSequences.get(sessionId);
        const current = this._sessions.find(candidate => candidate.sessionId === sessionId);
        if (expectedSequence === undefined || !current || current.sequence >= expectedSequence) {
          this.pendingSessionSequences.delete(sessionId);
          return;
        }
        const result = await this.api.subscribe({ sessionId, afterSequence: current.sequence });
        this.subscribedSessionIds.add(sessionId);
        const refreshed = toSession(result.session);
        if (refreshed.sessionId !== sessionId) {
          throw new Error(`Session refresh returned '${refreshed.sessionId}' for '${sessionId}'`);
        }
        if (refreshed.sequence <= current.sequence && refreshed.sequence < expectedSequence) {
          throw new Error(`Session subscription did not advance '${sessionId}' beyond sequence ${current.sequence}`);
        }
        this.replaceSession(refreshed);
        if (this._active?.session.sessionId === sessionId) {
          this._active = refreshed.status === "active"
            ? activeThread(refreshed, this._active.threadId) ?? firstActiveThread(this._sessions)
            : firstActiveThread(this._sessions);
        }
        this.restoreAvailableSelection();
        this._error = undefined;
        this._onDidChange.fire();
        if (refreshed.status !== "active") await this.unsubscribeSession(sessionId);
      }
    } catch (error) {
      this.setError(error);
    }
  }

  private replaceSession(session: Session): void {
    this._sessions = this._sessions.map(candidate => candidate.sessionId === session.sessionId ? session : candidate);
  }
}

function toSession(session: SessionDto): Session {
  return {
    sessionId: session.sessionId,
    title: session.title,
    status: session.status,
    model: session.model ? toModelRef(session.model) : session.model,
    sequence: session.sequence,
    threads: session.threads.map((thread) => ({ ...thread, origin: { ...thread.origin } })),
  };
}

function toModelRef(model: ModelRefDto): ModelRef { return { provider: model.provider, model: model.model }; }

function firstActiveThread(sessions: readonly Session[]): IActiveSessionThread | undefined {
  for (const session of sessions) {
    if (session.status !== "active") continue;
    const thread = session.threads.find((candidate) => candidate.status === "active");
    if (thread) return { session, threadId: thread.threadId };
  }
  return undefined;
}

function activeThread(session: Session, threadId: ThreadId): IActiveSessionThread | undefined {
  return session.threads.some(thread => thread.threadId === threadId && thread.status === "active")
    ? { session, threadId }
    : undefined;
}

function commandId(kind: string): string { return `desktop-${kind}-${createUuid()}`; }

function sameModel(left: ModelRef | ModelRefDto | null | undefined, right: ModelRef | ModelRefDto | null | undefined): boolean {
  return left?.provider === right?.provider && left?.model === right?.model;
}
