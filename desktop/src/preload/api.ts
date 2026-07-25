import type {
  ServerNotification,
  ThreadReadParams,
  ThreadReadResult,
  ThreadStartParams,
  ThreadStartResult,
  TurnInterruptParams,
  TurnStartParams,
  TurnStartResult,
} from "../../generated/app-server/v1/types.js";

export interface ZetaPreloadApi {
  thread: {
    start(params: ThreadStartParams): Promise<ThreadStartResult>;
    read(params: ThreadReadParams): Promise<ThreadReadResult>;
  };
  turn: { start(params: TurnStartParams): Promise<TurnStartResult>; interrupt(params: TurnInterruptParams): Promise<void>; };
  events: { subscribe(listener: (event: ServerNotification) => void): () => void; };
}
