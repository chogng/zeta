import type {
  Session,
  SessionId,
  ThreadId,
} from "../../../../../../generated/app-server/types.js";
import { Emitter, type Event } from "../../../../base/common/event.js";
import { DisposableOwner } from "../../../../base/common/lifecycle.js";
import { createUuid } from "../../../../base/common/uuid.js";
import type {
  ZetaRendererApi,
} from "../../../../platform/app-server/common/renderer-api.js";
import {
  createServiceIdentifier,
} from "../../../../platform/instantiation/common/instantiation.js";

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
  readonly #api: ZetaRendererApi;
  readonly #onDidChange = this.own(new Emitter<void>());
  #sessions: readonly Session[] = [];
  #active: IActiveSessionThread | undefined;
  #state: WorkbenchSessionState = "loading";
  #error: string | undefined;
  #initializePromise: Promise<void> | undefined;

  readonly onDidChange = this.#onDidChange.event;

  constructor(api: ZetaRendererApi) {
    super();
    this.#api = api;
  }

  get sessions(): readonly Session[] {
    return this.#sessions;
  }

  get active(): IActiveSessionThread | undefined {
    return this.#active;
  }

  get state(): WorkbenchSessionState {
    return this.#state;
  }

  get error(): string | undefined {
    return this.#error;
  }

  initialize(): Promise<void> {
    if (!this.#initializePromise || this.#state === "error") {
      this.#initializePromise = this.#loadSessions();
    }
    return this.#initializePromise;
  }

  selectThread(sessionId: SessionId, threadId: ThreadId): void {
    const session = this.#sessions.find(
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
      this.#active?.session.sessionId === sessionId &&
      this.#active.threadId === threadId
    ) return;
    this.#active = { session, threadId };
    this.#error = undefined;
    this.#onDidChange.fire();
  }

  async ensureActiveThread(): Promise<IActiveSessionThread> {
    await this.initialize();
    return this.#active ?? this.startNewSession();
  }

  async startNewSession(
    title = "New Chat",
  ): Promise<IActiveSessionThread> {
    this.#setState("creating");
    try {
      const created = await this.#api.session.create({
        commandId: commandId("session"),
        title,
      });
      const thread = await this.#api.session.createThread({
        commandId: commandId("thread"),
        sessionId: created.session.sessionId,
        expectedSequence: created.session.sequence,
        title: "Main",
      });
      this.#sessions = [
        thread.session,
        ...this.#sessions.filter(
          (session) =>
            session.sessionId !== thread.session.sessionId,
        ),
      ];
      this.#active = {
        session: thread.session,
        threadId: thread.threadId,
      };
      this.#setState("ready");
      return this.#active;
    } catch (error) {
      this.#setError(error);
      throw error;
    }
  }

  async #loadSessions(): Promise<void> {
    this.#setState("loading");
    try {
      const result = await this.#api.session.list();
      this.#sessions = result.sessions;
      this.#active = firstActiveThread(this.#sessions);
      this.#setState("ready");
    } catch (error) {
      this.#setError(error);
    }
  }

  #setState(state: WorkbenchSessionState): void {
    this.#state = state;
    this.#error = undefined;
    this.#onDidChange.fire();
  }

  #setError(error: unknown): void {
    this.#state = "error";
    this.#error = error instanceof Error
      ? error.message
      : "Unable to load sessions.";
    this.#onDidChange.fire();
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
