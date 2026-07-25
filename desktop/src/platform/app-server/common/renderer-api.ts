import type {
  ServerNotification,
  ThreadReadParams,
  ThreadReadResult,
  ThreadStartParams,
  ThreadStartResult,
  TurnInterruptParams,
  TurnStartParams,
  TurnStartResult,
} from "../../../../generated/app-server/v1/types.js";

/**
 * The narrowly scoped, typed capability bridge available to the renderer.
 *
 * Electron preload implementations expose this contract without leaking IPC or
 * other Electron primitives to workbench code.
 */
export interface ZetaRendererApi {
  thread: {
    start(params: ThreadStartParams): Promise<ThreadStartResult>;
    read(params: ThreadReadParams): Promise<ThreadReadResult>;
  };
  turn: {
    start(params: TurnStartParams): Promise<TurnStartResult>;
    interrupt(params: TurnInterruptParams): Promise<void>;
  };
  events: {
    subscribe(listener: (event: ServerNotification) => void): () => void;
  };
}
