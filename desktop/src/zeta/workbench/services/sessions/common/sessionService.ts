import type { ModelRef, Session, SessionId, ThreadId } from "../../../../../../generated/app-server/types.js";
import { Emitter, type Event } from "../../../../base/common/event.js";
import { DisposableOwner } from "../../../../base/common/lifecycle.js";
import { createUuid } from "../../../../base/common/uuid.js";
import type { ZetaRendererApi } from "../../../../platform/app-server/common/renderer-api.js";
import { createServiceIdentifier } from "../../../../platform/instantiation/common/instantiation.js";

/** Selected Session and Thread shared by session navigation and features. */
export interface IActiveSessionThread {
  readonly session: Session;
  readonly threadId: ThreadId;
}

/** Current initialization or mutation state of the session projection. */
export type WorkbenchSessionState =
  | "loading"
  | "ready"
  | "creating"
  | "archiving"
  | "error";

/**
 * Owns the active Session and Thread selection for one workbench window.
 *
 * Feature views consume this selection instead of choosing an arbitrary
 * Thread independently. Durable transcript projection remains feature-owned.
 */
export interface IWorkbenchSessionService {
  readonly onDidChange: Event<void>;
  readonly sessions: readonly Session[];
  readonly active: IActiveSessionThread | undefined;
  readonly state: WorkbenchSessionState;
  readonly error: string | undefined;

  initialize(): Promise<void>;
  selectThread(sessionId: SessionId, threadId: ThreadId): void;
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
  private readonly api: ZetaRendererApi;
  private readonly _onDidChange = this.own(new Emitter<void>());
  private _sessions: readonly Session[] = [];
  private _active: IActiveSessionThread | undefined;
  private _state: WorkbenchSessionState = "loading";
  private _error: string | undefined;
  private initializePromise: Promise<void> | undefined;

  readonly onDidChange = this._onDidChange.event;

  constructor(api: ZetaRendererApi) {
    super();
    this.api = api;
  }

  get sessions(): readonly Session[] {
    return this._sessions;
  }

  get active(): IActiveSessionThread | undefined {
    return this._active;
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
    if (
      this._active?.session.sessionId === sessionId &&
      this._active.threadId === threadId
    ) return;
    this._active = { session, threadId };
    this._error = undefined;
    this._onDidChange.fire();
  }

  async ensureActiveThread(): Promise<IActiveSessionThread> {
    await this.initialize();
    return this._active ?? this.startNewSession();
  }

  async startNewSession(
    title = "New Chat",
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
      this._sessions = [
        thread.session,
        ...this._sessions.filter(
          (session) =>
            session.sessionId !== thread.session.sessionId,
        ),
      ];
      this._active = {
        session: thread.session,
        threadId: thread.threadId,
      };
      this.setState("ready");
      return this._active;
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
    this._state = "error";
    this._error = error instanceof Error
      ? error.message
      : "Unable to load sessions.";
    this._onDidChange.fire();
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
