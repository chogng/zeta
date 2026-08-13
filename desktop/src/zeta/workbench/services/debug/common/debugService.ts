import { type Event } from "../../../../base/common/event.js";
import { type IDisposable } from "../../../../base/common/lifecycle.js";
import { type URI } from "../../../../base/common/uri.js";
import { createServiceIdentifier } from "../../../../platform/instantiation/common/instantiation.js";

export interface IDebugConfiguration {
  readonly id: string;
  readonly name: string;
  readonly type: string;
  readonly request: "launch" | "attach";
  readonly adapter: { readonly program: string; readonly arguments: readonly string[] };
  readonly arguments: Readonly<Record<string, unknown>>;
}

export interface IDebugBreakpoint {
  readonly id: string;
  readonly resource: URI;
  readonly lineNumber: number;
  readonly enabled: boolean;
  readonly verified: boolean;
  readonly message?: string;
}

export interface IDebugStackFrame {
  readonly id: number;
  readonly name: string;
  readonly source?: { readonly name?: string; readonly path?: string; readonly sourceReference?: number };
  readonly lineNumber: number;
  readonly columnNumber: number;
}

export interface IDebugScope {
  readonly name: string;
  readonly variablesReference: number;
  readonly expensive: boolean;
}

export interface IDebugVariable {
  readonly name: string;
  readonly value: string;
  readonly type?: string;
  readonly variablesReference: number;
}

export type DebugSessionState = "starting" | "running" | "stopped" | "terminated" | "error";

export interface IDebugSession extends IDisposable {
  readonly configuration: IDebugConfiguration;
  readonly state: DebugSessionState;
  readonly reason?: string;
  readonly threadId?: number;
  readonly onDidChangeState: Event<DebugSessionState>;
  readonly onDidOutput: Event<string>;
  continue(): Promise<void>;
  pause(): Promise<void>;
  stepOver(): Promise<void>;
  stepInto(): Promise<void>;
  stepOut(): Promise<void>;
  stackTrace(): Promise<readonly IDebugStackFrame[]>;
  scopes(frameId: number): Promise<readonly IDebugScope[]>;
  variables(reference: number): Promise<readonly IDebugVariable[]>;
  disconnect(): Promise<void>;
}

/** Code Workbench owner for launch configurations, breakpoints, and DAP session semantics. */
export interface IDebugService extends IDisposable {
  readonly configurations: readonly IDebugConfiguration[];
  readonly breakpoints: readonly IDebugBreakpoint[];
  readonly session: IDebugSession | undefined;
  readonly onDidChangeConfigurations: Event<readonly IDebugConfiguration[]>;
  readonly onDidChangeBreakpoints: Event<readonly IDebugBreakpoint[]>;
  readonly onDidChangeSession: Event<IDebugSession | undefined>;
  refresh(): Promise<readonly IDebugConfiguration[]>;
  start(configuration: IDebugConfiguration): Promise<IDebugSession>;
  stop(): Promise<void>;
  toggleBreakpoint(resource: URI, lineNumber: number): void;
  removeBreakpoint(id: string): void;
}

export const IDebugService = createServiceIdentifier<IDebugService>("debugService");
