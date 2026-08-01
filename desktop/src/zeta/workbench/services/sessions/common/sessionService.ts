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

/**
 * An unsaved Chat tab that has not yet acquired a durable Session identity.
 *
 * Drafts are window-local presentation state. They retain user choices until
 * the first send materializes the corresponding Session and root Thread.
 */
export interface IChatDraft {
  readonly draftId: string;
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
 * Owns durable Session selection and local Chat drafts for one workbench window.
 *
 * Feature views consume the durable selection instead of choosing an
 * arbitrary Thread independently. A draft becomes durable only when a Chat
 * pane asks to materialize it for its first send.
 */
export interface IWorkbenchSessionService {
  readonly onDidChange: Event<void>;
  readonly sessions: readonly Session[];
  readonly active: IActiveSessionThread | undefined;
  readonly drafts: readonly IChatDraft[];
  readonly activeDraft: IChatDraft | undefined;
  readonly state: WorkbenchSessionState;
  readonly error: string | undefined;

  initialize(): Promise<void>;
  selectThread(sessionId: SessionId, threadId: ThreadId): void;
  createDraft(title?: string): IChatDraft;
  selectDraft(draftId: string): void;
  discardDraft(draftId: string): void;
  setDraftModel(draftId: string, model: ModelRef): void;
  materializeDraft(draftId: string): Promise<IActiveSessionThread>;
  promoteDraft(draftId: string, active: IActiveSessionThread): void;
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
  private _drafts: readonly IChatDraft[] = [];
  private _activeDraftId: string | undefined;
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

  get drafts(): readonly IChatDraft[] {
    return this._drafts;
  }

  get activeDraft(): IChatDraft | undefined {
    return this._drafts.find(
      (draft) => draft.draftId === this._activeDraftId,
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
    if (this._active?.session.sessionId === sessionId && this._active.threadId === threadId && this._activeDraftId === undefined) return;
    this._active = { session, threadId };
    this._activeDraftId = undefined;
    this._error = undefined;
    this._onDidChange.fire();
  }

  createDraft(title = "New Chat"): IChatDraft {
    const draft: IChatDraft = {
      draftId: createUuid(),
      title,
      model: undefined,
    };
    this._drafts = [draft, ...this._drafts];
    this._activeDraftId = draft.draftId;
    this._error = undefined;
    this._onDidChange.fire();
    return draft;
  }

  selectDraft(draftId: string): void {
    if (!this._drafts.some((draft) => draft.draftId === draftId)) {
      throw new Error(`Chat Draft is not available: ${draftId}`);
    }
    if (this._activeDraftId === draftId) return;
    this._activeDraftId = draftId;
    this._error = undefined;
    this._onDidChange.fire();
  }

  discardDraft(draftId: string): void {
    const drafts = this._drafts.filter((draft) => draft.draftId !== draftId);
    if (drafts.length === this._drafts.length) return;
    this._drafts = drafts;
    if (this._activeDraftId === draftId) this._activeDraftId = undefined;
    this._onDidChange.fire();
  }

  setDraftModel(draftId: string, model: ModelRef): void {
    const current = this._drafts.find((draft) => draft.draftId === draftId);
    if (!current) throw new Error(`Chat Draft is not available: ${draftId}`);
    if (sameModel(current.model, model)) return;
    this._drafts = this._drafts.map((draft) =>
      draft.draftId === draftId ? { ...draft, model } : draft,
    );
    this._onDidChange.fire();
  }

  /** Creates server state for a draft without changing the visible tab yet. */
  async materializeDraft(draftId: string): Promise<IActiveSessionThread> {
    const draft = this._drafts.find((candidate) => candidate.draftId === draftId);
    if (!draft) throw new Error(`Chat Draft is not available: ${draftId}`);
    return this.createSession(draft.title, draft.model);
  }

  /** Replaces a local draft with its already-created durable Session. */
  promoteDraft(draftId: string, active: IActiveSessionThread): void {
    const wasActive = this._activeDraftId === draftId;
    this._drafts = this._drafts.filter((draft) => draft.draftId !== draftId);
    if (wasActive) this._activeDraftId = undefined;
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

  private activateSession(active: IActiveSessionThread): void {
    this._sessions = [
      active.session,
      ...this._sessions.filter(
        (session) => session.sessionId !== active.session.sessionId,
      ),
    ];
    this._active = active;
    this._activeDraftId = undefined;
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
