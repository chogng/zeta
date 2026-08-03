import type { Event } from "../../../../base/common/event.js";
import { createServiceIdentifier } from "../../../../platform/instantiation/common/instantiation.js";

export type SessionId = string;
export type ThreadId = string;

export interface ModelRef {
  readonly provider: string;
  readonly model: string;
}

export type ThreadOrigin = { readonly type: "root" } | { readonly type: "fork"; readonly parentThreadId: ThreadId; readonly parentSequence: number };
export type SessionThreadStatus = "creating" | "active" | "archived";

export interface SessionThread {
  readonly threadId: ThreadId;
  readonly origin: ThreadOrigin;
  readonly status: SessionThreadStatus;
}

export type SessionStatus = "active" | "completed" | "archived";

export interface Session {
  readonly sessionId: SessionId;
  readonly title: string;
  readonly status: SessionStatus;
  readonly model?: ModelRef | null;
  readonly sequence: number;
  readonly threads: readonly SessionThread[];
}

/** Selected Session and Thread shared by session navigation and features. */
export interface IActiveSessionThread {
  readonly session: Session;
  readonly threadId: ThreadId;
}

/** An untitled Chat session that has not yet acquired a durable Session identity. */
export interface IUntitledChatSession {
  readonly untitledSessionId: string;
  readonly title: string;
  readonly model: ModelRef | undefined;
}

export type WorkbenchSessionState = "loading" | "ready" | "creating" | "stopping" | "archiving" | "error";

/** Owns durable Session selection and window-local untitled Chat sessions. */
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
  stopSession(sessionId: SessionId): Promise<void>;
  archiveSession(sessionId: SessionId): Promise<void>;
  setModel(sessionId: SessionId, model: ModelRef): Promise<void>;
}

export const IWorkbenchSessionService = createServiceIdentifier<IWorkbenchSessionService>("workbenchSessionService");
