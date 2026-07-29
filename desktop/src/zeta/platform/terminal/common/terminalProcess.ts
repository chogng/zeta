import type { IDisposable } from "../../../base/common/lifecycle.js";

export type TerminalProcessConnectionState = "stopped" | "starting" | "initializing" | "ready" | "stopping" | "crashed" | "restarting";

export interface ITerminalProcessProfile {
  readonly profileId: string;
  readonly title: string;
  readonly isDefault: boolean;
}

export type TerminalProcessProfileSelection =
  | { readonly type: "default" }
  | { readonly type: "profile"; readonly profileId: string };

export interface ITerminalProcessCreateOptions {
  readonly rows: number;
  readonly cols: number;
  readonly profile: TerminalProcessProfileSelection;
}

export interface ITerminalProcessCreation {
  readonly terminalId: string;
  readonly profile: ITerminalProcessProfile;
}

export interface ITerminalProcessWriteOptions {
  readonly terminalId: string;
  readonly data: string;
}

export interface ITerminalProcessResizeOptions {
  readonly terminalId: string;
  readonly rows: number;
  readonly cols: number;
}

export interface ITerminalProcessReadOptions {
  readonly terminalId: string;
  readonly afterSequence: number;
  readonly maxChunks: number;
}

export interface ITerminalProcessOutputChunk {
  readonly sequence: number;
  readonly dataBase64: string;
}

export interface ITerminalProcessReadResult {
  readonly terminalId: string;
  readonly chunks: readonly ITerminalProcessOutputChunk[];
  readonly nextSequence: number;
  readonly outputGap: boolean;
  readonly exited: boolean;
  readonly exitCode: number | null;
}

export interface ITerminalProcessCloseOptions {
  readonly terminalId: string;
}

/** Platform contract for creating and communicating with terminal processes. */
export interface ITerminalProcessService {
  listProfiles(): Promise<readonly ITerminalProcessProfile[]>;
  create(options: ITerminalProcessCreateOptions): Promise<ITerminalProcessCreation>;
  write(options: ITerminalProcessWriteOptions): Promise<void>;
  resize(options: ITerminalProcessResizeOptions): Promise<void>;
  read(options: ITerminalProcessReadOptions): Promise<ITerminalProcessReadResult>;
  close(options: ITerminalProcessCloseOptions): Promise<void>;
  getConnectionState(): Promise<TerminalProcessConnectionState>;
  onConnectionState(listener: (state: TerminalProcessConnectionState) => void): IDisposable;
}
