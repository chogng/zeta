import type { ModelRef, Session, SessionId, ThreadId } from "../../../../../../generated/app-server/types.js";
import { Emitter, type Event } from "../../../../base/common/event.js";
import { DisposableOwner } from "../../../../base/common/lifecycle.js";
import { createUuid } from "../../../../base/common/uuid.js";
import type { IRendererHost } from "../../../../platform/renderer/common/rendererHost.js";
import { createServiceIdentifier } from "../../../../platform/instantiation/common/instantiation.js";

/** Selected Session and Thread shared by session navigation and features. */
export interface IActiveSessionThread {
  readonly session: Session;
  readonly threadId: ThreadId;
}

/**
 * An untitled Chat session that has not yet acquired a durable Session identity.
 *
 * Untitled sessions are window-local presentation state. They retain user
 * choices until the first send materializes the corresponding durable Session
 * and root Thread.
 */
export interface IUntitledChatSession {
  readonly untitledSessionId: string;
  readonly title: string;
  readonly model: ModelRef | undefined;
}

/** Current initialization or mutation state of the session projection. */
export type WorkbenchSessionState =
  | "loading"
  | "ready"
  | "creating"
  | "archiving"
  | "error";

/**
 * Owns durable Session selection and untitled Chat sessions for one workbench window.
 *
 * Feature views consume the durable selection instead of choosing an
 * arbitrary Thread independently. An untitled session becomes durable only
 * when a Chat pane asks to materialize it for its first send. Once initialization
 * settles, the service selects an available active Thread or untitled session.
 * It permits an empty selection after the last Chat tab closes; the Chat view
 * owns creation of a new untitled tab when its host becomes visible again.
 */
export interface IWorkbenchSessionService {
  readonly onDidChange: Event<void>;
  readonly sessions: readonly Session[];
  readonly active: IActiveSessionThread | undefined;
  readonly untitledSessions: readonly IUntitledChatSession[];
  readonly activeUntitledSession: IUntitledChatSession | undefined;
  readonly state: WorkbenchSessionState;
  readonly error: string | undefined;

  initialize(): Promise<void>;
  selectThread(sessionId: SessionId, threadId: ThreadId): void;
  createUntitledSession(title?: string): IUntitledChatSession;
  selectUntitledSession(untitledSessionId: string): void;
  discardUntitledSession(untitledSessionId: string): void;
  setUntitledSessionModel(untitledSessionId: string, model: ModelRef): void;
  materializeUntitledSession(untitledSessionId: string): Promise<IActiveSessionThread>;
  promoteUntitledSession(untitledSessionId: string, active: IActiveSessionThread): void;
  ensureActiveThread(): Promise<IActiveSessionThread>;
  startNewSession(title?: string): Promise<IActiveSessionThread>;
  archiveSession(sessionId: SessionId): Promise<void>;
  setModel(sessionId: SessionId, model: ModelRef): Promise<void>;
}

export const IWorkbenchSessionService =
  createServiceIdentifier<IWorkbenchSessionService>(
    "workbenchSessionService",
  );

/**
 * App Server-backed active Session selector.
 *
 * Until the protocol exposes recency metadata, initialization preserves the
 * server list order and selects its first active Thread.
 */
export class WorkbenchSessionService
  extends DisposableOwner
  implements IWorkbenchSessionService {
  private readonly api: IRendererHost;
  private readonly _onDidChange = this.own(new Emitter<void>());
  private _sessions: readonly Session[] = [];
  private _active: IActiveSessionThread | undefined;
  private _untitledSessions: readonly IUntitledChatSession[] = [];
  private _activeUntitledSessionId: string | undefined;
  private _state: WorkbenchSessionState = "loading";
  private _error: string | undefined;
  private initializePromise: Promise<void> | undefined;

  readonly onDidChange = this._onDidChange.event;

  constructor(api: IRendererHost) {
    super();
    this.api = api;
  }

  get sessions(): readonly Session[] {
    return this._sessions;
  }

  get active(): IActiveSessionThread | undefined {
    return this._active;
  }

  get untitledSessions(): readonly IUntitledChatSession[] {
    return this._untitledSessions;
  }

  get activeUntitledSession(): IUntitledChatSession | undefined {
    return this._untitledSessions.find(
      (session) => session.untitledSessionId === this._activeUntitledSessionId,
    );
  }

  get state(): WorkbenchSessionState {
    return this._state;
  }

  get error(): string | undefined {
    return this._error;
  }

  initialize(): Promise<void> {
    if (!this.initializePromise || this._state === "error") {
      this.initializePromise = this.loadSessions();
    }
    return this.initializePromise;
  }

  selectThread(sessionId: SessionId, threadId: ThreadId): void {
    const session = this._sessions.find(
      (candidate) => candidate.sessionId === sessionId,
    );
    const thread = session?.threads.find(
      (candidate) =>
        candidate.threadId === threadId &&
        candidate.status === "active",
    );
    if (!session || !thread || session.status !== "active") {
      throw new Error(`Active Session Thread is not available: ${threadId}`);
    }
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
    if (!this._untitledSessions.some((session) => session.untitledSessionId === untitledSessionId)) {
      throw new Error(`Untitled Chat Session is not available: ${untitledSessionId}`);
    }
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
    this._untitledSessions = this._untitledSessions.map((session) =>
      session.untitledSessionId === untitledSessionId ? { ...session, model } : session,
    );
    this._onDidChange.fire();
  }

  /** Creates server state for an untitled session without changing the visible tab yet. */
  async materializeUntitledSession(untitledSessionId: string): Promise<IActiveSessionThread> {
    const session = this._untitledSessions.find((candidate) => candidate.untitledSessionId === untitledSessionId);
    if (!session) throw new Error(`Untitled Chat Session is not available: ${untitledSessionId}`);
    return this.createSession(session.title, session.model);
  }

  /** Replaces an untitled session with its already-created durable Session. */
  promoteUntitledSession(untitledSessionId: string, active: IActiveSessionThread): void {
    const wasActive = this._activeUntitledSessionId === untitledSessionId;
    this._untitledSessions = this._untitledSessions.filter((session) => session.untitledSessionId !== untitledSessionId);
    if (wasActive) this._activeUntitledSessionId = undefined;
    this._sessions = [
      active.session,
      ...this._sessions.filter(
        (session) => session.sessionId !== active.session.sessionId,
      ),
    ];
    if (wasActive || !this._active) this._active = active;
    this.setState("ready");
  }

  async ensureActiveThread(): Promise<IActiveSessionThread> {
    await this.initialize();
    return this._active ?? this.startNewSession();
  }

  async startNewSession(
    title = "New Chat",
  ): Promise<IActiveSessionThread> {
    const active = await this.createSession(title);
    this.activateSession(active);
    return active;
  }

  private async createSession(
    title: string,
    model: ModelRef | undefined = undefined,
  ): Promise<IActiveSessionThread> {
    this.setState("creating");
    try {
      const created = await this.api.session.create({
        commandId: commandId("session"),
        title,
      });
      const thread = await this.api.session.createThread({
        commandId: commandId("thread"),
        sessionId: created.session.sessionId,
        expectedSequence: created.session.sequence,
        title: "Main",
      });
      const session = model && !sameModel(thread.session.model, model)
        ? (await this.api.session.setModel({
          commandId: commandId("session-model"),
          sessionId: thread.session.sessionId,
          expectedSequence: thread.session.sequence,
          model,
        })).session
        : thread.session;
      return { session, threadId: thread.threadId };
    } catch (error) {
      this.setError(error);
      throw error;
    }
  }

  async archiveSession(sessionId: SessionId): Promise<void> {
    await this.initialize();
    const session = this._sessions.find(
      (candidate) =>
        candidate.sessionId === sessionId &&
        candidate.status === "active",
    );
    if (!session) {
      throw new Error(`Active Session is not available: ${sessionId}`);
    }
    this.setState("archiving");
    try {
      const result = await this.api.session.archive({
        commandId: commandId("archive-session"),
        sessionId,
        expectedSequence: session.sequence,
      });
      this._sessions = this._sessions.map((candidate) =>
        candidate.sessionId === sessionId ? result.session : candidate
      );
      if (this._active?.session.sessionId === sessionId) {
        this._active = firstActiveThread(this._sessions);
      }
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
      const result = await this.api.session.setModel({
        commandId: commandId("session-model"),
        sessionId,
        expectedSequence: session.sequence,
        model,
      });
      this._sessions = this._sessions.map((candidate) => candidate.sessionId === sessionId ? result.session : candidate);
      if (this._active?.session.sessionId === sessionId) {
        this._active = { session: result.session, threadId: this._active.threadId };
      }
      this._error = undefined;
      this._onDidChange.fire();
    } catch (error) {
      this.setError(error);
      throw error;
    }
  }

  private async loadSessions(): Promise<void> {
    this.setState("loading");
    try {
      const result = await this.api.session.list();
      this._sessions = result.sessions;
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
    this._error = error instanceof Error
      ? error.message
      : "Unable to load sessions.";
    this._onDidChange.fire();
  }

  private addUntitledSession(title: string): IUntitledChatSession {
    const session: IUntitledChatSession = {
      untitledSessionId: createUuid(),
      title,
      model: undefined,
    };
    this._untitledSessions = [session, ...this._untitledSessions];
    return session;
  }

  private restoreAvailableSelection(): void {
    if (this.activeUntitledSession || this._active) return;
    this._activeUntitledSessionId = this._untitledSessions[0]?.untitledSessionId;
  }

  private activateSession(active: IActiveSessionThread): void {
    this._sessions = [
      active.session,
      ...this._sessions.filter(
        (session) => session.sessionId !== active.session.sessionId,
      ),
    ];
    this._active = active;
    this._activeUntitledSessionId = undefined;
    this.setState("ready");
  }
}

function firstActiveThread(
  sessions: readonly Session[],
): IActiveSessionThread | undefined {
  for (const session of sessions) {
    if (session.status !== "active") continue;
    const thread = session.threads.find(
      (candidate) => candidate.status === "active",
    );
    if (thread) return { session, threadId: thread.threadId };
  }
  return undefined;
}

function commandId(kind: string): string {
  return `desktop-${kind}-${createUuid()}`;
}

function sameModel(
  left: ModelRef | null | undefined,
  right: ModelRef | null | undefined,
): boolean {
  return left?.provider === right?.provider && left?.model === right?.model;
}
