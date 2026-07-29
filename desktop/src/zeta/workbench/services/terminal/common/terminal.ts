import type { Event } from "../../../../base/common/event.js";
import type { IDisposable } from "../../../../base/common/lifecycle.js";
import { createServiceIdentifier } from "../../../../platform/instantiation/common/instantiation.js";

/** Character-cell dimensions used by Workbench terminal callers. */
export interface ITerminalDimensions {
  readonly rows: number;
  readonly cols: number;
}

/** Lifecycle state of one Workbench terminal instance. */
export type TerminalInstanceState = "running" | "exited" | "disconnected" | "error";

/** One available shell profile exposed to Workbench callers. */
export interface ITerminalProfile {
  readonly profileId: string;
  readonly title: string;
  readonly isDefault: boolean;
}

/** Explicit profile selection for a new terminal. */
export type ITerminalProfileSelection =
  | { readonly type: "default" }
  | { readonly type: "profile"; readonly profileId: string };

/** Complete caller-facing input for creating one terminal. */
export interface ITerminalCreateOptions {
  readonly dimensions: ITerminalDimensions;
  readonly profile: ITerminalProfileSelection;
}

/** One interactive terminal independently of its transport representation. */
export interface ITerminalInstance extends IDisposable {
  readonly id: string;
  readonly title: string;
  readonly profile: ITerminalProfile;
  readonly state: TerminalInstanceState;
  readonly exitCode: number | undefined;
  readonly onDidWriteData: Event<Uint8Array>;
  readonly onDidExit: Event<number | undefined>;
  readonly onDidChangeState: Event<TerminalInstanceState>;

  write(data: string): void;
  resize(dimensions: ITerminalDimensions): void;
  close(): Promise<void>;
}

/** Workbench service contract for terminal instances and active-instance selection. */
export interface ITerminalService extends IDisposable {
  readonly instances: readonly ITerminalInstance[];
  readonly activeInstance: ITerminalInstance | undefined;
  readonly onDidCreateInstance: Event<ITerminalInstance>;
  readonly onDidDisposeInstance: Event<ITerminalInstance>;
  readonly onDidChangeActiveInstance: Event<ITerminalInstance | undefined>;

  getProfiles(): Promise<readonly ITerminalProfile[]>;
  createTerminal(options: ITerminalCreateOptions): Promise<ITerminalInstance>;
  relaunchTerminal(instance: ITerminalInstance, dimensions: ITerminalDimensions): Promise<void>;
  setActiveInstance(instance: ITerminalInstance | undefined): void;
  closeTerminal(instance: ITerminalInstance): Promise<void>;
}

export const ITerminalService = createServiceIdentifier<ITerminalService>("terminalService");
